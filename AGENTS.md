# LiquidVoice — Agent Guide

Windows system-wide STT dictation. Hold/toggle a hotkey → liquid-glass overlay morphs in → mic audio → OpenAI `gpt-4o-transcribe` → text injected at cursor via Win32 `SendInput` (never clipboard). Lives in the system tray hidden-icons area.

## Stack
- **Shell:** Tauri 2 (Rust) — tray, global hotkey, transparent always-on-top windows. WebView2 ships with Win10/11.
- **Frontend:** SvelteKit 2 + Svelte 5 (runes) + Vite, static adapter (`fallback: index.html`).
- **Audio:** `cpal` (device default config) → resample to 16 kHz mono → `hound` WAV.
- **HTTP:** `reqwest` multipart POST to OpenAI transcriptions endpoint.
- **Injection:** `windows` crate `SendInput` + `KEYEVENTF_UNICODE`, chunked 64 chars.
- **Font:** `@fontsource-variable/space-grotesk` (imported in Svelte, NOT a CDN link).

## Rust modules (`src-tauri/src/`)
- `lib.rs` — Tauri setup, tray menu, hotkey handler, state machine glue, overlay show/hide.
- `main.rs` — entry, calls `liquidvoice_lib::run()`.
- `audio.rs` — `AudioRecorder` (cpal stream, `!Send` wrapper), `pcm_to_wav`, linear resample.
- `transcribe.rs` — async OpenAI call, 15 s timeout.
- `inject.rs` — `type_text` (Win32, `cfg(windows)`; no-op stub elsewhere).
- `config.rs` — `AppConfig` load/save at `%APPDATA%/liquidvoice/config.json`.

## Frontend (`src/`)
- `routes/+page.svelte` — overlay page; listens to Tauri events `state`, `mic-level`, `error-msg`; drives `Capsule` target 0/1. Dev-preview mode when `__TAURI_INTERNALS__` absent.
- `components/Capsule.svelte` — **the entire overlay**: one SVG, pure-rAF timeline (`raw` 0..1), goo-filter liquid morph (dot→stretch→capsule), mic well, tapered waveform, timer, status dot, breathing underglow.
- `routes/settings/+page.svelte` — settings window (API key, model, hotkey, trigger mode, language, prompt).

## State machine
`IDLE` (tray only) ─hotkey─▶ `LISTENING` (overlay visible, mic on) ─release/toggle─▶ `PROCESSING` (transcribe) ─▶ inject text ─▶ collapse overlay ─▶ `IDLE`. Errors shake + 5 s caption.

## Run / build
- `npm run tauri dev` — full app, hot reload. Logs (Rust errors, audio, API) print to this terminal.
- `npm run dev` — frontend only at `localhost:1420`; auto-plays morph with fake mic (no Tauri backend).
- `npm run tauri build` — NSIS installer at `src-tauri/target/release/bundle/nsis/`.

## Conventions / gotchas
- **Svelte 5 reactivity:** any value derived from a `$state` MUST be `$derived`, never a top-level `const` — a plain `const` captures the initial value once and freezes (this broke the morph: geometry stuck at the dot). Inside `$effect`/rAF, read states via `untrack(() => x)`.
- **Mic config:** never request 16 kHz directly — Windows mics reject it ("config not supported"). Capture device default (often 48 kHz stereo), mix to mono, resample on `stop()`.
- **Transparent window:** `shadow: false`, `decorations: false`, `focusable: false`, `skipTaskbar: true`; call `set_ignore_cursor_events(true)` so it's click-through; `hide()` the window when done (CSS opacity alone leaves a dead rectangle).
- **Tray icon:** define ONCE in Rust (`TrayIconBuilder`), not also in `tauri.conf.json` `trayIcon` — else two icons.
- **Hotkey:** `tauri-plugin-global-shortcut`; event field is `event.state` (`ShortcutState::Pressed/Released`), not enum variants on the event.
- **`State` borrows:** bind `let state = app.state();` then `drop(state)` before any `.await`/spawn or nested lock to avoid "does not live long enough".
- **Smart App Control** on Win11 blocks `cargo` (os error 4551) — disable it or use the CI artifact.

## Config keys
`api_key`, `model`, `hotkey` (default `Ctrl+Space`), `trigger_mode` (`push-to-talk`|`toggle`), `language`, `prompt`, `theme`, `max_recording_sec`.
