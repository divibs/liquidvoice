<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import Metaballs from '../components/Metaballs.svelte';
  import WaveformBars from '../components/WaveformBars.svelte';
  import StatusText from '../components/StatusText.svelte';
  import '@fontsource-variable/space-grotesk';

  type AppState = 'hidden' | 'listening' | 'processing' | 'done' | 'error';

  let state = $state<AppState>('hidden');
  let micLevel = $state(0);
  let errorMsg = $state('');
  let visible = $state(false);

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  onMount(() => {
    if (!isTauri) {
      document.documentElement.classList.add('dev');
      visible = true;
      state = 'listening';
      let t = 0;
      const interval = setInterval(() => {
        t += 0.08;
        micLevel = 0.12 + Math.abs(Math.sin(t)) * 0.35 + Math.random() * 0.08;
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

<div
  class="stage"
  class:visible
  class:error={state === 'error'}
  style="--level: {micLevel};"
>
  {#if visible}
    <div class="glow" class:fast={state === 'processing'}></div>

    <div class="pill-border" class:fast={state === 'processing'} class:err={state === 'error'}>
      <div class="pill">
        <Metaballs level={micLevel} active={state === 'listening'} />

        {#if state === 'listening'}
          <WaveformBars level={micLevel} />
        {:else if state === 'processing'}
          <div class="loading">
            {#each Array(3) as _, i}
              <span style="animation-delay: {i * 0.15}s"></span>
            {/each}
          </div>
        {:else if state === 'done'}
          <svg class="check" viewBox="0 0 24 24" width="20" height="20">
            <path d="M5 13l4 4L19 7" fill="none" stroke="#5eead4" stroke-width="2.5"
              stroke-linecap="round" stroke-linejoin="round" pathLength="1" />
          </svg>
        {:else if state === 'error'}
          <span class="bang">!</span>
        {/if}
      </div>
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
    background:
      radial-gradient(900px 500px at 70% 20%, rgba(34, 211, 238, 0.08), transparent 60%),
      radial-gradient(700px 400px at 20% 80%, rgba(251, 191, 36, 0.05), transparent 60%),
      linear-gradient(160deg, #10161d, #0a0f14 55%, #0d1219);
  }

  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    opacity: 0;
    transform: translateY(26px) scale(0.82);
    transition:
      opacity 0.28s ease,
      transform 0.45s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .stage.visible {
    opacity: 1;
    transform: translateY(0) scale(1);
  }

  .stage.error {
    animation: shake 0.4s ease;
  }

  /* ambient light spill behind the pill, intensity follows mic level */
  .glow {
    position: absolute;
    width: 250px;
    height: 60px;
    border-radius: 999px;
    background: conic-gradient(from var(--angle, 0deg),
      #22d3ee, #2dd4bf, #34d399, #22d3ee);
    filter: blur(22px);
    opacity: calc(0.25 + var(--level) * 0.55);
    animation: spin 5s linear infinite;
    transition: opacity 0.15s ease-out;
  }

  .glow.fast {
    animation-duration: 1.6s;
    opacity: 0.7;
  }

  .pill-border {
    position: relative;
    width: 240px;
    height: 52px;
    border-radius: 999px;
    padding: 1.5px;
    background: conic-gradient(from var(--angle, 0deg),
      #22d3ee, #2dd4bf 30%, #34d399 55%, #0e7490 80%, #22d3ee);
    animation: spin 4s linear infinite;
  }

  .pill-border.fast {
    animation-duration: 1.2s;
  }

  .pill-border.err {
    background: conic-gradient(from var(--angle, 0deg),
      #fb7185, #f43f5e 40%, #fb7185);
  }

  .pill {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 999px;
    background: rgba(10, 17, 22, 0.55);
    backdrop-filter: blur(22px) saturate(1.5);
    -webkit-backdrop-filter: blur(22px) saturate(1.5);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .loading {
    display: flex;
    gap: 7px;
    z-index: 1;
  }

  .loading span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #2dd4bf;
    animation: bounce 0.7s ease-in-out infinite;
  }

  .check {
    z-index: 1;
  }

  .check path {
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: draw 0.35s ease forwards;
  }

  .bang {
    font-family: 'Space Grotesk Variable', sans-serif;
    font-weight: 700;
    font-size: 20px;
    color: #fda4af;
    z-index: 1;
  }

  @keyframes bounce {
    0%, 100% { transform: translateY(0); opacity: 0.5; }
    50% { transform: translateY(-6px); opacity: 1; }
  }

  @keyframes draw {
    to { stroke-dashoffset: 0; }
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-6px); }
    75% { transform: translateX(6px); }
  }

  @keyframes spin {
    to { --angle: 360deg; }
  }

  @property --angle {
    syntax: '<angle>';
    initial-value: 0deg;
    inherits: false;
  }
</style>
