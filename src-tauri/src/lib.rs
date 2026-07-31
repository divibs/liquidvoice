mod audio;
mod config;
mod inject;
mod transcribe;

use audio::AudioRecorder;
use config::{AppConfig, TriggerMode};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Listening,
    Processing,
}

struct AppState {
    recorder: Mutex<AudioRecorder>,
    config: Mutex<AppConfig>,
    phase: Mutex<Phase>,
    /// Bumps each time listening starts; max-duration tasks check this so a
    /// stale timer from a prior take cannot stop a newer one.
    listen_gen: Mutex<u64>,
    /// Currently registered hotkey string (for unregister-before-rebind).
    bound_hotkey: Mutex<String>,
}

fn lock<'a, T>(m: &'a Mutex<T>, label: &str) -> Result<std::sync::MutexGuard<'a, T>, String> {
    m.lock().map_err(|_| format!("{label} lock poisoned"))
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<AppConfig, String> {
    Ok(lock(&state.config, "config")?.clone())
}

#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    state: State<AppState>,
    config: AppConfig,
) -> Result<(), String> {
    let config = config.sanitize();
    let new_hotkey = config.hotkey.clone();

    {
        let mut bound = lock(&state.bound_hotkey, "hotkey")?;
        if *bound != new_hotkey {
            rebind_hotkey(&app, &bound, &new_hotkey)?;
            *bound = new_hotkey;
        }
    }

    config::save(&config)?;
    let frost = config.frost_strength;
    *lock(&state.config, "config")? = config;
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("frost-strength", frost);
    }
    Ok(())
}

fn rebind_hotkey(app: &tauri::AppHandle, old: &str, new: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let new_sc = new
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid hotkey '{new}': {e}"))?;

    if !old.is_empty() {
        if let Ok(old_sc) = old.parse::<Shortcut>() {
            let _ = gs.unregister(old_sc);
        }
    }

    if let Err(e) = gs.on_shortcut(new_sc, hotkey_handler(app.clone())) {
        // Best-effort restore of the previous binding.
        if !old.is_empty() {
            if let Ok(old_sc) = old.parse::<Shortcut>() {
                let _ = gs.on_shortcut(old_sc, hotkey_handler(app.clone()));
            }
        }
        return Err(format!("Failed to register hotkey '{new}': {e}"));
    }
    Ok(())
}

fn hotkey_handler(
    app_handle: tauri::AppHandle,
) -> impl Fn(&tauri::AppHandle, &Shortcut, tauri_plugin_global_shortcut::ShortcutEvent) + Send + Sync + 'static
{
    move |_app, _shortcut, event| {
        let handle = app_handle.clone();
        match event.state {
            ShortcutState::Pressed => on_hotkey_press(&handle),
            ShortcutState::Released => on_hotkey_release(&handle),
        }
    }
}

fn on_hotkey_press(handle: &tauri::AppHandle) {
    let state: State<AppState> = handle.state();
    let mode = match lock(&state.config, "config") {
        Ok(c) => c.trigger_mode.clone(),
        Err(_) => return,
    };

    let mut phase = match lock(&state.phase, "phase") {
        Ok(p) => p,
        Err(_) => return,
    };

    match mode {
        TriggerMode::Toggle => match *phase {
            Phase::Listening => {
                *phase = Phase::Processing;
                drop(phase);
                stop_and_transcribe(handle);
            }
            Phase::Idle => {
                // Claim Listening before drop so a rapid second press cannot double-start.
                *phase = Phase::Listening;
                drop(phase);
                if !start_listening(handle) {
                    set_phase_idle(handle);
                }
            }
            Phase::Processing => {}
        },
        TriggerMode::PushToTalk => {
            if *phase == Phase::Idle {
                *phase = Phase::Listening;
                drop(phase);
                if !start_listening(handle) {
                    set_phase_idle(handle);
                }
            }
        }
    }
}

