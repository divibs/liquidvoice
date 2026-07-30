<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import GooBlob from '../components/GooBlob.svelte';
  import WaveformBars from '../components/WaveformBars.svelte';
  import StatusText from '../components/StatusText.svelte';

  type AppState = 'hidden' | 'listening' | 'processing' | 'done' | 'error';

  let state = $state<AppState>('hidden');
  let micLevel = $state(0);
  let errorMsg = $state('');
  let visible = $state(false);

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  onMount(() => {
    // Dev preview: simulate listening state with fake mic levels
    if (!isTauri) {
      document.documentElement.classList.add('dev');
      visible = true;
      state = 'listening';
      let t = 0;
      const interval = setInterval(() => {
        t += 0.1;
        micLevel = 0.15 + Math.abs(Math.sin(t)) * 0.4;
      }, 50);
      return () => clearInterval(interval);
    }

    const unlisteners: (() => void)[] = [];

    listen<number>('mic-level', (e) => {
      micLevel = e.payload;
    }).then(fn => unlisteners.push(fn));

    listen<string>('state', (e) => {
      const s = e.payload as AppState;
      state = s;
      if (s === 'listening' || s === 'processing' || s === 'error') {
        visible = true;
      }
      if (s === 'done') {
        setTimeout(() => { visible = false; state = 'hidden'; }, 600);
      }
      if (s === 'error') {
        setTimeout(() => { visible = false; state = 'hidden'; }, 5000);
      }
    }).then(fn => unlisteners.push(fn));

    listen<string>('error-msg', (e) => {
      errorMsg = e.payload;
    }).then(fn => unlisteners.push(fn));

    return () => unlisteners.forEach(fn => fn());
  });
</script>

<div class="overlay" class:visible class:error={state === 'error'}>
  {#if visible}
    <div class="blob-container" class:processing={state === 'processing'}>
      <GooBlob level={micLevel} active={state === 'listening'} />
      {#if state === 'listening'}
        <WaveformBars level={micLevel} />
      {/if}
      {#if state === 'processing'}
        <div class="shimmer"></div>
      {/if}
    </div>
    <StatusText {state} {errorMsg} />
  {/if}
</div>

<style>
  :global(body) {
    background: transparent;
    overflow: hidden;
    pointer-events: none;
    user-select: none;
  }

  :global(html.dev body) {
    background: #111113;
  }

  .overlay {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    opacity: 0;
    transform: scale(0.3) translateY(-20px);
    transition: opacity 0.3s ease, transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .overlay.visible {
    opacity: 1;
    transform: scale(1) translateY(0);
  }

  .overlay.error {
    animation: shake 0.4s ease;
  }

  .blob-container {
    position: relative;
    width: 200px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.3s ease;
  }

  .blob-container.processing {
    transform: scale(0.65);
  }

  .shimmer {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: linear-gradient(90deg, transparent, rgba(99, 102, 241, 0.3), transparent);
    animation: shimmer 1s infinite;
  }

  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-6px); }
    75% { transform: translateX(6px); }
  }
</style>
