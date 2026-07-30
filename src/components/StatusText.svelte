<script lang="ts">
  let { state = '', errorMsg = '' } = $props();

  function label(): string {
    switch (state) {
      case 'listening': return 'Listening';
      case 'processing': return 'Transcribing';
      case 'done': return 'Done';
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
    margin-top: 7px;
    font-family: 'Space Grotesk Variable', 'Segoe UI', sans-serif;
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: rgba(165, 243, 252, 0.65);
    text-align: center;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status.error {
    color: #fda4af;
    letter-spacing: 0.08em;
    text-transform: none;
    font-size: 10px;
  }
</style>
