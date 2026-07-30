<script lang="ts">
  let { level = 0, active = false } = $props();

  let t = $state(0);
  let raf: number;

  function animate() {
    t += 0.025;
    raf = requestAnimationFrame(animate);
  }

  $effect(() => {
    if (active) {
      raf = requestAnimationFrame(animate);
      return () => cancelAnimationFrame(raf);
    }
  });

  const BALLS = [
    { base: 40, speed: 1.1, phase: 0, r: 13 },
    { base: 95, speed: 0.8, phase: 2.1, r: 16 },
    { base: 150, speed: 1.4, phase: 4.2, r: 14 },
    { base: 200, speed: 0.9, phase: 1.2, r: 12 },
  ];

  function cx(b: typeof BALLS[0]): number {
    return b.base + Math.sin(t * b.speed + b.phase) * (10 + level * 22);
  }

  function cy(b: typeof BALLS[0]): number {
    return 22 + Math.cos(t * b.speed * 0.8 + b.phase) * (4 + level * 10);
  }
</script>

<svg class="metaballs" width="240" height="44" viewBox="0 0 240 44">
  <defs>
    <filter id="goo">
      <feGaussianBlur in="SourceGraphic" stdDeviation="5" result="blur" />
      <feColorMatrix in="blur"
        values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 16 -6"
        result="goo" />
    </filter>
  </defs>
  <g filter="url(#goo)">
    {#each BALLS as b}
      <circle cx={cx(b)} cy={cy(b)} r={b.r + level * 8} fill="rgba(45, 212, 191, 0.16)" />
    {/each}
  </g>
</svg>

<style>
  .metaballs {
    position: absolute;
    inset: 4px 0;
  }
</style>
