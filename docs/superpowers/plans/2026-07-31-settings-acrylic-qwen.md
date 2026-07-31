# Settings acrylic + Qwen ASR Implementation Plan

> **For agentic workers:** Inline execution in this session (user requested implement).

**Goal:** Overlay acrylic blur toggle, collapsible settings, Qwen ASR model, default mini model, waveform rebalance.

**Architecture:** Config gains `glass_blur`; `transcribe` routes by model; overlay show/hide applies/clears acrylic via `window-vibrancy`; settings uses collapsible sections.

**Tech Stack:** Tauri 2, window-vibrancy, reqwest, Svelte 5

---

### Files
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/config.rs`, `src-tauri/src/transcribe.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/audio.rs`, `src/routes/settings/+page.svelte`, `src/components/Capsule.svelte`, `AGENTS.md` (config keys)

### Tasks
1. Config + tests for `glass_blur`, models, default mini
2. Transcribe OpenAI vs Qwen routing
3. Acrylic helpers in lib.rs on show/hide
4. Settings collapsible UI + blur + key label
5. Waveform gate/bar (partially done)
6. `cargo test --lib` + `cargo check`
