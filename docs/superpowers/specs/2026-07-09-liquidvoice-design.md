# LiquidVoice - Design Spec

Windows system-wide STT dictation app with liquid-animated overlay, powered by OpenAI gpt-4o-transcribe.

## Overview

Push-to-talk (or toggle) dictation that lives in the Windows system tray (hidden icons area). On hotkey trigger (default `Ctrl+Space`), a liquid blob overlay animates in at top-center of screen, captures mic audio, sends it to OpenAI's `gpt-4o-transcribe` API, and injects the resulting text directly at the cursor via Unicode keystroke simulation. No clipboard involvement.

## Requirements

- **Platform**: Windows 10/11 only
- **RAM**: <15MB idle (tray only), <50MB active (overlay + mic)
- **STT engine**: OpenAI `gpt-4o-transcribe` via REST API
- **UI**: Liquid/gooey animated overlay, top-center, mic-reactive waveform bars
- **Tray**: Hidden icons area, right-click → Settings / Quit
- **Hotkey**: Default `Ctrl+Space`, configurable
- **Trigger modes**: Push-to-talk (hold) and toggle (press start/stop), configurable
- **Text output**: Direct keystroke injection (`SendInput` + `KEYEVENTF_UNICODE`), never clipboard
- **Installer**: NSIS `.exe` via Tauri bundler

## Tech Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Shell | Tauri 2 (Rust) | <50MB RAM, native WebView2, tray, global hotkeys |
| Frontend | Svelte 5 + Vite | Tiny bundle, reactive, easy animation |
| Styling | Tailwind CSS 4 + custom CSS | Utility + keyframes |
| Animation | SVG gooey filter + CSS springs | Liquid metaball effect without heavy libs |
| Audio | `cpal` crate | Low-level mic capture, 16kHz mono PCM |
| HTTP | `reqwest` | Async OpenAI API calls |
| Text injection | Win32 `SendInput` via `windows` crate | Unicode keystroke simulation |
| Config | `serde_json` + file | Simple JSON config |

## Architecture

### Runtime States

```
IDLE ──(hotkey)──► LISTENING ──(release/toggle)──► PROCESSING ──(done)──► IDLE
```

| State | Visible | RAM | Activity |
|-------|---------|-----|----------|
| IDLE | Tray icon only | ~15MB | Rust event loop, hotkey listener. WebView window hidden. |
| LISTENING | Overlay top-center | ~45MB | Mic capture, amplitude events → JS, blob animating |
| PROCESSING | Overlay contracted | ~45MB | WAV encode → POST OpenAI → inject text → fade out |

### Key Decisions

- WebView window created once at startup, toggled `visible` - no destroy/recreate flicker
- Overlay: `transparent: true`, `decorations: false`, `always_on_top: true`, `focusable: false`
- Audio buffer: in-memory `Vec<i16>`, no temp files
- OpenAI call: async `reqwest`, non-blocking on Tauri's async runtime

## Rust Backend Modules

### `main.rs` - App lifecycle
- Tauri builder: tray icon, global hotkey registration, Tauri commands
- Tray: icon in hidden area, right-click menu → "Settings" / "Quit"
- Overlay window: created at startup, hidden, positioned top-center of primary monitor
- Hotkey events → state machine transitions + frontend events

### `hotkey.rs` - Global hotkey manager
- `tauri-plugin-global-shortcut` (Win32 `RegisterHotKey`)
- Default: `Ctrl+Space`, stored in config, re-registered on change
- Push-to-talk: keydown starts recording, keyup stops
- Toggle: single press flips between LISTENING and PROCESSING

### `audio.rs` - Mic capture
- `cpal` crate, 16kHz mono i16 PCM
- Opens default input device on LISTENING start
- Streams into `Vec<i16>` buffer
- Every ~50ms: compute RMS amplitude → emit `tauri::Event("mic-level", f32)` to frontend
- On stop: close stream, return buffer
- Hard cap: 60 seconds (~1.9MB WAV), auto-stops

### `transcribe.rs` - OpenAI API
- `reqwest::Client` POST to `https://api.openai.com/v1/audio/transcriptions`
- Multipart form fields:
  - `file`: WAV bytes (encoded from PCM buffer)
  - `model`: `"gpt-4o-transcribe"`
  - `response_format`: `"text"`
  - `language`: optional hint from config
  - `prompt`: optional custom vocabulary from config
- Timeout: 15 seconds
- Returns `Result<String, TranscribeError>`

### `inject.rs` - Text injection
- Win32 `SendInput` with `KEYEVENTF_UNICODE` per character
- No clipboard
- Batched: chunks of 64 chars with 1ms inter-chunk delay
- Handles Unicode (emoji, CJK, accented chars)

