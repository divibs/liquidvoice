# LiquidVoice

Windows system-wide speech-to-text dictation. Hold (or toggle) a hotkey → a frosted liquid-glass overlay appears → your mic audio is transcribed with OpenAI → text is typed at the cursor. Lives in the system tray.

**Platform:** Windows 10 / 11  
**Version:** 0.1.0  
**Repo:** [github.com/divibs/liquidvoice](https://github.com/divibs/liquidvoice)

---

## What it does

1. Runs quietly in the **system tray** (including the hidden-icons area).
2. On hotkey (**Ctrl+Space** by default), shows a small **liquid-glass pill** near the top of the screen with mic level, waveform, and timer.
3. Captures microphone audio, resamples to 16 kHz mono WAV.
4. Sends audio to OpenAI (`gpt-4o-transcribe` or `gpt-4o-mini-transcribe`).
5. Injects the transcript into the focused app with Win32 **`SendInput`** (Unicode keystrokes — **never the clipboard**).
6. Collapses the overlay and returns to idle.

Silent / near-silent clips are skipped so the model doesn’t hallucinate filler like “Thank you.”

---

## Features

| Feature | Details |
|--------|---------|
| Push-to-talk or toggle | Hold to talk, or press once to start/stop |
| Global hotkey | Configurable (default `Ctrl+Space`) |
| Liquid-glass overlay | Dark frosted pill, elastic morph, red status dot |
| Settings window | API key, model, hotkey, trigger mode, language hint, custom vocabulary, wallpaper, launch at login |
| Wallpapers | **Blueprint**, **Signal**, **Zinc** (settings background themes) |
| Launch at login | Optional Windows startup registration |
| Tray menu | Settings / Quit |
| Privacy of install | Config is per Windows user under `%APPDATA%` — not baked into the installer |

---

## Install (end users)

1. Download the latest **LiquidVoice-Windows** artifact from [GitHub Actions](https://github.com/divibs/liquidvoice/actions) (workflow **Build Windows Installer**), or build locally (below).
2. Run the NSIS installer (e.g. `LiquidVoice_0.1.0_x64-setup.exe`).
3. Open **Settings** from the tray icon.
4. Paste your **OpenAI API key** → **Save**.
5. Optionally set **Launch at login → On** → **Save**.
6. Focus any text field, hold **Ctrl+Space**, speak, release.

Typical footprint (measured on a real install): installer / app on the order of a few MB; idle RAM often under ~10 MB for the main process (WebView2 may appear as separate processes).

---

## Requirements

- Windows 10 or 11 (WebView2 is included with modern Windows)
- An [OpenAI API key](https://platform.openai.com/api-keys) with access to the transcription models
- Microphone permission for the app

---

## Settings & config

| Setting | Purpose |
|---------|---------|
| OpenAI API Key | Required for transcription |
| Model | `gpt-4o-transcribe` or `gpt-4o-mini-transcribe` |
| Hotkey | Global shortcut string (e.g. `Ctrl+Space`) |
| Trigger mode | Hold to talk / Toggle |
| Language hint | Optional (e.g. `en`) |
| Custom vocabulary | Optional prompt / term hints for the API |
| Wallpaper | Blueprint / Signal / Zinc |
| Launch at login | Register or remove Windows autostart |

Stored at:

```text
%APPDATA%\liquidvoice\config.json
```

Keys: `api_key`, `model`, `hotkey`, `trigger_mode`, `language`, `prompt`, `theme`, `max_recording_sec`.

Your API key stays on that PC’s user profile. Installing the same `.exe` on another machine does **not** copy your settings.

---

## How it works (architecture)

```text
Hotkey → LISTENING (overlay + mic)
      → PROCESSING (OpenAI transcription)
      → SendInput type-out
      → IDLE (overlay hidden)
```

| Layer | Tech |
|-------|------|
| Shell | [Tauri 2](https://tauri.app/) (Rust) — tray, global hotkey, transparent windows |
| UI | SvelteKit 2 + Svelte 5 + Vite (static adapter) |
| Audio | `cpal` (device default rate) → mono mix → resample to 16 kHz → `hound` WAV |
| API | `reqwest` multipart POST to OpenAI transcriptions (15 s timeout) |
| Injection | `windows` crate, `SendInput` + `KEYEVENTF_UNICODE`, 64-char chunks |
| Autostart | `tauri-plugin-autostart` |
| Font | `@fontsource-variable/space-grotesk` (bundled, not CDN) |

Rust modules live under `src-tauri/src/`: `lib.rs`, `audio.rs`, `transcribe.rs`, `inject.rs`, `config.rs`.  
Frontend: overlay (`src/routes/+page.svelte`), glass pill (`src/components/Capsule.svelte`), settings (`src/routes/settings/+page.svelte`).

Contributor / agent notes: see [AGENTS.md](./AGENTS.md).

---

## Develop

### Prerequisites

- Node.js 22+
- Rust (stable) via [rustup](https://rustup.rs/)
- On Windows: Visual Studio Build Tools with **Desktop development with C++**
- WebView2 (usually already installed on Win10/11)

> On Windows 11, **Smart App Control** can block `cargo` (os error 4551). Disable it for local builds, or use the CI installer artifact.

### Commands

```bash
npm ci
npm run tauri dev      # full app + hot reload
npm run dev            # UI only at http://localhost:1420 (fake mic preview)
npm run check          # svelte-check
npm run tauri build    # release + NSIS installer
```

### Build output (Windows)

| Artifact | Path |
|----------|------|
| NSIS installer | `src-tauri/target/release/bundle/nsis/*.exe` |
| App binary | `src-tauri/target/release/liquidvoice.exe` |

CI (`.github/workflows/build.yml`) builds the installer on every push to `main` and uploads artifact **LiquidVoice-Windows**.

---

## Privacy & security notes

- Audio is sent to OpenAI for transcription when you finish a utterance (not for silent skips).
- Transcript text is typed into whatever app is focused; treat that as a trust boundary.
- API key is stored in plaintext in `%APPDATA%\liquidvoice\config.json` (v0.1). Do not share that file.
- Injection into elevated (admin) windows may fail silently due to Windows UIPI.

---

## License

MIT — see [LICENSE](./LICENSE) if present, or `package.json` (`"license": "MIT"`).
