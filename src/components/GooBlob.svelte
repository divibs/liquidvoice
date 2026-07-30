<script lang="ts">
  let { level = 0, active = false } = $props();

  let t = $state(0);
  let raf: number;

  function animate() {
    t += 0.03;
    raf = requestAnimationFrame(animate);
  }

  $effect(() => {
    if (active) {
      raf = requestAnimationFrame(animate);
      return () => cancelAnimationFrame(raf);
    }
  });

  const NUM_CIRCLES = 5;

  function cx(i: number): number {
    const base = 140 + (i - 2) * 32;
    const wobble = Math.sin(t * 2 + i * 1.3) * (4 + level * 30);
    return base + wobble;
  }

  function cy(i: number): number {
    const base = 36;
    const wobble = Math.cos(t * 1.7 + i * 0.9) * (3 + level * 20);
    return base + wobble;
  }

  function r(i: number): number {
    const base = 18 + (i % 2) * 6;
    return base + level * 25;
  }
</script>

<svg width="280" height="72" viewBox="0 0 280 72" class="goo-svg">
  <defs>
    <filter id="goo">
      <feGaussianBlur in="SourceGraphic" stdDeviation="6" result="blur" />
      <feColorMatrix
        in="blur"
        values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -7"
        result="goo"
      />
      <feComposite in="SourceGraphic" in2="goo" operator="atop" />
    </filter>
    <linearGradient id="blobGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="rgba(30,30,30,0.9)" />
      <stop offset="100%" stop-color="rgba(45,45,50,0.85)" />
    </linearGradient>
  </defs>

  <g filter="url(#goo)">
    {#each Array(NUM_CIRCLES) as _, i}
      <circle
        cx={cx(i)}
        cy={cy(i)}
        r={r(i)}
        fill="url(#blobGrad)"
      />
    {/each}
  </g>

  <rect
    x="1" y="1" width="278" height="70" rx="35"
    fill="none"
    stroke="rgba(255,255,255,0.08)"
    stroke-width="1"
  />
</svg>

<style>
  .goo-svg {
    position: absolute;
    inset: 0;
  }
</style>
