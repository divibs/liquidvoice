<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import '@fontsource-variable/space-grotesk';

  let apiKey = $state('');
  let model = $state('gpt-4o-transcribe');
  let hotkey = $state('Ctrl+Space');
  let triggerMode = $state('push-to-talk');
  let language = $state('');
  let prompt = $state('');
  let theme = $state('auto');
  let saved = $state(false);

  onMount(async () => {
    document.documentElement.classList.add('settings-page');
    const cfg = await invoke<any>('get_config');
    apiKey = cfg.api_key;
    model = cfg.model;
    hotkey = cfg.hotkey;
    triggerMode = cfg.trigger_mode;
    language = cfg.language;
    prompt = cfg.prompt;
    theme = cfg.theme;
  });

  async function save() {
    await invoke('save_config', {
      config: {
        api_key: apiKey,
        model,
        hotkey,
        trigger_mode: triggerMode,
        language,
        prompt,
        theme,
        max_recording_sec: 60,
      },
    });
    saved = true;
    setTimeout(() => (saved = false), 1500);
  }
</script>

<div class="shell">
  <div class="field-bg" aria-hidden="true">
    <span class="orb o1"></span>
    <span class="orb o2"></span>
    <span class="orb o3"></span>
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

    <section class="panel">
      <div class="frost" aria-hidden="true"></div>
      <div class="grain" aria-hidden="true"></div>
      <div class="panel-body">
        <label class="field">
          <span class="label">OpenAI API Key</span>
          <input type="password" bind:value={apiKey} placeholder="sk-..." />
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
          <input bind:value={hotkey} placeholder="Ctrl+Space" />
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
      </div>
    </section>

    <section class="panel">
      <div class="frost" aria-hidden="true"></div>
      <div class="grain" aria-hidden="true"></div>
      <div class="panel-body">
        <label class="field">
          <span class="label">Language hint <em>optional</em></span>
          <input bind:value={language} placeholder="en" />
        </label>

        <label class="field">
          <span class="label">Custom vocabulary <em>optional</em></span>
          <input bind:value={prompt} placeholder="Technical terms, names..." />
        </label>
      </div>
    </section>

    <button type="button" class="save" onclick={save} class:saved>
      {saved ? 'Saved' : 'Save changes'}
    </button>
  </div>
</div>

<style>
  :global(html.settings-page),
  :global(html.settings-page body) {
    margin: 0;
    background: #07070a;
    overflow: hidden;
  }

  .shell {
    position: relative;
    min-height: 100vh;
    font-family: 'Space Grotesk Variable', 'Segoe UI', system-ui, sans-serif;
    color: rgba(255, 255, 255, 0.92);
  }

  .field-bg {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background:
      radial-gradient(ellipse 70% 50% at 18% 12%, #4c1d95 0%, transparent 55%),
      radial-gradient(ellipse 55% 45% at 88% 18%, #1e3a8a 0%, transparent 50%),
      radial-gradient(ellipse 50% 50% at 70% 95%, #9d174d 0%, transparent 50%),
      #07070a;
  }

  .orb {
    position: absolute;
    border-radius: 50%;
    filter: blur(36px);
    pointer-events: none;
  }
  .o1 {
    width: 180px;
    height: 180px;
    left: -40px;
    top: 20%;
    background: #7c3aed;
    opacity: 0.45;
  }
  .o2 {
    width: 160px;
    height: 160px;
    right: -30px;
    top: 8%;
    background: #2563eb;
    opacity: 0.35;
  }
  .o3 {
    width: 140px;
    height: 140px;
    left: 35%;
    bottom: -20px;
    background: #db2777;
    opacity: 0.3;
  }

  .settings {
    position: relative;
    z-index: 1;
    min-height: 100vh;
    padding: 20px 18px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-sizing: border-box;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 2px 6px;
  }

  .status {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: 0 0 auto;
    background: radial-gradient(circle at 35% 30%, #fecaca, #ef4444 45%, #dc2626);
    box-shadow: 0 0 10px rgba(239, 68, 68, 0.7);
    animation: pulse 1.3s ease-in-out infinite;
  }

  .titles {
    flex: 1;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: #fff;
  }

  .sub {
    margin: 1px 0 0;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.4);
  }

  .ver {
    font-size: 10px;
    letter-spacing: 0.12em;
    color: rgba(255, 255, 255, 0.3);
  }

  /* Frosted panel — same material language as overlay capsule */
  .panel {
    position: relative;
    border-radius: 16px;
    overflow: hidden;
    border: none;
    box-shadow:
      0 10px 28px rgba(0, 0, 0, 0.35),
      0 2px 6px rgba(0, 0, 0, 0.25);
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
    opacity: 0.35;
    mix-blend-mode: overlay;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 180 180' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 120px 120px;
  }

  .panel-body {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 11px;
    padding: 13px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.45);
  }

  .label em {
    font-style: normal;
    font-weight: 400;
    letter-spacing: 0.02em;
    text-transform: none;
    color: rgba(255, 255, 255, 0.28);
    margin-left: 5px;
  }

  input,
  select {
    padding: 8px 11px;
    border-radius: 999px;
    border: none;
    background: rgba(0, 0, 0, 0.35);
    color: rgba(255, 255, 255, 0.92);
    font-size: 13px;
    font-family: inherit;
    outline: none;
    box-shadow:
      inset 0 1px 2px rgba(255, 255, 255, 0.06),
      inset 0 -2px 6px rgba(0, 0, 0, 0.35);
    transition: background 0.15s ease, box-shadow 0.15s ease;
  }

  select {
    appearance: none;
    background-image: linear-gradient(45deg, transparent 50%, rgba(255, 255, 255, 0.45) 50%),
      linear-gradient(135deg, rgba(255, 255, 255, 0.45) 50%, transparent 50%);
    background-position:
      calc(100% - 16px) 50%,
      calc(100% - 11px) 50%;
    background-size:
      5px 5px,
      5px 5px;
    background-repeat: no-repeat;
    padding-right: 28px;
  }

  input:focus,
  select:focus {
    background: rgba(0, 0, 0, 0.45);
    box-shadow:
      inset 0 1px 2px rgba(255, 255, 255, 0.08),
      inset 0 -2px 6px rgba(0, 0, 0, 0.4),
      0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  input::placeholder {
    color: rgba(255, 255, 255, 0.28);
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
    padding: 7px 0;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: rgba(255, 255, 255, 0.45);
    font-size: 12px;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .segmented button.active {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
    box-shadow:
      inset 0 1px 1px rgba(255, 255, 255, 0.15),
      0 2px 8px rgba(0, 0, 0, 0.25);
  }

  .save {
    margin-top: auto;
    position: relative;
    padding: 11px;
    border: none;
    border-radius: 999px;
    overflow: hidden;
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
    font-size: 13px;
    font-weight: 650;
    letter-spacing: 0.04em;
    cursor: pointer;
    box-shadow:
      0 10px 28px rgba(0, 0, 0, 0.35),
      inset 0 1px 1px rgba(255, 255, 255, 0.14),
      inset 0 -8px 16px rgba(0, 0, 0, 0.28);
    transition: transform 0.12s ease, filter 0.15s ease;
  }

  .save:hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  .save:active {
    transform: translateY(0);
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
