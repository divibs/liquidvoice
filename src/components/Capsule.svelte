<script lang="ts">
  import { untrack } from 'svelte';

  let {
    level = 0,
    target = 0,
    elapsed = 0,
    mode = 'listen',
    onCollapsed = () => {},
  } = $props<{
    level: number;
    target: 0 | 1;
    elapsed: number;
    mode: 'listen' | 'process' | 'error';
    onCollapsed?: () => void;
  }>();

  // morph progress 0..1
  let raw = $state(0);
  let lvl = $state(0);
  let wt = $state(0);
  let collapsed = false;

  const OPEN = 650;
  const CLOSE = 380;
  const MAX_W = 168;
  const MAX_H = 34;
  const N = 10;

  $effect(() => {
    let raf = 0;
    let last = performance.now();
    const loop = (now: number) => {
      const dt = Math.min(50, now - last);
      last = now;

      const tgt = untrack(() => target);
      raw = tgt === 1 ? Math.min(1, raw + dt / OPEN) : Math.max(0, raw - dt / CLOSE);

      const lv = untrack(() => level);
      const md = untrack(() => mode);
      const want = md === 'process' ? 0.05 : lv;
      lvl += (want - lvl) * Math.min(1, dt * 0.012);

      wt += dt * 0.0035;

      if (tgt === 0 && raw <= 0 && !collapsed) {
        collapsed = true;
        untrack(() => onCollapsed)();
      } else if (tgt === 1) {
        collapsed = false;
      }

      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });

  const clamp01 = (x: number) => (x < 0 ? 0 : x > 1 ? 1 : x);
  const smooth = (e0: number, e1: number, x: number) => {
    const u = clamp01((x - e0) / (e1 - e0));
    return u * u * (3 - 2 * u);
  };
  const lerp = (a: number, b: number, x: number) => a + (b - a) * x;

  function springKick(t: number) {
    const u = clamp01(t / 0.32);
    return Math.exp(-3.2 * u) * Math.cos(u * Math.PI * 2.2);
  }

  const appear = $derived(smooth(0, 0.14, raw));
  const stretch = $derived(smooth(0.06, 0.48, raw));
  const refine = $derived(smooth(0.28, 0.85, raw));
  const settle = $derived(smooth(0.62, 1, raw));
  const pinch = $derived(Math.sin(smooth(0.08, 0.5, raw) * Math.PI));

  const pillW = $derived(lerp(12, MAX_W, stretch) * appear);
  const pillH = $derived(
    Math.max(12 * appear, lerp(12, MAX_H, refine) * appear) * (1 - pinch * 0.12),
  );
  const uiOp = $derived(smooth(0.38, 0.72, raw));
  const uiY = $derived(lerp(6, 0, smooth(0.4, 0.78, raw)));
  const scale = $derived(
    appear * (1 + springKick(raw) * 0.22 * appear + Math.sin(settle * Math.PI) * 0.045 * (1 - settle)),
  );

  const mm = $derived(String(Math.floor(elapsed / 60)).padStart(2, '0'));
  const ss = $derived(String(elapsed % 60).padStart(2, '0'));

  const err = $derived(mode === 'error');
  const processing = $derived(mode === 'process');

  function barH(i: number) {
    const taper = Math.sin((Math.PI * i) / (N - 1));
    const amp = 0.5 + 0.5 * Math.sin(wt + i * 0.55);
    return Math.max(2, (2.2 + taper * (2.8 + lvl * 11) * amp) * uiOp);
  }
</script>

<div
  class="capsule"
  class:error={err}
  style:--w="{pillW}px"
  style:--h="{pillH}px"
  style:--s={scale}
  style:--ui={uiOp}
  style:--uy="{uiY}px"
>
  <div class="glass">
    <div class="chrome">
      <div class="mic" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <rect x="9" y="2" width="6" height="11" rx="3" fill="currentColor" stroke="none" />
          <path d="M5 11a7 7 0 0 0 14 0" />
          <line x1="12" y1="18" x2="12" y2="22" />
          <line x1="8" y1="22" x2="16" y2="22" />
        </svg>
      </div>

      <div class="wave" aria-hidden="true">
        {#each Array(N) as _, i}
          <i style:--bh="{barH(i)}px"></i>
        {/each}
      </div>

      <div class="meta">
        <span class="timer">{mm}:{ss}</span>
        <span class="rec" class:busy={processing}></span>
      </div>
    </div>
  </div>
</div>

<style>
  .capsule {
    display: grid;
    place-items: center;
    width: 200px;
    height: 48px;
    transform: scale(var(--s, 1));
    transform-origin: center center;
    will-change: transform;
  }

  .glass {
    width: var(--w, 12px);
    height: var(--h, 12px);
    border-radius: 999px;
    border: none;
    overflow: hidden;
    isolation: isolate;
    /* dark frost — works on transparent overlay; blurs stage chrome in dev */
    background: rgba(6, 6, 12, 0.55);
    -webkit-backdrop-filter: blur(16px) saturate(185%) brightness(0.92);
    backdrop-filter: blur(16px) saturate(185%) brightness(0.92);
    box-shadow:
      0 10px 36px rgba(0, 0, 0, 0.4),
      0 2px 8px rgba(0, 0, 0, 0.25),
      inset 0 1px 1px rgba(255, 255, 255, 0.08),
      inset 0 -10px 18px rgba(0, 0, 0, 0.28);
  }

  .capsule.error .glass {
    background: rgba(40, 8, 16, 0.58);
    box-shadow:
      0 10px 36px rgba(127, 29, 29, 0.35),
      0 2px 8px rgba(0, 0, 0, 0.25),
      inset 0 1px 1px rgba(255, 255, 255, 0.08),
      inset 0 -10px 18px rgba(0, 0, 0, 0.28);
  }

  .chrome {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 5px 0 3px;
    opacity: var(--ui, 0);
    transform: translateY(var(--uy, 5px));
    pointer-events: none;
  }

  .mic {
    width: 22px;
    height: 22px;
    flex: 0 0 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.3);
    color: #fff;
    box-shadow:
      inset 0 1px 2px rgba(255, 255, 255, 0.1),
      inset 0 -2px 4px rgba(0, 0, 0, 0.4);
  }

  .mic svg {
    width: 10px;
    height: 10px;
  }

  .wave {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1.8px;
    height: 16px;
    min-width: 0;
  }

  .wave i {
    display: block;
    width: 2px;
    border-radius: 99px;
    height: var(--bh, 4px);
    background: rgba(255, 255, 255, 0.9);
    box-shadow: 0 0 8px rgba(255, 255, 255, 0.35);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-left: 5px;
    height: 14px;
    box-shadow: inset 1px 0 0 rgba(255, 255, 255, 0.12);
  }

  .timer {
    font-family: 'Space Grotesk Variable', 'Segoe UI', sans-serif;
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
    color: rgba(255, 255, 255, 0.92);
  }

  .rec {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 30%, #fecaca, #ef4444 45%, #dc2626);
    box-shadow: 0 0 10px rgba(239, 68, 68, 0.7);
    animation: pulse 1.3s ease-in-out infinite;
  }

  .rec.busy {
    animation-duration: 0.7s;
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
