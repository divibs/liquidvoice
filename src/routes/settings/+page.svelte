<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { onMount } from 'svelte';
  import '@fontsource-variable/space-grotesk';

  type Wallpaper = 'blueprint' | 'signal' | 'zinc';
  type TriggerMode = 'push-to-talk' | 'toggle';

  interface AppConfig {
    api_key: string;
    model: string;
    hotkey: string;
    trigger_mode: TriggerMode;
    language: string;
    prompt: string;
    theme: string;
    max_recording_sec: number;
  }

  let apiKey = $state('');
  let model = $state('gpt-4o-transcribe');
  let hotkey = $state('Ctrl+Space');
  let triggerMode = $state<TriggerMode>('push-to-talk');
  let language = $state('');
  let prompt = $state('');
  let wallpaper = $state<Wallpaper>('blueprint');
  let maxRecordingSec = $state(60);
  let launchAtLogin = $state(false);
  let saved = $state(false);
  let saveError = $state('');
  let saving = $state(false);

  function normalizeTheme(t: string): Wallpaper {
    if (t === 'signal' || t === 'zinc' || t === 'blueprint') return t;
    return 'blueprint';
  }

  function normalizeTrigger(t: string): TriggerMode {
    return t === 'toggle' ? 'toggle' : 'push-to-talk';
  }

  onMount(async () => {
    document.documentElement.classList.add('settings-page');
    try {
      const cfg = await invoke<AppConfig>('get_config');
      apiKey = cfg.api_key ?? '';
      model = cfg.model || 'gpt-4o-transcribe';
      hotkey = cfg.hotkey || 'Ctrl+Space';
      triggerMode = normalizeTrigger(cfg.trigger_mode);
      language = cfg.language ?? '';
      prompt = cfg.prompt ?? '';
      wallpaper = normalizeTheme(cfg.theme ?? 'blueprint');
      maxRecordingSec = Math.min(300, Math.max(5, cfg.max_recording_sec || 60));
    } catch (e) {
      saveError = String(e);
    }
    try {
      launchAtLogin = await isEnabled();
    } catch {
      launchAtLogin = false;
    }
  });

  async function save() {
    if (saving) return;
    saving = true;
    saveError = '';
    try {
      await invoke('save_config', {
        config: {
          api_key: apiKey.trim(),
          model,
          hotkey: hotkey.trim() || 'Ctrl+Space',
          trigger_mode: triggerMode,
          language: language.trim(),
          prompt: prompt.trim(),
          theme: wallpaper,
          max_recording_sec: Math.min(300, Math.max(5, Math.round(Number(maxRecordingSec)) || 60)),
        } satisfies AppConfig,
      });
      try {
        if (launchAtLogin) await enable();
        else await disable();
      } catch (e) {
        console.error('autostart update failed', e);
        saveError = 'Saved, but launch-at-login failed';
      }
      saved = true;
      setTimeout(() => (saved = false), 1500);
    } catch (e) {
      saveError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="shell" data-wall={wallpaper}>
  <div class="wall" aria-hidden="true">
    <div class="wall-layer"></div>
  </div>

  <div class="settings">
    <header>
      <span class="status"></span>
      <div class="titles">
        <h1>LiquidVoice</h1>
        <p class="sub">Settings</p>
      </div>
      <span class="ver">v0.1</span>
    </header>

    <div class="scroll">
      <section class="panel">
        <div class="frost" aria-hidden="true"></div>
        <div class="grain" aria-hidden="true"></div>
        <div class="panel-body">
          <label class="field">
            <span class="label">OpenAI API Key</span>
            <input type="password" bind:value={apiKey} placeholder="sk-..." autocomplete="off" />
          </label>

          <label class="field">
            <span class="label">Model</span>
            <select bind:value={model}>
              <option value="gpt-4o-transcribe">gpt-4o-transcribe</option>
              <option value="gpt-4o-mini-transcribe">gpt-4o-mini-transcribe</option>
            </select>
          </label>
        </div>
      </section>

      <section class="panel">
        <div class="frost" aria-hidden="true"></div>
        <div class="grain" aria-hidden="true"></div>
        <div class="panel-body">
          <label class="field">
            <span class="label">Hotkey</span>
            <input bind:value={hotkey} placeholder="Ctrl+Space" spellcheck="false" />
          </label>

          <div class="field">
            <span class="label">Trigger Mode</span>
            <div class="segmented">
              <button
                type="button"
                class:active={triggerMode === 'push-to-talk'}
                onclick={() => (triggerMode = 'push-to-talk')}
              >
                Hold to talk
              </button>
              <button
                type="button"
                class:active={triggerMode === 'toggle'}
                onclick={() => (triggerMode = 'toggle')}
              >
                Toggle
              </button>
            </div>
          </div>

          <div class="field">
            <span class="label">Launch at login</span>
            <div class="segmented">
              <button
                type="button"
                class:active={!launchAtLogin}
                onclick={() => (launchAtLogin = false)}
              >
                Off
              </button>
              <button
                type="button"
                class:active={launchAtLogin}
                onclick={() => (launchAtLogin = true)}
              >
                On
              </button>
            </div>
          </div>

          <label class="field">
            <span class="label">Max recording <em>{maxRecordingSec}s</em></span>
            <input
              type="range"
              min="5"
              max="300"
              step="5"
              value={maxRecordingSec}
              oninput={(e) => {
                maxRecordingSec = Number(e.currentTarget.value);
              }}
            />
          </label>
        </div>
      </section>

      <section class="panel">
        <div class="frost" aria-hidden="true"></div>
        <div class="grain" aria-hidden="true"></div>
        <div class="panel-body">
          <label class="field">
            <span class="label">Language hint <em>optional</em></span>
            <input bind:value={language} placeholder="en" spellcheck="false" />
          </label>

          <label class="field">
            <span class="label">Custom vocabulary <em>optional</em></span>
            <input bind:value={prompt} placeholder="Technical terms, names..." />
          </label>
        </div>
      </section>

      <section class="panel">
        <div class="frost" aria-hidden="true"></div>
        <div class="grain" aria-hidden="true"></div>
        <div class="panel-body">
          <div class="field">
            <span class="label">Wallpaper</span>
            <div class="walls">
              <button
                type="button"
                class="wall-pick blueprint"
                class:active={wallpaper === 'blueprint'}
                onclick={() => (wallpaper = 'blueprint')}
                aria-label="Blueprint wallpaper"
              >
                <span class="wall-preview"></span>
                <span class="wall-name">Blueprint</span>
              </button>
              <button
                type="button"
                class="wall-pick signal"
                class:active={wallpaper === 'signal'}
                onclick={() => (wallpaper = 'signal')}
                aria-label="Signal wallpaper"
              >
                <span class="wall-preview"></span>
                <span class="wall-name">Signal</span>
              </button>
              <button
                type="button"
                class="wall-pick zinc"
                class:active={wallpaper === 'zinc'}
                onclick={() => (wallpaper = 'zinc')}
                aria-label="Zinc wallpaper"
              >
                <span class="wall-preview"></span>
                <span class="wall-name">Zinc</span>
              </button>
            </div>
          </div>
        </div>
      </section>
    </div>

    {#if saveError}
      <p class="err" role="alert">{saveError}</p>
    {/if}

    <button type="button" class="save" onclick={save} class:saved disabled={saving}>
      {saved ? 'Saved' : saving ? 'Saving…' : 'Save changes'}
    </button>
  </div>
</div>

<style>
  :global(html.settings-page),
  :global(html.settings-page body) {
    margin: 0;
    height: 100%;
    background: #0a0a0b;
    overflow: hidden;
  }

  .shell {
    position: relative;
    height: 100vh;
    height: 100dvh;
    font-family: 'Space Grotesk Variable', 'Segoe UI', system-ui, sans-serif;
    color: rgba(255, 255, 255, 0.92);
    overflow: hidden;
  }

  .wall {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .wall-layer {
    position: absolute;
    inset: 0;
  }

  /* 1 · Blueprint: engineering grid, ink + cyan hairlines */
  .shell[data-wall='blueprint'] .wall-layer {
    background-color: #071018;
    background-image:
      linear-gradient(rgba(56, 189, 248, 0.09) 1px, transparent 1px),
      linear-gradient(90deg, rgba(56, 189, 248, 0.09) 1px, transparent 1px),
      linear-gradient(rgba(56, 189, 248, 0.04) 1px, transparent 1px),
      linear-gradient(90deg, rgba(56, 189, 248, 0.04) 1px, transparent 1px);
    background-size:
      48px 48px,
      48px 48px,
      8px 8px,
      8px 8px;
    background-position:
      -1px -1px,
      -1px -1px,
      -1px -1px,
      -1px -1px;
  }

  .shell[data-wall='blueprint'] .wall-layer::after {
    content: '';
    position: absolute;
    inset: 0;
    background:
      linear-gradient(90deg, rgba(7, 16, 24, 0.85) 0%, transparent 18%, transparent 82%, rgba(7, 16, 24, 0.85) 100%),
      linear-gradient(0deg, rgba(7, 16, 24, 0.9) 0%, transparent 22%, transparent 78%, rgba(7, 16, 24, 0.55) 100%);
  }

  /* 2 · Signal: CRT raster / scan, no glow blobs */
  .shell[data-wall='signal'] .wall-layer {
    background-color: #050805;
    background-image: repeating-linear-gradient(
      0deg,
      rgba(34, 197, 94, 0.07) 0 1px,
      transparent 1px 3px
    );
  }

  .shell[data-wall='signal'] .wall-layer::before {
    content: '';
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(
      90deg,
      transparent 0 11px,
      rgba(0, 0, 0, 0.35) 11px 12px
    );
    opacity: 0.55;
  }

  .shell[data-wall='signal'] .wall-layer::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      180deg,
      rgba(0, 0, 0, 0.55) 0%,
      transparent 18%,
      transparent 82%,
      rgba(0, 0, 0, 0.7) 100%
    );
  }

  /* 3 · Zinc: hard industrial bands, metal gray */
  .shell[data-wall='zinc'] .wall-layer {
    background-color: #121417;
    background-image: repeating-linear-gradient(
      -32deg,
      #121417 0 28px,
      #1a1d22 28px 56px,
      #0e1013 56px 70px
    );
  }

  .shell[data-wall='zinc'] .wall-layer::after {
    content: '';
    position: absolute;
    inset: 0;
    background:
      linear-gradient(115deg, rgba(255, 255, 255, 0.04) 0%, transparent 40%),
      linear-gradient(295deg, rgba(0, 0, 0, 0.45) 0%, transparent 45%);
  }

  .settings {
    position: relative;
    z-index: 1;
    height: 100%;
    padding: 12px 12px 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-sizing: border-box;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    padding: 0 2px 2px;
  }

  .status {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: 0 0 auto;
    background: radial-gradient(circle at 35% 30%, #fecaca, #ef4444 45%, #dc2626);
    box-shadow: 0 0 8px rgba(239, 68, 68, 0.65);
    animation: pulse 1.3s ease-in-out infinite;
  }

  .titles {
    flex: 1;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: #fff;
  }

  .sub {
    margin: 0;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.38);
  }

  .ver {
    font-size: 9px;
    letter-spacing: 0.12em;
    color: rgba(255, 255, 255, 0.28);
  }

  .scroll {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-right: 2px;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
  }

  .scroll::-webkit-scrollbar {
    width: 6px;
  }
  .scroll::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.18);
    border-radius: 99px;
  }

  .panel {
    position: relative;
    border-radius: 14px;
    overflow: hidden;
    border: none;
    flex: 0 0 auto;
    box-shadow:
      0 8px 22px rgba(0, 0, 0, 0.32),
      0 1px 4px rgba(0, 0, 0, 0.22);
  }

  .frost {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    pointer-events: none;
    background:
      linear-gradient(
        165deg,
        rgba(255, 255, 255, 0.1) 0%,
        rgba(255, 255, 255, 0.02) 38%,
        rgba(0, 0, 0, 0.22) 100%
      ),
      rgba(12, 10, 20, 0.62);
    -webkit-backdrop-filter: blur(22px) saturate(180%) brightness(0.9);
    backdrop-filter: blur(22px) saturate(180%) brightness(0.9);
    box-shadow:
      inset 0 1px 1px rgba(255, 255, 255, 0.12),
      inset 0 -12px 22px rgba(0, 0, 0, 0.35);
  }

  .grain {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    pointer-events: none;
    opacity: 0.32;
    mix-blend-mode: overlay;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 180 180' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 120px 120px;
  }

  .panel-body {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.42);
  }

  .label em {
    font-style: normal;
    font-weight: 400;
    letter-spacing: 0.02em;
    text-transform: none;
    color: rgba(255, 255, 255, 0.26);
    margin-left: 4px;
  }

  input,
  select {
    padding: 7px 10px;
    border-radius: 999px;
    border: none;
    background: rgba(0, 0, 0, 0.35);
    color: rgba(255, 255, 255, 0.92);
    font-size: 12.5px;
    font-family: inherit;
    outline: none;
    box-shadow:
      inset 0 1px 2px rgba(255, 255, 255, 0.06),
      inset 0 -2px 6px rgba(0, 0, 0, 0.35);
  }

  select {
    appearance: none;
    background-image: linear-gradient(45deg, transparent 50%, rgba(255, 255, 255, 0.45) 50%),
      linear-gradient(135deg, rgba(255, 255, 255, 0.45) 50%, transparent 50%);
    background-position:
      calc(100% - 15px) 50%,
      calc(100% - 10px) 50%;
    background-size:
      5px 5px,
      5px 5px;
    background-repeat: no-repeat;
    padding-right: 26px;
  }

  input:focus,
  select:focus {
    background: rgba(0, 0, 0, 0.45);
    box-shadow:
      inset 0 1px 2px rgba(255, 255, 255, 0.08),
      inset 0 -2px 6px rgba(0, 0, 0, 0.4),
      0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  input[type='range'] {
    width: 100%;
    padding: 6px 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    accent-color: rgba(255, 255, 255, 0.75);
    cursor: pointer;
  }

  input[type='range']:focus {
    background: transparent;
    box-shadow: none;
  }

  input::placeholder {
    color: rgba(255, 255, 255, 0.26);
  }

  .segmented {
    display: flex;
    gap: 3px;
    padding: 3px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.35);
    box-shadow:
      inset 0 1px 2px rgba(255, 255, 255, 0.05),
      inset 0 -2px 6px rgba(0, 0, 0, 0.35);
  }

  .segmented button {
    flex: 1;
    padding: 6px 0;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.42);
    font-size: 11.5px;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
  }

  .segmented button.active {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
    box-shadow:
      inset 0 1px 1px rgba(255, 255, 255, 0.15),
      0 2px 8px rgba(0, 0, 0, 0.25);
  }

  .walls {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }

  .wall-pick {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    color: rgba(255, 255, 255, 0.45);
    font-family: inherit;
  }

  .wall-preview {
    display: block;
    height: 44px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    overflow: hidden;
  }

  .wall-pick.active .wall-preview {
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.28),
      0 0 0 1px rgba(255, 255, 255, 0.2);
  }

  .wall-pick.active .wall-name {
    color: #fff;
  }

  .wall-name {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding-left: 2px;
  }

  .wall-pick.blueprint .wall-preview {
    background-color: #071018;
    background-image:
      linear-gradient(rgba(56, 189, 248, 0.2) 1px, transparent 1px),
      linear-gradient(90deg, rgba(56, 189, 248, 0.2) 1px, transparent 1px);
    background-size: 10px 10px;
  }

  .wall-pick.signal .wall-preview {
    background-color: #050805;
    background-image: repeating-linear-gradient(
      0deg,
      rgba(34, 197, 94, 0.22) 0 1px,
      transparent 1px 3px
    );
  }

  .wall-pick.zinc .wall-preview {
    background-image: repeating-linear-gradient(
      -32deg,
      #121417 0 8px,
      #1a1d22 8px 16px,
      #0e1013 16px 20px
    );
  }

  .err {
    margin: 0;
    flex: 0 0 auto;
    font-size: 11px;
    color: #fda4af;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .save {
    flex: 0 0 auto;
    padding: 9px;
    border: none;
    border-radius: 999px;
    background:
      linear-gradient(
        165deg,
        rgba(255, 255, 255, 0.14) 0%,
        rgba(255, 255, 255, 0.04) 40%,
        rgba(0, 0, 0, 0.15) 100%
      ),
      rgba(20, 16, 32, 0.75);
    -webkit-backdrop-filter: blur(16px) saturate(160%);
    backdrop-filter: blur(16px) saturate(160%);
    color: #fff;
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 650;
    letter-spacing: 0.04em;
    cursor: pointer;
    box-shadow:
      0 8px 22px rgba(0, 0, 0, 0.32),
      inset 0 1px 1px rgba(255, 255, 255, 0.14),
      inset 0 -8px 16px rgba(0, 0, 0, 0.28);
  }

  .save:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .save:disabled {
    opacity: 0.7;
    cursor: default;
  }

  .save.saved {
    color: #fecaca;
  }

  @keyframes pulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 1;
    }
    50% {
      transform: scale(0.82);
      opacity: 0.7;
    }
  }
</style>