fn on_hotkey_release(handle: &tauri::AppHandle) {
    let state: State<AppState> = handle.state();
    let mode = match lock(&state.config, "config") {
        Ok(c) => c.trigger_mode.clone(),
        Err(_) => return,
    };
    if mode != TriggerMode::PushToTalk {
        return;
    }

    let mut phase = match lock(&state.phase, "phase") {
        Ok(p) => p,
        Err(_) => return,
    };
    if *phase == Phase::Listening {
        *phase = Phase::Processing;
        drop(phase);
        stop_and_transcribe(handle);
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("LiquidVoice")
                .build(),
        )
        .manage(AppState {
            recorder: Mutex::new(AudioRecorder::new()),
            config: Mutex::new(config::load()),
            phase: Mutex::new(Phase::Idle),
            listen_gen: Mutex::new(0),
            bound_hotkey: Mutex::new(String::new()),
        })
        .invoke_handler(tauri::generate_handler![get_config, save_config])
        .setup(|app| {
            build_tray(app)?;
            position_overlay(app);

            let state: State<AppState> = app.state();
            let hotkey = state
                .config
                .lock()
                .expect("config")
                .hotkey
                .clone();
            drop(state);

            let shortcut = hotkey
                .parse::<Shortcut>()
                .unwrap_or_else(|_| "Ctrl+Space".parse().expect("default hotkey"));
            app.global_shortcut()
                .on_shortcut(shortcut, hotkey_handler(app.handle().clone()))?;
            *app.state::<AppState>()
                .bound_hotkey
                .lock()
                .expect("bound_hotkey") = hotkey;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LiquidVoice");
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::WindowNotFound)?;

    let _tray = tauri::tray::TrayIconBuilder::new()
        .icon(icon)
        .tooltip("LiquidVoice")
        .menu(
            &tauri::menu::MenuBuilder::new(app)
                .item(&tauri::menu::MenuItem::with_id(
                    app,
                    "settings",
                    "Settings",
                    true,
                    None::<&str>,
                )?)
                .separator()
                .item(&tauri::menu::MenuItem::with_id(
                    app,
                    "quit",
                    "Quit",
                    true,
                    None::<&str>,
                )?)
                .build()?,
        )
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => {
                if let Some(win) = app.get_webview_window("settings") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn position_overlay(app: &tauri::App) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        place_overlay(&overlay);
    }
}

const OVERLAY_SIZE: (f64, f64) = (460.0, 90.0);

fn place_overlay(overlay: &tauri::WebviewWindow) {
    let (lw, lh) = OVERLAY_SIZE;
    let Ok(Some(monitor)) = overlay.primary_monitor() else {
        return;
    };
    let origin = monitor.position();
    let screen_w = monitor.size().width as i32;
    let scale = monitor.scale_factor();
    let win_w = (lw * scale) as i32;

    let _ = overlay.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: lw,
        height: lh,
    }));
    let _ = overlay.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
        x: origin.x + (screen_w - win_w) / 2,
        y: origin.y + (4.0 * scale) as i32,
    }));
}

fn emit_overlay(handle: &tauri::AppHandle, state: &str) {
    if let Some(overlay) = handle.get_webview_window("overlay") {
        let _ = overlay.set_ignore_cursor_events(true);
        place_overlay(&overlay);
        clear_overlay_blur(&overlay);
        let _ = overlay.show();
        let frost = handle
            .state::<AppState>()
            .config
            .lock()
            .map(|c| c.frost_strength)
            .unwrap_or(72);
        let _ = overlay.emit("frost-strength", frost);
        let _ = overlay.emit("state", state);
    }
}

fn emit_error(handle: &tauri::AppHandle, msg: impl AsRef<str>) {
    if let Some(overlay) = handle.get_webview_window("overlay") {
        let _ = overlay.set_ignore_cursor_events(true);
        place_overlay(&overlay);
        clear_overlay_blur(&overlay);
        let _ = overlay.show();
        let _ = overlay.emit("state", "error");
        let _ = overlay.emit("error-msg", msg.as_ref());
    }
    // Return to Idle so the next hotkey press works after the UI collapses.
    if let Ok(mut phase) = lock(&handle.state::<AppState>().phase, "phase") {
        *phase = Phase::Idle;
    }
}

fn hide_overlay_later(handle: tauri::AppHandle, ms: u64) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        if let Some(overlay) = handle.get_webview_window("overlay") {
            clear_overlay_blur(&overlay);
            let _ = overlay.hide();
        }
    });
}

fn clear_overlay_blur(overlay: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        let _ = window_vibrancy::clear_acrylic(overlay);
    }
    #[cfg(not(windows))]
    {
        let _ = overlay;
    }
}

