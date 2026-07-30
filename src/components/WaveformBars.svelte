<script lang="ts">
  let { level = 0 } = $props();

  const NUM_BARS = 7;
  let phases = $state(Array.from({ length: NUM_BARS }, (_, i) => i * 0.8));
  let t = $state(0);
  let raf: number;

  function animate() {
    t += 0.08;
    raf = requestAnimationFrame(animate);
  }

  $effect(() => {
    raf = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(raf);
  });

  function barHeight(i: number): number {
    const wave = Math.sin(t + phases[i]) * 0.5 + 0.5;
    return 4 + wave * level * 36;
  }
</script>

<div class="bars">
  {#each Array(NUM_BARS) as _, i}
    <div
      class="bar"
      style="height: {barHeight(i)}px;"
    ></div>
  {/each}
</div>

<style>
  .bars {
    display: flex;
    align-items: center;
    gap: 5px;
    z-index: 1;
  }

  .bar {
    width: 4px;
    border-radius: 2px;
    background: linear-gradient(180deg, #6366f1, #a855f7);
    transition: height 0.05s ease-out;
    min-height: 4px;
  }
</style>
