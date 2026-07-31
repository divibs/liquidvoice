# LiquidVoice - Agent Guide

Windows system-wide STT dictation. Hold/toggle a hotkey → liquid-glass overlay morphs in → mic audio → OpenAI `gpt-4o-transcribe` → text injected at cursor via Win32 `SendInput` (never clipboard). Lives in the system tray hidden-icons area.

## Stack
- **Shell:** Tauri 2 (Rust): tray, global hotkey, transparent always-on-top windows. WebView2 ships with Win10/11.
- **Frontend:** SvelteKit 2 + Svelte 5 (runes) + Vite, static adapter (`fallback: index.html`).
- **Audio:** `cpal` (device default config, i16/f32/u16) → resample to 16 kHz mono → `hound` WAV.
- **HTTP:** shared `reqwest` client, multipart POST to OpenAI transcriptions (15 s timeout).
- **Injection:** `windows` crate `SendInput` + `KEYEVENTF_UNICODE`, chunked 64 UTF-16 units.
- **Font:** `@fontsource-variable/space-grotesk` (imported in Svelte, NOT a CDN link).

## Rust modules (`src-tauri/src/`)
- `lib.rs`: Tauri setup, tray menu, hotkey handler, `Idle | Listening | Processing` phase machine, overlay show/hide, hotkey rebind on save.
- `main.rs`: entry, calls `liquidvoice_lib::run()`.
- `audio.rs`: `AudioRecorder` (cpal stream, `!Send` wrapper), `pcm_to_wav`, linear resample, speech/hallucination helpers.
- `transcribe.rs`: async OpenAI call.
- `inject.rs`: `type_text` (Win32, `cfg(windows)`; no-op stub elsewhere).
- `config.rs`: `AppConfig` load/save/sanitize at `%APPDATA%/liquidvoice/config.json`.

## Frontend (`src/`)
- `routes/+page.svelte`: overlay page; listens to Tauri events `state`, `mic-level`, `error-msg`; drives `Capsule` target 0/1. Dev-preview mode when `__TAURI_INTERNALS__` absent.
- `components/Capsule.svelte`: the overlay pill: pure-rAF timeline (`raw` 0..1), frosted morph (dot→stretch→capsule), then exit as orb with spinner → check → shrink-to-none.
- `routes/settings/+page.svelte`: settings window (API key, model, hotkey, trigger mode, language, prompt, wallpaper, launch at login, max recording).

## State machine
`IDLE` (tray only) ─hotkey─▶ `LISTENING` (overlay pill) ─release/toggle─▶ `PROCESSING` (pill morphs to orb + spinner) ─▶ inject ─▶ success check on orb ─▶ shrink away ─▶ `IDLE`. Silent/skip uses orb without check. Errors shake + 5 s caption, then return to idle. Hotkeys are ignored while `PROCESSING`.

## Run / build
- `npm run tauri dev`: full app, hot reload. Logs (Rust errors, audio, API) print to this terminal.
- `npm run dev`: frontend only at `localhost:1420`; auto-plays morph with fake mic (no Tauri backend).
- `npm run tauri build`: NSIS installer at `src-tauri/target/release/bundle/nsis/`.
- `cd src-tauri && cargo test --lib`: unit tests for audio/config helpers.

## Conventions / gotchas
- **Svelte 5 reactivity:** any value derived from a `$state` MUST be `$derived`, never a top-level `const` (a plain `const` captures the initial value once and freezes; this broke the morph: geometry stuck at the dot). Inside `$effect`/rAF, read states via `untrack(() => x)`.
- **Mic config:** never request 16 kHz directly; Windows mics reject it ("config not supported"). Capture device default (often 48 kHz stereo), mix to mono, resample on `stop()`.
- **Transparent window:** `shadow: false`, `decorations: false`, `focusable: false`, `skipTaskbar: true`; call `set_ignore_cursor_events(true)` so it's click-through; `hide()` the window when done (CSS opacity alone leaves a dead rectangle).
- **Tray icon:** define ONCE in Rust (`TrayIconBuilder`), not also in `tauri.conf.json` `trayIcon` (else two icons).
- **Hotkey:** `tauri-plugin-global-shortcut`; event field is `event.state` (`ShortcutState::Pressed/Released`), not enum variants on the event. Changing hotkey in settings rebinds at save time.
- **`State` borrows:** bind `let state = app.state();` then `drop(state)` before any `.await`/spawn or nested lock to avoid "does not live long enough".
- **Smart App Control** on Win11 blocks `cargo` (os error 4551): disable it or use a release installer artifact.
- **Style:** do not use em dashes (-) in comments, docs, or UI copy.

## Config keys
`api_key`, `model`, `hotkey` (default `Ctrl+Space`), `trigger_mode` (`push-to-talk`|`toggle`), `language`, `prompt`, `theme`, `max_recording_sec` (clamped 5-300; auto-stops listening).