fn start_listening(handle: &tauri::AppHandle) -> bool {
    let state: State<AppState> = handle.state();
    let max_sec = state
        .config
        .lock()
        .map(|c| c.max_recording_sec)
        .unwrap_or(60);

    let gen = {
        let mut g = match lock(&state.listen_gen, "listen_gen") {
            Ok(g) => g,
            Err(e) => {
                emit_error(handle, e);
                return false;
            }
        };
        *g = g.wrapping_add(1);
        *g
    };

    let mut recorder = match lock(&state.recorder, "recorder") {
        Ok(r) => r,
        Err(e) => {
            emit_error(handle, e);
            return false;
        }
    };

    let h = handle.clone();
    match recorder.start(move |level| {
        let _ = h.emit("mic-level", level);
    }) {
        Ok(()) => {
            drop(recorder);
            drop(state);
            emit_overlay(handle, "listening");

            // Auto-stop when max recording length is hit (generation-guarded).
            let h2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(max_sec as u64)).await;
                let state: State<AppState> = h2.state();
                let current = match lock(&state.listen_gen, "listen_gen") {
                    Ok(g) => *g,
                    Err(_) => return,
                };
                if current != gen {
                    return;
                }
                let mut phase = match lock(&state.phase, "phase") {
                    Ok(p) => p,
                    Err(_) => return,
                };
                if *phase == Phase::Listening {
                    *phase = Phase::Processing;
                    drop(phase);
                    drop(state);
                    stop_and_transcribe(&h2);
                }
            });
            true
        }
        Err(e) => {
            drop(recorder);
            drop(state);
            emit_error(handle, e);
            false
        }
    }
}

fn stop_and_transcribe(handle: &tauri::AppHandle) {
    let state: State<AppState> = handle.state();
    let max_sec = state
        .config
        .lock()
        .map(|c| c.max_recording_sec)
        .unwrap_or(60);
    let pcm = match lock(&state.recorder, "recorder") {
        Ok(mut r) => r.stop(Some(max_sec)),
        Err(_) => {
            if let Ok(mut p) = lock(&state.phase, "phase") {
                *p = Phase::Idle;
            }
            return;
        }
    };
    drop(state);

    if let Some(overlay) = handle.get_webview_window("overlay") {
        let _ = overlay.emit("state", "processing");
    }

    // Skip API when recording is too short or effectively silent.
    // Whisper/gpt-4o-transcribe often invents filler on quiet audio.
    if pcm.len() < 1600 || !audio::has_audible_speech(&pcm, 16000) {
        finish_idle(handle, true);
        return;
    }

    let cfg = match lock(&handle.state::<AppState>().config, "config") {
        Ok(c) => c.clone(),
        Err(e) => {
            emit_error(handle, e);
            return;
        }
    };

    if cfg.active_api_key().is_empty() {
        let msg = if config::is_qwen_model(&cfg.model) {
            "Set Qwen / DashScope API key in settings"
        } else {
            "Set OpenAI API key in settings"
        };
        emit_error(handle, msg);
        return;
    }

    let wav = match audio::pcm_to_wav(&pcm, 16000) {
        Ok(w) => w,
        Err(e) => {
            emit_error(handle, e);
            return;
        }
    };

    let h = handle.clone();
    let api_key = cfg.active_api_key().to_string();
    tauri::async_runtime::spawn(async move {
        let result =
            transcribe::transcribe(&api_key, &cfg.model, wav, &cfg.language, &cfg.prompt)
                .await;

        match result {
            Ok(text) => {
                let mut injected = false;
                if !text.is_empty() && !audio::is_likely_hallucination(&text) {
                    match inject::type_text(&text) {
                        Ok(()) => injected = true,
                        Err(e) => {
                            emit_error(&h, e);
                            hide_overlay_later(h.clone(), 5000);
                            return;
                        }
                    }
                }
                if let Some(overlay) = h.get_webview_window("overlay") {
                    let _ = overlay.emit("state", if injected { "done" } else { "skipped" });
                }
                // Enough time for orb morph + check (or skip) + shrink.
                hide_overlay_later(h.clone(), if injected { 1600 } else { 1000 });
                set_phase_idle(&h);
            }
            Err(e) => {
                emit_error(&h, e);
                hide_overlay_later(h, 5000);
            }
        }
    });
}

fn finish_idle(handle: &tauri::AppHandle, hide_now: bool) {
    if let Some(overlay) = handle.get_webview_window("overlay") {
        // Quiet exit: orb then shrink (no success check).
        let _ = overlay.emit("state", "skipped");
        if hide_now {
            hide_overlay_later(handle.clone(), 1000);
        }
    }
    set_phase_idle(handle);
}

fn set_phase_idle(handle: &tauri::AppHandle) {
    if let Ok(mut phase) = lock(&handle.state::<AppState>().phase, "phase") {
        *phase = Phase::Idle;
    }
}
