mod audio;
mod config;
mod inject;
mod secret;
mod transcribe;

use audio::AudioRecorder;
use config::{AppConfig, TriggerMode};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State, WindowEvent};
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
    /// Bumps each time listening starts; max-duration tasks and delayed overlay
    /// hides check this so a stale task from a prior take cannot act on a newer one.
    listen_gen: Mutex<u64>,
    /// Currently registered hotkey string (for unregister-before-rebind).
    bound_hotkey: Mutex<String>,
    /// Trigger mode that started the current take (None when idle). Press/release
    /// handlers use this so a settings change mid-take cannot stick or drop a take.
    take_mode: Mutex<Option<TriggerMode>>,
    /// Foreground window when the take started; injection is refused if focus moved.
    inject_target: Mutex<Option<isize>>,
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

    // Persist first: a failed rebind must not leave disk and memory diverged.
    config::save(&config)?;

    let mut bound = lock(&state.bound_hotkey, "hotkey")?;
    if *bound != new_hotkey {
        rebind_hotkey(&app, &bound, &new_hotkey)?;
        *bound = new_hotkey;
    }
    drop(bound);

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
    let take = match lock(&state.take_mode, "take_mode") {
        Ok(t) => *t,
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
        TriggerMode::PushToTalk => match *phase {
            Phase::Idle => {
                *phase = Phase::Listening;
                drop(phase);
                if !start_listening(handle) {
                    set_phase_idle(handle);
                }
            }
            // A take that started under Toggle still toggles off on press even
            // if the user switched the mode in settings mid-take.
            Phase::Listening if take == Some(TriggerMode::Toggle) => {
                *phase = Phase::Processing;
                drop(phase);
                stop_and_transcribe(handle);
            }
            _ => {}
        },
    }
}

