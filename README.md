# LiquidVoice

Minimal Windows SST. Hold a hotkey, speak, and the text is typed at the cursor.

Runs in the system tray. Works in any app with a text field.

**Platform:** Windows 10 / 11  
**Version:** 0.1.0  
**Typical background use:** ~160 MB RAM, ~1% CPU

---

## Install

1. Download the liquidvoice installer.
2. Run the installer and open LiquidVoice from the system tray.
3. Open **Settings**, choose a model, add your API key, save.
4. Focus a text field, hold **Ctrl+Space**, speak, release.

Default hotkey: **Ctrl+Space**.

---

## Requirements

- Windows 10 or 11
- Microphone access
- An OpenAI and/or Qwen API key, depending on the model you select

---

## Settings

| Setting | Notes |
| --- | --- |
| Model | OpenAI or Qwen transcription models |
| API key | Separate keys can be saved per provider |
| Hotkey | Global shortcut (default `Ctrl+Space`) |
| Trigger mode | Hold to talk, or toggle |
| Language hint | Optional |
| Custom vocabulary | Optional terms to improve recognition |
| Frost glass | Overlay frost strength (0–100) |
| Wallpaper | Settings window theme |
| Launch at login | Optional |
| Max recording | Auto-stops a long take |

---

## Privacy

- Audio is sent over HTTPS to the selected transcription provider only; the
  provider may retain audio per their own data policies.
- Text is inserted into the focused application.
- API keys are stored in `%APPDATA%\liquidvoice\config.json`, encrypted with
  Windows DPAPI (user-scoped). Legacy plaintext keys are migrated automatically
  on the next save. Non-Windows dev builds store keys as plain text.
- No audio is written to disk; recordings exist in memory only for the duration
  of the take.

---

## License

MIT
