<script lang="ts">
  let { level = 0 } = $props();

  const NUM_BARS = 9;
  const phases = Array.from({ length: NUM_BARS }, (_, i) => i * 0.7);
  let t = $state(0);
  let raf: number;

  function animate() {
    t += 0.09;
    raf = requestAnimationFrame(animate);
  }

  $effect(() => {
    raf = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(raf);
  });

  function barHeight(i: number): number {
    const wave = Math.sin(t + phases[i]) * 0.5 + 0.5;
    return 4 + wave * level * 30;
  }
</script>

<div class="bars">
  {#each Array(NUM_BARS) as _, i}
    <div class="bar" style="height: {barHeight(i)}px;"></div>
  {/each}
</div>

<style>
  .bars {
    display: flex;
    align-items: center;
    gap: 4px;
    z-index: 1;
  }

  .bar {
    width: 3px;
    min-height: 4px;
    border-radius: 2px;
    background: linear-gradient(180deg, #67e8f9, #2dd4bf);
    box-shadow: 0 0 6px rgba(45, 212, 191, 0.5);
    transition: height 0.06s ease-out;
  }
</style>