fn on_hotkey_release(handle: &tauri::AppHandle) {
    let state: State<AppState> = handle.state();
    let take = match lock(&state.take_mode, "take_mode") {
        Ok(t) => *t,
        Err(_) => return,
    };
    // Only push-to-talk takes end on release. A take_mode of None is treated as
    // push-to-talk so an inconsistent state cannot leave the app stuck listening.
    if take == Some(TriggerMode::Toggle) {
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
            take_mode: Mutex::new(None),
            inject_target: Mutex::new(None),
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

            let shortcut = hotkey.parse::<Shortcut>().unwrap_or_else(|_| {
                eprintln!("LiquidVoice: invalid hotkey '{hotkey}' in config; using Ctrl+Space");
                "Ctrl+Space".parse().expect("default hotkey")
            });
            let binding = shortcut.to_string();
            match app
                .global_shortcut()
                .on_shortcut(shortcut, hotkey_handler(app.handle().clone()))
            {
                Ok(()) => {
                    *app.state::<AppState>()
                        .bound_hotkey
                        .lock()
                        .expect("bound_hotkey") = binding;
                }
                // A busy shortcut must not prevent the app from starting; the
                // user can pick another one in settings.
                Err(e) => eprintln!("LiquidVoice: could not register hotkey: {e}"),
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    // Tray apps keep the settings window around; closing hides it.
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
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
    // Prefer the monitor under the cursor; fall back to the primary monitor.
    let app = overlay.app_handle();
    let monitor = overlay
        .cursor_position()
        .ok()
        .and_then(|pos| app.monitor_from_point(pos.x, pos.y).ok().flatten())
        .or_else(|| overlay.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
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
    let gen = lock(&handle.state::<AppState>().listen_gen, "listen_gen")
        .map(|g| *g)
        .unwrap_or(0);
    if let Some(overlay) = handle.get_webview_window("overlay") {
        let _ = overlay.set_ignore_cursor_events(true);
        place_overlay(&overlay);
        clear_overlay_blur(&overlay);
        let _ = overlay.show();
        let _ = overlay.emit("state", "error");
        let _ = overlay.emit("error-msg", msg.as_ref());
    }
    hide_overlay_later(handle.clone(), 5000, gen);
    // Return to Idle so the next hotkey press works after the UI collapses.
    set_phase_idle(handle);
}

fn hide_overlay_later(handle: tauri::AppHandle, ms: u64, gen: u64) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        let state: State<AppState> = handle.state();
        let current = lock(&state.listen_gen, "listen_gen")
            .map(|g| *g)
            .unwrap_or(u64::MAX);
        // A newer take bumped the generation; leave its overlay alone.
        if current != gen {
            return;
        }
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
    let (max_sec, mode) = match lock(&state.config, "config") {
        Ok(c) => (c.max_recording_sec, c.trigger_mode.clone()),
        Err(e) => {
            emit_error(handle, e);
            return false;
        }
    };

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
    let on_error = move |msg: String| {
        let st: State<AppState> = h.state();
        let listening = lock(&st.phase, "phase")
            .map(|p| *p == Phase::Listening)
            .unwrap_or(false);
        if listening {
            emit_error(&h, &msg);
        }
    };
    let h = handle.clone();
    match recorder.start(
        move |level| {
            let _ = h.emit("mic-level", level);
        },
        on_error,
    ) {
        Ok(()) => {
            drop(recorder);
            if let Err(e) = lock(&state.take_mode, "take_mode").map(|mut t| *t = Some(mode)) {
                emit_error(handle, e);
                return false;
            }
            let target = inject::foreground_window();
            if let Err(e) = lock(&state.inject_target, "inject_target").map(|mut t| *t = target) {
                emit_error(handle, e);
                return false;
            }
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
    let gen = match lock(&state.listen_gen, "listen_gen") {
        Ok(g) => *g,
        Err(e) => {
            emit_error(handle, e);
            return;
        }
    };
    let target = lock(&state.inject_target, "inject_target")
        .ok()
        .and_then(|t| *t);
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

    // show() here so a take that somehow lost its overlay still gets feedback.
    emit_overlay(handle, "processing");

    // Skip the API for accidental taps; nothing usable was recorded.
    if pcm.len() < 1600 {
        finish_idle(handle, true, gen);
        return;
    }
    // Recorded but too quiet: tell the user instead of silently skipping.
    if !audio::has_audible_speech(&pcm, 16000) {
        emit_error(handle, "Nothing audible — speak up or check your mic");
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
    let duration_sec = (pcm.len() / 16_000) as u32;

    let h = handle.clone();
    let api_key = cfg.active_api_key().to_string();
    tauri::async_runtime::spawn(async move {
        let result = transcribe::transcribe(
            &api_key,
            &cfg.model,
            wav,
            &cfg.language,
            &cfg.prompt,
            duration_sec,
        )
        .await;

        match result {
            Ok(text) => {
                let mut injected = false;
                if !text.is_empty() && !audio::is_likely_hallucination(&text) {
                    match inject::type_text(&text, target) {
                        Ok(()) => injected = true,
                        Err(e) => {
                            emit_error(&h, e);
                            return;
                        }
                    }
                }
                if let Some(overlay) = h.get_webview_window("overlay") {
                    let _ = overlay.emit("state", if injected { "done" } else { "skipped" });
                }
                // Enough time for orb morph + check (or skip) + shrink.
                hide_overlay_later(h.clone(), if injected { 1600 } else { 1000 }, gen);
                set_phase_idle(&h);
            }
            Err(e) => {
                emit_error(&h, e);
            }
        }
    });
}

fn finish_idle(handle: &tauri::AppHandle, hide_now: bool, gen: u64) {
    if let Some(overlay) = handle.get_webview_window("overlay") {
        // Quiet exit: orb then shrink (no success check).
        let _ = overlay.emit("state", "skipped");
        if hide_now {
            hide_overlay_later(handle.clone(), 1000, gen);
        }
    }
    set_phase_idle(handle);
}

fn set_phase_idle(handle: &tauri::AppHandle) {
    let state: State<AppState> = handle.state();
    if let Ok(mut phase) = lock(&state.phase, "phase") {
        *phase = Phase::Idle;
    }
    if let Ok(mut take) = lock(&state.take_mode, "take_mode") {
        *take = None;
    }
    if let Ok(mut target) = lock(&state.inject_target, "inject_target") {
        *target = None;
    };
}
