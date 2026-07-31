# Settings acrylic, Qwen ASR, and waveform rebalance - Design Spec

**Date:** 2026-07-31  
**Status:** Locked (user-approved)  
**Scope:** Real Windows acrylic on the listening overlay (settings toggle), collapsible settings layout, Qwen ASR model option with single switching API key field, default model change, and waveform sensitivity rebalance.

## Goal

1. Make the listening pill use real desktop acrylic blur (not CSS-only frost), controllable from Settings, on by default.
2. Reorganize Settings into collapsible sections so it feels less overcrowded.
3. Support `qwen-audio-3.0-asr-flash` alongside OpenAI models with one API key field that follows the selected model.
4. Keep the waveform flat when silent, but respond clearly when the user speaks.

## Locked decisions

| Decision | Choice |
|---|---|
| Blur type | Real Windows Acrylic on the **overlay pill window only** |
| Blur default | On (`glass_blur: true`) |
| Blur failure | Fall back to current solid frost; never crash |
| Settings layout | Collapsible sections: API · Dictation · Appearance |
| Default open section | API |
| Default model | `gpt-4o-mini-transcribe` |
| Models | `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`, `qwen-audio-3.0-asr-flash` |
| API keys | Single `api_key` field; label/provider switches with model |
| Qwen transport | DashScope multimodal / OpenAI-compatible ASR with local WAV as base64 data URL |
| Waveform | Flat when silent; stronger speech response (softer gate + punchier bars) |

## Architecture

### Config (`AppConfig`)

New / changed fields:

| Field | Type | Default | Notes |
|---|---|---|---|
| `glass_blur` | `bool` | `true` | Sanitize missing key to `true` for new installs; existing configs without the key get default on load |
| `model` | `string` | `gpt-4o-mini-transcribe` | Allowed list updated; unknown models reset to default |
| `api_key` | `string` | `""` | Same storage; meaning depends on model (OpenAI vs DashScope/QwenCloud) |
| `theme` | `string` | `blueprint` | Unchanged (wallpaper) |

`sanitize` updates:

- Allow the three models above.
- Default model string becomes `gpt-4o-mini-transcribe`.
- `glass_blur` present or defaulted to `true`.

### Overlay acrylic

- On Windows, when entering `LISTENING` (overlay shown), if `glass_blur` is true, apply Acrylic to the **overlay** window via Tauri / `window-vibrancy` (already in the dependency tree through Tauri).
- When leaving overlay visibility (`hide` after collapse, or before processing morph if the window remains visible), keep acrylic on while the pill is on screen; clear acrylic when the overlay window is fully hidden.
- Settings window is **not** acrylic in this pass (user chose overlay-only).
- CSS frost/tint/grain on the pill stay as the visual skin; acrylic provides the real desktop blur behind the transparent window.
- If apply fails, log and continue with current non-acrylic frost.

### Transcription routing

`transcribe` becomes provider-aware by model:

| Model | Endpoint | Auth |
|---|---|---|
| `gpt-4o-transcribe` / `gpt-4o-mini-transcribe` | `https://api.openai.com/v1/audio/transcriptions` multipart WAV | Bearer OpenAI key |
| `qwen-audio-3.0-asr-flash` | `POST https://dashscope-intl.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation` with model `qwen-audio-3.0-asr-flash` | Bearer DashScope / QwenCloud key |

Qwen path:

- Encode captured WAV bytes as a data URL (`data:audio/wav;base64,...`) in the multimodal `input.messages` audio/input_audio field per QwenCloud docs.
- Pass language hint / vocabulary as optional context when non-empty; omit when empty.
- Parse transcript text from the JSON response; map HTTP/auth errors into the existing `error-msg` path.

Empty key message should name the expected provider (OpenAI vs Qwen) based on selected model.

### Settings UI

Collapsible panels (one scroll):

1. **API** (default expanded)
   - Model `<select>` with three options (labels may show friendly names; values are exact model ids).
   - API key password field; label and placeholder switch:
     - OpenAI models: "OpenAI API Key" / `sk-...`
     - Qwen: "Qwen / DashScope API Key" / appropriate placeholder
2. **Dictation** (collapsed by default)
   - Hotkey, trigger mode, max recording, language hint, custom vocabulary, launch at login
3. **Appearance** (collapsed by default)
   - Wallpaper picks
   - Glass blur On/Off segmented control bound to `glass_blur`

Save still writes full config + autostart; rebind hotkey behavior unchanged. Saving `glass_blur` should take effect on the **next** listening session (or immediately if overlay is already visible: apply/clear acrylic).

### Waveform rebalance

Frontend (`Capsule.svelte`):

- Bar motion scales with speech energy only (flat at silence).
- Small dead-zone, sqrt punch, higher motion gain so desk-distance speech is visible.
- Slightly faster level follow and wave phase.

Backend (`audio.rs`):

- Noise gate milder than the over-aggressive `floor * 3.0 / 0.28` pass; target quiet ambient flat, normal speech clearly above zero (approx `floor * 2.2 / 0.16`).

## Data flow

```
Settings save
  -> config.json (api_key, model, glass_blur, ...)
  -> hotkey rebind (existing)
  -> next LISTENING: if glass_blur apply Acrylic on overlay

Hotkey LISTENING
  -> show overlay (+ acrylic if enabled)
  -> mic levels -> gated meter -> waveform

Hotkey stop
  -> PROCESSING
  -> WAV
  -> if OpenAI model: OpenAI transcriptions
     if Qwen model: DashScope ASR with base64 WAV
  -> inject / skip / error (existing state machine)
```

## Error handling

- Acrylic apply/clear failure: log, keep UI usable without blur.
- Missing API key: error caption naming OpenAI or Qwen based on model.
- Qwen/OpenAI HTTP failures: surface truncated provider message via existing `error-msg`.
- Invalid model in config: sanitize to default mini model.

## Out of scope

- Acrylic on the settings window.
- Separate stored keys for OpenAI and Qwen (single field only).
- Streaming ASR / partial transcripts.
- Changing injection, tray, or hotkey mechanics.

## Acceptance checks

- [ ] Silent mic: waveform stays flat.
- [ ] Speaking at normal desk distance: bars move clearly.
- [ ] New install: model defaults to `gpt-4o-mini-transcribe`, glass blur on.
- [ ] With blur on: overlay shows real desktop blur behind the pill.
- [ ] With blur off: overlay matches solid frost without acrylic.
- [ ] Settings: three collapsible sections; API open by default.
- [ ] Selecting Qwen switches API key label; save + dictate uses DashScope path.
- [ ] Selecting an OpenAI model uses the existing OpenAI path.
- [ ] Acrylic failure does not prevent listening/transcription.
