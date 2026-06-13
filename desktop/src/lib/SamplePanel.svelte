<script lang="ts">
  let { samples = [] } = $props<{ samples: any[] }>();
  let expanded = $state<Record<string, boolean>>({});
</script>

<div class="panel">
  <h3 class="panel-title">📋 文风样例</h3>
  {#if samples.length === 0}
    <p class="empty">暂无文风样例</p>
  {:else}
    {#each samples as s}
      <div class="card" class:expanded={expanded[s.id]}>
        <button class="card-header" onclick={() => expanded[s.id] = !expanded[s.id]}>
          <span class="card-title">{s.title}</span>
          <span class="arrow">{expanded[s.id] ? '▼' : '▶'}</span>
        </button>
        {#if expanded[s.id]}
          <p class="card-content">{s.content}</p>
        {:else}
          <p class="card-preview">{s.content.slice(0, 100)}{s.content.length > 100 ? '...' : ''}</p>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .panel { padding: 8px; }
  .panel-title { font-size: 13px; font-weight: 600; margin-bottom: 8px; color: var(--text-dim); }
  .empty { font-size: 12px; color: var(--text-dim); text-align: center; padding: 20px; }
  .card {
    margin: 4px 0; border-radius: 6px;
    background: var(--bg); border: 1px solid var(--border);
    overflow: hidden; transition: border-color 0.15s;
  }
  .card:hover { border-color: var(--accent-dim); }
  .card.expanded { border-color: var(--accent); }
  .card-header {
    width: 100%; display: flex; align-items: center; justify-content: space-between;
    padding: 8px 10px; background: none; color: var(--text); font-size: 13px; font-weight: 500;
    text-align: left; border: none;
  }
  .card-header:hover { background: rgba(255,255,255,0.03); }
  .arrow { font-size: 10px; color: var(--text-dim); }
  .card-preview { padding: 0 10px 8px; font-size: 12px; color: var(--text-dim); line-height: 1.4; }
  .card-content { padding: 0 10px 8px; font-size: 12px; line-height: 1.5; white-space: pre-wrap; }
</style>
