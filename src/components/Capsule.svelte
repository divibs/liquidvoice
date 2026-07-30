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

  let p = $state(0); // morph progress 0..1
  let t = $state(0); // animation clock
  let notified = false;

  // single rAF loop: tween morph + advance clock (reads via untrack so it runs once)
  $effect(() => {
    let raf = 0;
    const loop = () => {
      const tgt = untrack(() => target);
      p += (tgt - p) * 0.2;
      if (Math.abs(tgt - p) < 0.008) {
        p = tgt;
        if (tgt === 0 && !notified) {
          notified = true;
          untrack(() => onCollapsed)();
        }
      } else {
        notified = false;
      }
      t += 0.05;
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });

  const lerp = (a: number, b: number, x: number) => a + (b - a) * x;
  const clamp01 = (x: number) => (x < 0 ? 0 : x > 1 ? 1 : x);
  const smooth = (e0: number, e1: number, x: number) => {
    const u = clamp01((x - e0) / (e1 - e0));
    return u * u * (3 - 2 * u);
  };

  // geometry from morph progress
  const leftCx = $derived(lerp(210, 44, p));
  const rightCx = $derived(lerp(210, 384, p));
  const capR = $derived(lerp(7, 30, p));
  const rectGrow = $derived(smooth(0.2, 0.8, p));
  const rectH = $derived(lerp(0, 52, rectGrow));
  const rectW = $derived(Math.max(0, rightCx - leftCx));

  const co = $derived(smooth(0.3, 0.92, p)); // content opacity
  const cs = $derived(lerp(0.82, 1, smooth(0.4, 1, p))); // content settle scale

  const effLevel = $derived(mode === 'process' ? 0.06 : level);
  const rim = $derived(mode === 'error' ? '#fb7185' : '#c4a7ff');
  const glow = $derived(mode === 'error' ? 'rgba(244,63,94,0.55)' : 'rgba(139,92,246,0.6)');
  const glowOp = $derived(
    clamp01((0.22 + effLevel * 0.5) * co + 0.05 * Math.sin(t * 1.3) + 0.05),
  );

  const mm = $derived(String(Math.floor(elapsed / 60)).padStart(2, '0'));
  const ss = $derived(String(elapsed % 60).padStart(2, '0'));

  // tapered waveform
  const N = 23;
  const x0 = 92;
  const span = 208;
  function bar(i: number) {
    const e = Math.sin((Math.PI * i) / (N - 1)); // 0 at ends → dots, 1 mid
    const live = 0.45 + 0.55 * Math.abs(Math.sin(t * 3 + i * 0.6));
    const h = 2 + e * (4 + effLevel * 30) * live;
    const x = x0 + (span * i) / (N - 1);
    return { x, h };
  }
</script>

<svg class="capsule" viewBox="0 0 420 64" width="420" height="64">
  <defs>
    <linearGradient id="glass" gradientUnits="userSpaceOnUse" x1="0" y1="2" x2="0" y2="62">
      <stop offset="0%" stop-color="rgba(150,120,210,0.34)" />
      <stop offset="16%" stop-color="rgba(46,32,72,0.55)" />
      <stop offset="100%" stop-color="rgba(15,11,26,0.7)" />
    </linearGradient>
    <radialGradient id="dotG" cx="35%" cy="30%" r="75%">
      <stop offset="0%" stop-color="#d8b4fe" />
      <stop offset="100%" stop-color="#7c3aed" />
    </radialGradient>
    <linearGradient id="barG" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#e9d5ff" />
      <stop offset="100%" stop-color="#8b5cf6" />
    </linearGradient>

    <filter id="glassF" x="-20%" y="-55%" width="140%" height="210%">
      <feGaussianBlur in="SourceGraphic" stdDeviation="6" result="b" />
      <feColorMatrix in="b" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 22 -9" result="goo" />
      <feMorphology in="goo" operator="dilate" radius="1.1" result="dil" />
      <feComposite in="dil" in2="goo" operator="arithmetic" k1="0" k2="1" k3="-1" k4="0" result="ring" />
      <feFlood flood-color={rim} result="rimc" />
      <feComposite in="rimc" in2="ring" operator="in" result="rimcol" />
      <feGaussianBlur in="rimcol" stdDeviation="0.4" result="rimsoft" />
      <feMerge>
        <feMergeNode in="rimsoft" />
        <feMergeNode in="goo" />
      </feMerge>
    </filter>

    <filter id="glowF" x="-60%" y="-120%" width="220%" height="340%">
      <feGaussianBlur stdDeviation="11" />
    </filter>
  </defs>

  <!-- breathing underglow -->
  <ellipse
    cx="210"
    cy="40"
    rx={lerp(16, 172, p)}
    ry={lerp(5, 24, p)}
    fill={glow}
    opacity={glowOp}
    filter="url(#glowF)"
  />

  <!-- liquid glass body (dot -> peanut -> pill) -->
  <g filter="url(#glassF)">
    <circle cx={leftCx} cy={32} r={capR} fill="url(#glass)" />
    <rect x={leftCx} y={32 - rectH / 2} width={rectW} height={rectH} rx={rectH / 2} fill="url(#glass)" />
    <circle cx={rightCx} cy={32} r={capR} fill="url(#glass)" />
  </g>

  <!-- contents bloom in as the pill settles -->
  <g opacity={co} transform="translate(210 32) scale({cs}) translate(-210 -32)">
    <!-- mic well -->
    <circle cx={leftCx} cy={32} r={20} fill="rgba(10,7,18,0.45)" stroke={rim} stroke-opacity="0.4" stroke-width="1" />
    <g transform="translate({leftCx - 9} 23)" stroke="#d8b4fe" fill="none" stroke-width="1.6" stroke-linecap="round">
      <rect x="6.5" y="2" width="5" height="9" rx="2.5" fill="#d8b4fe" stroke="none" />
      <path d="M4.5 8.5 a4.5 4.5 0 0 0 9 0" />
      <line x1="9" y1="13" x2="9" y2="16" />
      <line x1="6" y1="16" x2="12" y2="16" />
    </g>

    <!-- tapered waveform -->
    {#each Array(N) as _, i}
      {@const b = bar(i)}
      <rect x={b.x - 1.5} y={32 - b.h / 2} width="3" height={b.h} rx="1.5" fill="url(#barG)" />
    {/each}

    <!-- divider + timer -->
    <line x1="312" y1="20" x2="312" y2="44" stroke={rim} stroke-opacity="0.3" stroke-width="1" />
    <text
      x="324"
      y="37"
      fill="rgba(233,224,255,0.92)"
      font-family="'Space Grotesk Variable', sans-serif"
      font-size="13"
      letter-spacing="0.5"
    >{mm}:{ss}</text>

    <!-- status dot -->
    <circle cx="392" cy="32" r={mode === 'process' ? 6 + Math.sin(t * 6) * 1.4 : 6} fill="url(#dotG)">
      {#if mode === 'process'}
        <animate attributeName="opacity" values="1;0.4;1" dur="0.9s" repeatCount="indefinite" />
      {/if}
    </circle>
  </g>
</svg>

<style>
  .capsule {
    display: block;
    overflow: visible;
  }
</style>
