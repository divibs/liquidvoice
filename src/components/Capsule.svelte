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
    mode: 'listen' | 'process' | 'success' | 'skipped' | 'error';
    onCollapsed?: () => void;
  }>();

  let raw = $state(0);
  let orb = $state(0);
  let exit = $state(1);
  let lvl = $state(0);
  let wt = $state(0);
  let collapsed = false;
  let holdMs = 0;

  const OPEN = 650;
  const CLOSE = 380;
  const TO_ORB = 320;
  const EXIT_MS = 280;
  const CHECK_HOLD = 450;
  const SKIP_HOLD = 180;
  const MAX_W = 168;
  const MAX_H = 34;
  const ORB = 22;
  const N = 10;

  $effect(() => {
    const md = mode;
    if (md === 'listen' && target === 1) {
      orb = 0;
      exit = 1;
      holdMs = 0;
      collapsed = false;
    }
    if (md === 'success' || md === 'skipped') {
      holdMs = 0;
    }
  });

  $effect(() => {
    let raf = 0;
    let last = performance.now();
    const loop = (now: number) => {
      const dt = Math.min(50, now - last);
      last = now;

      const tgt = untrack(() => target);
      const md = untrack(() => mode);

      if (md === 'listen') {
        raw = tgt === 1 ? Math.min(1, raw + dt / OPEN) : Math.max(0, raw - dt / CLOSE);
        orb = Math.max(0, orb - dt / TO_ORB);
        exit = 1;
      } else if (md === 'process' || md === 'success' || md === 'skipped') {
        raw = Math.max(raw, 0.85);
        orb = Math.min(1, orb + dt / TO_ORB);
        if (md === 'success' && orb >= 0.98) {
          holdMs += dt;
          if (holdMs >= CHECK_HOLD) exit = Math.max(0, exit - dt / EXIT_MS);
        } else if (md === 'skipped' && orb >= 0.98) {
          holdMs += dt;
          if (holdMs >= SKIP_HOLD) exit = Math.max(0, exit - dt / EXIT_MS);
        }
      } else if (md === 'error') {
        raw = Math.max(raw, 0.85);
        orb = Math.max(0, orb - dt / TO_ORB);
        exit = 1;
      }

      const lv = untrack(() => level);
      const want = md === 'listen' ? lv : 0.04;
      lvl += (want - lvl) * Math.min(1, dt * 0.012);
      wt += dt * 0.0035;

      const listenGone = md === 'listen' && tgt === 0 && raw <= 0;
      const orbGone =
        (md === 'success' || md === 'skipped') && exit <= 0.02 && orb >= 0.9;

      if ((listenGone || orbGone) && !collapsed) {
        collapsed = true;
        untrack(() => onCollapsed)();
      } else if (tgt === 1 && md === 'listen') {
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

  const pillW0 = $derived(lerp(12, MAX_W, stretch) * appear);
  const pillH0 = $derived(
    Math.max(12 * appear, lerp(12, MAX_H, refine) * appear) * (1 - pinch * 0.12),
  );

  const o = $derived(smooth(0, 1, orb));
  const pillW = $derived(lerp(pillW0, ORB, o));
  const pillH = $derived(lerp(pillH0, ORB, o));
  const uiOp = $derived(smooth(0.38, 0.72, raw) * (1 - o));
  const uiY = $derived(lerp(6, 0, smooth(0.4, 0.78, raw)));
  const openScale = $derived(
    appear *
      (1 +
        springKick(raw) * 0.22 * appear +
        Math.sin(settle * Math.PI) * 0.045 * (1 - settle)),
  );
  const scale = $derived(Math.max(0.001, openScale * exit));

  const mm = $derived(String(Math.floor(elapsed / 60)).padStart(2, '0'));
  const ss = $derived(String(elapsed % 60).padStart(2, '0'));

  const err = $derived(mode === 'error');
  const processing = $derived(mode === 'process');
  const success = $derived(mode === 'success');
  const showOrbUi = $derived(o > 0.55);

  function barH(i: number) {
    const taper = Math.sin((Math.PI * i) / (N - 1));
    const wave = 0.5 + 0.5 * Math.sin(wt + i * 0.55);
    // Dead-zone so ambient hiss stays flat; only real speech drives motion.
    const energy = Math.max(0, (lvl - 0.04) / 0.96);
    const motion = taper * energy * 14 * wave;
    return Math.max(2, (2.2 + motion) * uiOp);
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
    <div class="frost" aria-hidden="true"></div>
    <div class="grain" aria-hidden="true"></div>

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

    {#if showOrbUi}
      <div class="orb-ui" aria-hidden="true">
        {#if success}
          <svg class="check" viewBox="0 0 24 24" fill="none">
            <path
              d="M5 12.5l4.5 4.5L19 7.5"
              stroke="currentColor"
              stroke-width="2.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        {:else if processing}
          <span class="spin"></span>
        {/if}
      </div>
    {/if}
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
    position: relative;
    width: var(--w, 12px);
    height: var(--h, 12px);
    border-radius: 999px;
    border: none;
    overflow: hidden;
    box-shadow:
      0 12px 40px rgba(0, 0, 0, 0.45),
      0 2px 8px rgba(0, 0, 0, 0.3);
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
        rgba(0, 0, 0, 0.2) 100%
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
    opacity: 0.4;
    mix-blend-mode: overlay;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 180 180' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 120px 120px;
  }

  .capsule.error .frost {
    background:
      linear-gradient(
        165deg,
        rgba(255, 200, 200, 0.08) 0%,
        rgba(255, 255, 255, 0.02) 38%,
        rgba(40, 0, 0, 0.25) 100%
      ),
      rgba(42, 10, 16, 0.66);
    box-shadow:
      inset 0 1px 1px rgba(255, 255, 255, 0.1),
      inset 0 -12px 22px rgba(80, 0, 0, 0.35);
  }

  .capsule.error .glass {
    box-shadow:
      0 12px 40px rgba(127, 29, 29, 0.4),
      0 2px 8px rgba(0, 0, 0, 0.3);
  }

  .chrome {
    position: relative;
    z-index: 2;
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

  .orb-ui {
    position: absolute;
    inset: 0;
    z-index: 3;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.95);
    pointer-events: none;
  }

  .spin {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.22);
    border-top-color: rgba(255, 255, 255, 0.95);
    animation: spin 0.7s linear infinite;
  }

  .check {
    width: 12px;
    height: 12px;
    color: #86efac;
    filter: drop-shadow(0 0 6px rgba(134, 239, 172, 0.45));
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

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
