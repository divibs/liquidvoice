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

<div class="settings">
  <header>
    <span class="dot"></span>
    <h1>LiquidVoice</h1>
    <span class="ver">v0.1</span>
  </header>

  <section>
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
  </section>

  <section>
    <label class="field">
      <span class="label">Hotkey</span>
      <input bind:value={hotkey} placeholder="Ctrl+Space" />
    </label>

    <div class="field">
      <span class="label">Trigger Mode</span>
      <div class="segmented">
        <button
          class:active={triggerMode === 'push-to-talk'}
          onclick={() => (triggerMode = 'push-to-talk')}>
          Hold to talk
        </button>
        <button
          class:active={triggerMode === 'toggle'}
          onclick={() => (triggerMode = 'toggle')}>
          Toggle
        </button>
      </div>
    </div>
  </section>

  <section>
    <label class="field">
      <span class="label">Language hint <em>optional</em></span>
      <input bind:value={language} placeholder="en" />
    </label>

    <label class="field">
      <span class="label">Custom vocabulary <em>optional</em></span>
      <input bind:value={prompt} placeholder="Technical terms, names..." />
    </label>
  </section>

  <button class="save" onclick={save} class:saved>
    {saved ? '✓ Saved' : 'Save changes'}
  </button>
</div>

<style>
  .settings {
    font-family: 'Segoe UI', system-ui, sans-serif;
    min-height: 100vh;
    padding: 22px 22px 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    color: #ede9fe;
    background:
      radial-gradient(420px 240px at 85% -10%, rgba(139, 92, 246, 0.14), transparent 65%),
      radial-gradient(360px 220px at -10% 105%, rgba(217, 70, 239, 0.07), transparent 65%),
      linear-gradient(165deg, #0d0a16, #08060d 60%, #0a0812);
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 9px;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: linear-gradient(135deg, #c084fc, #8b5cf6);
    box-shadow: 0 0 10px rgba(139, 92, 246, 0.7);
    align-self: center;
    animation: pulse 2.4s ease-in-out infinite;
  }

  h1 {
    font-family: 'Space Grotesk Variable', sans-serif;
    font-size: 19px;
    font-weight: 700;
    letter-spacing: -0.01em;
    margin: 0;
    color: #f5f3ff;
  }

  .ver {
    font-size: 10px;
    letter-spacing: 0.14em;
    color: rgba(196, 167, 255, 0.45);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 11px;
    padding: 13px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid rgba(196, 167, 255, 0.09);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .label {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: rgba(196, 167, 255, 0.55);
  }

  .label em {
    font-style: normal;
    font-weight: 400;
    letter-spacing: 0.04em;
    text-transform: none;
    color: rgba(196, 167, 255, 0.3);
    margin-left: 5px;
  }

  input, select {
    padding: 7px 11px;
    border-radius: 7px;
    border: 1px solid rgba(196, 167, 255, 0.16);
    background: rgba(10, 8, 18, 0.7);
    color: #ede9fe;
    font-size: 13px;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  input:focus, select:focus {
    border-color: rgba(139, 92, 246, 0.6);
    box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.14);
  }

  input::placeholder {
    color: rgba(196, 167, 255, 0.25);
  }

  .segmented {
    display: flex;
    gap: 4px;
    padding: 3px;
    border-radius: 8px;
    background: rgba(10, 8, 18, 0.7);
    border: 1px solid rgba(196, 167, 255, 0.16);
  }

  .segmented button {
    flex: 1;
    padding: 6px 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: rgba(196, 167, 255, 0.5);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .segmented button.active {
    background: linear-gradient(135deg, rgba(139, 92, 246, 0.22), rgba(192, 132, 252, 0.18));
    color: #ddd6fe;
    box-shadow: inset 0 0 0 1px rgba(139, 92, 246, 0.4);
  }

  .save {
    margin-top: auto;
    padding: 10px;
    border: none;
    border-radius: 8px;
    background: linear-gradient(135deg, #8b5cf6, #7c3aed);
    color: #f5f3ff;
    font-family: 'Space Grotesk Variable', sans-serif;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: transform 0.12s ease, box-shadow 0.15s ease, filter 0.15s ease;
    box-shadow: 0 4px 18px rgba(124, 58, 237, 0.3);
  }

  .save:hover {
    filter: brightness(1.12);
    transform: translateY(-1px);
    box-shadow: 0 6px 22px rgba(124, 58, 237, 0.42);
  }

  .save:active {
    transform: translateY(0);
  }

  .save.saved {
    background: linear-gradient(135deg, #a78bfa, #8b5cf6);
  }

  @keyframes pulse {
    0%, 100% { box-shadow: 0 0 6px rgba(139, 92, 246, 0.5); }
    50% { box-shadow: 0 0 14px rgba(139, 92, 246, 0.95); }
  }
</style>
