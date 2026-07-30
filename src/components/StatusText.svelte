<script lang="ts">
  let { state = '', errorMsg = '' } = $props();

  function label(): string {
    switch (state) {
      case 'listening': return 'Listening…';
      case 'processing': return 'Transcribing…';
      case 'error': return errorMsg || 'Error';
      default: return '';
    }
  }
</script>

{#if label()}
  <p class="status" class:error={state === 'error'}>{label()}</p>
{/if}

<style>
  .status {
    margin-top: 6px;
    font-family: 'Segoe UI', system-ui, sans-serif;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.6);
    text-align: center;
  }

  .status.error {
    color: #f87171;
  }
</style>
