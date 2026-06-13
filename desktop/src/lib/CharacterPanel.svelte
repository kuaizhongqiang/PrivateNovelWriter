<script lang="ts">
  let { characters = [] } = $props<{ characters: any[] }>();

  let actor = $derived(characters.filter((c: any) => c.char_type === 0));
  let actresses = $derived(characters.filter((c: any) => c.char_type === 1));
  let others = $derived(characters.filter((c: any) => c.char_type === 2));
</script>

<div class="panel">
  <h3 class="panel-title">👤 角色</h3>
  {#if characters.length === 0}
    <p class="empty">暂无角色数据</p>
  {:else}
    {#if actor.length > 0}
      <h4 class="group-label">男主</h4>
      {#each actor as c}
        <div class="card card-actor">
          <span class="name">{c.name}</span>
          <span class="meta">{c.age > 0 ? `${c.age}岁` : ''} · {c.relationship}</span>
        </div>
      {/each}
    {/if}
    {#if actresses.length > 0}
      <h4 class="group-label">女主</h4>
      {#each actresses as c}
        <div class="card card-actress">
          <span class="name">{c.name}</span>
          <span class="meta">{c.age > 0 ? `${c.age}岁` : ''} · {c.relationship}</span>
        </div>
      {/each}
    {/if}
    {#if others.length > 0}
      <h4 class="group-label">其他</h4>
      {#each others as c}
        <div class="card card-other">
          <span class="name">{c.name}</span>
          <span class="meta">{c.age > 0 ? `${c.age}岁` : ''} · {c.relationship}</span>
        </div>
      {/each}
    {/if}
  {/if}
</div>

<style>
  .panel { padding: 8px; }
  .panel-title { font-size: 13px; font-weight: 600; margin-bottom: 8px; color: var(--text-dim); }
  .empty { font-size: 12px; color: var(--text-dim); text-align: center; padding: 20px; }
  .group-label { font-size: 11px; color: var(--text-dim); margin: 8px 4px 4px; text-transform: uppercase; letter-spacing: 0.5px; }
  .card {
    padding: 8px 10px;
    margin: 4px 0;
    border-radius: 6px;
    background: var(--bg);
    border: 1px solid var(--border);
    transition: border-color 0.15s;
  }
  .card:hover { border-color: var(--accent-dim); }
  .card-actor { border-left: 3px solid #3b82f6; }
  .card-actress { border-left: 3px solid #ec4899; }
  .card-other { border-left: 3px solid #6b7280; }
  .name { display: block; font-size: 13px; font-weight: 600; }
  .meta { font-size: 11px; color: var(--text-dim); margin-top: 2px; }
</style>
