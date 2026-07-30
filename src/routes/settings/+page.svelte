<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let apiKey = $state('');
  let model = $state('gpt-4o-transcribe');
  let hotkey = $state('Ctrl+Space');
  let triggerMode = $state('push-to-talk');
  let language = $state('');
  let prompt = $state('');
  let theme = $state('auto');
  let saved = $state(false);

  onMount(async () => {
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
  <h1>LiquidVoice</h1>

  <label>
    OpenAI API Key
    <input type="password" bind:value={apiKey} placeholder="sk-..." />
  </label>

  <label>
    Model
    <select bind:value={model}>
      <option value="gpt-4o-transcribe">gpt-4o-transcribe</option>
      <option value="gpt-4o-mini-transcribe">gpt-4o-mini-transcribe</option>
    </select>
  </label>

  <label>
    Hotkey
    <input bind:value={hotkey} placeholder="Ctrl+Space" />
  </label>

  <label>
    Trigger Mode
    <select bind:value={triggerMode}>
      <option value="push-to-talk">Push-to-talk (hold)</option>
      <option value="toggle">Toggle (press start/stop)</option>
    </select>
  </label>

  <label>
    Language hint (optional)
    <input bind:value={language} placeholder="en" />
  </label>

  <label>
    Custom prompt / vocabulary (optional)
    <input bind:value={prompt} placeholder="Technical terms..." />
  </label>

  <label>
    Theme
    <select bind:value={theme}>
      <option value="auto">Auto (system)</option>
      <option value="dark">Dark</option>
      <option value="light">Light</option>
    </select>
  </label>

  <button onclick={save}>
    {saved ? '✓ Saved' : 'Save'}
  </button>
</div>

<style>
  .settings {
    font-family: 'Segoe UI', system-ui, sans-serif;
    padding: 20px;
    max-width: 320px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: #1a1a1e;
    color: #e4e4e7;
    min-height: 100vh;
  }

  h1 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 4px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: #a1a1aa;
  }

  input, select {
    padding: 6px 10px;
    border-radius: 6px;
    border: 1px solid #3f3f46;
    background: #27272a;
    color: #e4e4e7;
    font-size: 13px;
    outline: none;
  }

  input:focus, select:focus {
    border-color: #6366f1;
  }

  button {
    margin-top: 8px;
    padding: 8px;
    border-radius: 6px;
    border: none;
    background: #6366f1;
    color: white;
    font-size: 13px;
    cursor: pointer;
  }

  button:hover {
    background: #4f46e5;
  }
</style>