### `config.rs` - Configuration
- Path: `%APPDATA%/liquidvoice/config.json`
- Fields: `apiKey`, `model`, `hotkey`, `triggerMode`, `language`, `prompt`, `theme`, `maxRecordingSec`
- Load on startup, save on settings change
- API key in plaintext (v0.1); DPAPI encryption planned for v0.2

## Frontend - Liquid Overlay

### Window
- Size: 480×120px
- Position: top-center (`left: (screenW - 480) / 2`, `top: 24px`)
- Transparent, no decorations, click-through (`pointer-events: none`)

### Component Tree
```
App.svelte
└── LiquidOverlay.svelte
    ├── GooBlob.svelte          ← SVG gooey filter + metaball circles
    │   ├── blob circles        ← 3-5 circles, positions driven by mic amplitude
    │   └── WaveformBars.svelte ← 5-7 vertical bars, scaleY from amplitude
    └── StatusText.svelte       ← "Listening…" / "…" / error messages
```

### Animation States

| Transition | Effect | Technique |
|------------|--------|-----------|
| Hidden → Listening | Scale 0→1, spring overshoot, edges wobble | CSS `cubic-bezier(0.34, 1.56, 0.64, 1)` |
| Listening (idle) | Blob edges undulate with mic | SVG `feTurbulence` + `feDisplacementMap`, `baseFrequency` driven by `mic-level` events |
| Listening (bars) | Waveform bars bounce | `transform: scaleY()` from amplitude array |
| → Processing | Blob contracts to 60%, shimmer | CSS scale transition + gradient animation |
| → Done | Flash → drip-dissolve downward | SVG path morph + opacity + translateY |

### Gooey SVG Filter
```svg
<filter id="goo">
  <feGaussianBlur in="SourceGraphic" stdDeviation="6" result="blur"/>
  <feColorMatrix in="blur"
    values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -7"
    result="goo"/>
  <feComposite in="SourceGraphic" in2="goo" operator="atop"/>
</filter>
```
Multiple circles with slight position offsets → merged liquid metaball appearance. Mic amplitude modulates circle positions and turbulence.

### Theme
- Dark (default): blob `rgba(30,30,30,0.85)`, border glow `rgba(255,255,255,0.1)`, bars gradient `#6366f1 → #a855f7`
- Light: blob `rgba(255,255,255,0.9)`, dark bars
- Follows Windows `prefers-color-scheme`, overridable in config

## Settings Window

- Separate Tauri window: 320×400px, standard decorations
- Opened from tray right-click → "Settings"
- Fields: API key, model selector, hotkey recorder, trigger mode toggle, language, prompt, theme
- Saves to config.json on change

## Error Handling

| Error | Behavior |
|-------|----------|
| No API key set | Overlay flashes red, StatusText: "Set API key in settings" |
| Microphone unavailable | Overlay flashes red: "No microphone found" |
| Network / API error | Blob shakes, StatusText: "Transcription failed" for 2s |
| Silence (empty audio) | No API call, blob fades out silently |
| Recording exceeds 60s | Auto-stops, processes normally |

## Project Structure

```
liquidvoice/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── hotkey.rs
│   │   ├── audio.rs
│   │   ├── transcribe.rs
│   │   ├── inject.rs
│   │   └── config.rs
│   ├── icons/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
├── src/
│   ├── App.svelte
│   ├── components/
│   │   ├── LiquidOverlay.svelte
│   │   ├── GooBlob.svelte
│   │   ├── WaveformBars.svelte
│   │   └── StatusText.svelte
│   ├── Settings.svelte
│   ├── app.css
│   └── main.ts
├── index.html
├── package.json
├── svelte.config.js
├── vite.config.ts
└── tsconfig.json
```

## MVP Scope (v0.1)

1. System tray icon (hidden area) + right-click menu
2. Global hotkey `Ctrl+Space` (configurable)
3. Push-to-talk + toggle modes
4. Liquid overlay with mic-reactive waveform
5. gpt-4o-transcribe API integration
6. Direct keystroke text injection (no clipboard)
7. Settings window (API key, hotkey, mode, language)
8. NSIS installer

## Out of Scope (future)

- Hands-free / wake-word mode (v0.2)
- AI text cleanup / formatting pass (v0.2)
- DPAPI API key encryption (v0.2)
- Streaming / real-time partial transcription (v0.3)
- Multi-language auto-detection UI (v0.3)
- Custom dictionaries / word replacements (v0.3)

## Testing

- **Rust unit tests**: WAV encoding (`audio.rs`), HTTP mock (`transcribe.rs`), char chunking (`inject.rs`)
- **Frontend**: Manual visual verification of animations
- **Integration**: Manual E2E - hotkey → speak → text appears in target app
