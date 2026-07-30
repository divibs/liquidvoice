mod audio;
mod config;
mod inject;
mod transcribe;

use audio::AudioRecorder;
use config::AppConfig;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct AppState {
    recorder: Mutex<AudioRecorder>,
    config: Mutex<AppConfig>,
    listening: Mutex<bool>,
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    config::save(&config)?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            recorder: Mutex::new(AudioRecorder::new()),
            config: Mutex::new(config::load()),
            listening: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![get_config, save_config])
        .setup(|app| {
            let tray = tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
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
            let _ = tray;

            if let Some(overlay) = app.get_webview_window("overlay") {
                if let Ok(Some(monitor)) = overlay.primary_monitor() {
                    let screen_w = monitor.size().width as i32;
                    let screen_h = monitor.size().height as i32;
                    let scale = monitor.scale_factor();
                    let win_w = (320.0 * scale) as i32;
                    let win_h = (90.0 * scale) as i32;
                    let _ = overlay.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition {
                            x: (screen_w - win_w) / 2,
                            y: screen_h - win_h - (100.0 * scale) as i32,
                        },
                    ));
                }
            }

            let state: State<AppState> = app.state();
            let hotkey = state.config.lock().unwrap().hotkey.clone();
            drop(state);

            let app_handle = app.handle().clone();
            app.global_shortcut().on_shortcut(
                hotkey.parse::<tauri_plugin_global_shortcut::Shortcut>().unwrap(),
                move |_app, _shortcut, event| {
                    let handle = app_handle.clone();
                    match event.state {
                        ShortcutState::Pressed => {
                            let state: State<AppState> = handle.state();
                            let mode = state.config.lock().unwrap().trigger_mode.clone();

                            if mode == config::TriggerMode::Toggle {
                                let mut listening = state.listening.lock().unwrap();
                                if *listening {
                                    *listening = false;
                                    drop(listening);
                                    stop_and_transcribe(&handle);
                                } else {
                                    *listening = true;
                                    drop(listening);
                                    start_listening(&handle);
                                }
                            } else {
                                *state.listening.lock().unwrap() = true;
                                drop(state);
                                start_listening(&handle);
                            }
                        }
                        ShortcutState::Released => {
                            let state: State<AppState> = handle.state();
                            let mode = state.config.lock().unwrap().trigger_mode.clone();
                            let is_listening = *state.listening.lock().unwrap();
                            drop(state);

                            if mode == config::TriggerMode::PushToTalk && is_listening {
                                let state: State<AppState> = handle.state();
                                *state.listening.lock().unwrap() = false;
                                drop(state);
                                stop_and_transcribe(&handle);
                            }
                        }
                    }
                },
            )?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LiquidVoice");
}

fn start_listening(handle: &tauri::AppHandle) {
    let state: State<AppState> = handle.state();
    let mut recorder = state.recorder.lock().unwrap();

    let h = handle.clone();
    match recorder.start(move |level| {
        let _ = h.emit("mic-level", level);
    }) {
        Ok(()) => {
            drop(recorder);
            drop(state);
            if let Some(overlay) = handle.get_webview_window("overlay") {
                let _ = overlay.set_ignore_cursor_events(true);
                let _ = overlay.show();
                let _ = overlay.emit("state", "listening");
            }
        }
        Err(e) => {
            drop(recorder);
            drop(state);
            if let Some(overlay) = handle.get_webview_window("overlay") {
                let _ = overlay.set_ignore_cursor_events(true);
                let _ = overlay.show();
                let _ = overlay.emit("state", "error");
                let _ = overlay.emit("error-msg", e);
            }
        }
    }
}

fn stop_and_transcribe(handle: &tauri::AppHandle) {
    let state: State<AppState> = handle.state();
    let pcm = state.recorder.lock().unwrap().stop();
    drop(state);

    if let Some(overlay) = handle.get_webview_window("overlay") {
        let _ = overlay.emit("state", "processing");
    }

    if pcm.len() < 8000 {
        if let Some(overlay) = handle.get_webview_window("overlay") {
            let _ = overlay.emit("state", "done");
            let _ = overlay.hide();
        }
        return;
    }

    let state: State<AppState> = handle.state();
    let cfg = state.config.lock().unwrap().clone();
    drop(state);

    if cfg.api_key.is_empty() {
        if let Some(overlay) = handle.get_webview_window("overlay") {
            let _ = overlay.emit("state", "error");
            let _ = overlay.emit("error-msg", "Set API key in settings");
        }
        return;
    }

    let wav = match audio::pcm_to_wav(&pcm, 16000) {
        Ok(w) => w,
        Err(e) => {
            if let Some(overlay) = handle.get_webview_window("overlay") {
                let _ = overlay.emit("state", "error");
                let _ = overlay.emit("error-msg", e);
            }
            return;
        }
    };

    let h = handle.clone();
    tauri::async_runtime::spawn(async move {
        let result =
            transcribe::transcribe(&cfg.api_key, &cfg.model, wav, &cfg.language, &cfg.prompt)
                .await;

        match result {
            Ok(text) => {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    let _ = inject::type_text(&text);
                }
                if let Some(overlay) = h.get_webview_window("overlay") {
                    let _ = overlay.emit("state", "done");
                }
                let h2 = h.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                    if let Some(overlay) = h2.get_webview_window("overlay") {
                        let _ = overlay.hide();
                    }
                });
            }
            Err(e) => {
                if let Some(overlay) = h.get_webview_window("overlay") {
                    let _ = overlay.emit("state", "error");
                    let _ = overlay.emit("error-msg", e);
                }
                let h2 = h.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
                    if let Some(overlay) = h2.get_webview_window("overlay") {
                        let _ = overlay.hide();
                    }
                });
            }
        }
    });
}
