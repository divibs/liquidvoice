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

  // raw = linear morph position 0..1 (drives a single timeline, plays forward & back)
  let raw = $state(0);
  let lvl = $state(0); // mic level, smoothed
  let wt = $state(0); // slow waveform clock
  let pt = $state(0); // pulse clock (status dot / glow breathe)
  let collapsed = false;

  const OPEN = 560; // ms dot -> capsule
  const CLOSE = 440; // ms capsule -> dot

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

      wt += dt * 0.0022; // gentle traveling wave
      pt += dt * 0.004;

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

  // staged sub-eases over the single timeline
  const sep = smooth(0.14, 0.62, raw); // caps pull apart (the stretch)
  const grow = smooth(0.0, 0.5, raw); // dot swells from the very first frame
  const neck = smooth(0.12, 0.96, raw); // liquid neck fills -> true capsule at 1

  const CX = 210;
  const capR = lerp(8, 26, grow);
  const half = lerp(0, 170, sep);
  const leftCx = CX - half;
  const rightCx = CX + half;
  const rectH = lerp(0, 52, neck); // == 2*capR at rest => perfect capsule, no peanut
  const rectW = Math.max(0, rightCx - leftCx);

  const co = smooth(0.34, 0.95, raw); // inner elements bloom in as glass forms

  // damped settle wobble on the last beat (and reversed on close)
  const sw = smooth(0.62, 1.0, raw);
  const groupScale = 1 + Math.sin(sw * Math.PI) * 0.06 * (1 - sw);

  const rim = $derived(mode === 'error' ? '#fb7185' : '#c4a7ff');
  const glowCol = $derived(mode === 'error' ? 'rgba(244,63,94,0.55)' : 'rgba(139,92,246,0.6)');
  const glowOp = $derived(
    clamp01((0.2 + lvl * 0.5) * co + 0.04 * Math.sin(pt * 1.3) + 0.05),
  );

  const mm = $derived(String(Math.floor(elapsed / 60)).padStart(2, '0'));
  const ss = $derived(String(elapsed % 60).padStart(2, '0'));

  // tapered spectrum: dots at the ends, tall in the middle, slow traveling motion
  const N = 23;
  const X0 = 82;
  const SPAN = 186;
  function bar(i: number) {
    const taper = Math.sin((Math.PI * i) / (N - 1));
    const wave = 0.55 + 0.45 * Math.sin(wt + i * 0.55);
    const h = 2 + taper * (3 + lvl * 26) * (0.6 + 0.4 * wave);
    return { x: X0 + (SPAN * i) / (N - 1), h };
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

    <!-- goo merge + extracted liquid rim -->
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
    rx={lerp(16, 176, raw)}
    ry={lerp(5, 24, raw)}
    fill={glowCol}
    opacity={glowOp}
    filter="url(#glowF)"
  />

  <!-- whole body settles as one liquid mass -->
  <g transform="translate(210 32) scale({groupScale}) translate(-210 -32)">
    <g filter="url(#glassF)">
      <circle cx={leftCx} cy={32} r={capR} fill="url(#glass)" />
      <rect x={leftCx} y={32 - rectH / 2} width={rectW} height={rectH} rx={rectH / 2} fill="url(#glass)" />
      <circle cx={rightCx} cy={32} r={capR} fill="url(#glass)" />
    </g>

    <!-- contents bloom in once the glass has a shape -->
    <g opacity={co}>
      <circle cx={leftCx} cy={32} r={20} fill="rgba(10,7,18,0.45)" stroke={rim} stroke-opacity="0.4" stroke-width="1" />
      <g transform="translate({leftCx - 9} 23)" stroke="#d8b4fe" fill="none" stroke-width="1.6" stroke-linecap="round">
        <rect x="6.5" y="2" width="5" height="9" rx="2.5" fill="#d8b4fe" stroke="none" />
        <path d="M4.5 8.5 a4.5 4.5 0 0 0 9 0" />
        <line x1="9" y1="13" x2="9" y2="16" />
        <line x1="6" y1="16" x2="12" y2="16" />
      </g>

      {#each Array(N) as _, i}
        {@const b = bar(i)}
        <rect x={b.x - 1.5} y={32 - b.h / 2} width="3" height={b.h} rx="1.5" fill="url(#barG)" />
      {/each}

      <line x1="288" y1="20" x2="288" y2="44" stroke={rim} stroke-opacity="0.3" stroke-width="1" />
      <text
        x="300"
        y="37"
        fill="rgba(233,224,255,0.92)"
        font-family="'Space Grotesk Variable', sans-serif"
        font-size="13"
        letter-spacing="0.5"
      >{mm}:{ss}</text>

      <circle cx={rightCx} cy={32} r={mode === 'process' ? 6 + Math.sin(pt * 6) * 1.4 : 6} fill="url(#dotG)">
        {#if mode === 'process'}
          <animate attributeName="opacity" values="1;0.4;1" dur="0.9s" repeatCount="indefinite" />
        {/if}
      </circle>
    </g>
  </g>
</svg>

<style>
  .capsule {
    display: block;
    overflow: visible;
  }
</style>
