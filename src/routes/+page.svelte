<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import Capsule from '../components/Capsule.svelte';
  import '@fontsource-variable/space-grotesk';

  type Mode = 'listen' | 'process' | 'error';

  let visible = $state(false);
  let target = $state<0 | 1>(0);
  let mode = $state<Mode>('listen');
  let level = $state(0);
  let elapsed = $state(0);
  let errorMsg = $state('');

  let startTs = 0;
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // timer ticks while listening
  $effect(() => {
    if (!visible || mode !== 'listen') return;
    let raf = 0;
    const tick = () => {
      elapsed = Math.floor((performance.now() - startTs) / 1000);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  onMount(() => {
    if (!isTauri) {
      // dev preview: play the morph + fake mic + counting timer
      document.documentElement.classList.add('dev');
      visible = true;
      target = 1;
      mode = 'listen';
      startTs = performance.now();
      let k = 0;
      const id = setInterval(() => {
        k += 0.08;
        level = 0.12 + Math.abs(Math.sin(k)) * 0.4 + Math.random() * 0.06;
      }, 50);
      return () => clearInterval(id);
    }

    const off: (() => void)[] = [];

    listen<number>('mic-level', (e) => {
      if (mode === 'listen') level = e.payload;
    }).then((fn) => off.push(fn));

    listen<string>('state', (e) => {
      const s = e.payload;
      if (s === 'listening') {
        errorMsg = '';
        elapsed = 0;
        level = 0;
        startTs = performance.now();
        mode = 'listen';
        visible = true;
        target = 1;
      } else if (s === 'processing') {
        mode = 'process';
      } else if (s === 'done') {
        target = 0; // collapse back to a dot, then unmount
      } else if (s === 'error') {
        mode = 'error';
        target = 1;
        visible = true;
        setTimeout(() => (target = 0), 5000);
      }
    }).then((fn) => off.push(fn));

    listen<string>('error-msg', (e) => {
      errorMsg = e.payload;
    }).then((fn) => off.push(fn));

    return () => off.forEach((fn) => fn());
  });

  function collapsed() {
    visible = false;
    mode = 'listen';
    errorMsg = '';
    level = 0;
  }
</script>

<div class="stage" class:visible class:shake={mode === 'error' && visible}>
  {#if visible}
    <Capsule {level} {target} {elapsed} {mode} onCollapsed={collapsed} />
    {#if mode === 'error' && errorMsg}
      <p class="caption">{errorMsg}</p>
    {/if}
  {/if}
</div>

<style>
  :global(body) {
    background: transparent;
    overflow: hidden;
    pointer-events: none;
    user-select: none;
  }

  /* Busy colorful field so backdrop-filter frost is visible in browser preview */
  :global(html.dev body) {
    background:
      radial-gradient(ellipse 70% 50% at 18% 28%, #4c1d95 0%, transparent 55%),
      radial-gradient(ellipse 60% 45% at 82% 18%, #1e3a8a 0%, transparent 50%),
      radial-gradient(ellipse 50% 55% at 62% 88%, #9d174d 0%, transparent 50%),
      radial-gradient(ellipse 40% 40% at 12% 82%, #0e7490 0%, transparent 45%),
      linear-gradient(165deg, #0a0814, #07070a 55%, #0c0a14);
  }

  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    padding-top: 6px;
    height: 100vh;
    gap: 6px;
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .stage.visible {
    opacity: 1;
  }

  .stage.shake {
    animation: shake 0.4s ease;
  }

  .caption {
    margin: 0;
    font-family: 'Space Grotesk Variable', 'Segoe UI', sans-serif;
    font-size: 10px;
    letter-spacing: 0.04em;
    color: #fda4af;
    text-align: center;
    max-width: 360px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-6px); }
    75% { transform: translateX(6px); }
  }
</style>
