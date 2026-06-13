<script lang="ts">
  let { setting = null } = $props<{ setting: any }>();

  const typeNames: Record<number, string> = { 0: '都市', 1: '玄幻', 2: '历史', 3: '奇幻', 4: '武侠', 5: '科幻' };
</script>

<div class="panel">
  <h3 class="panel-title">⚙ 小说设定</h3>
  {#if !setting}
    <p class="empty">暂无设定数据</p>
  {:else}
    <div class="card">
      <div class="field">
        <span class="label">标题</span>
        <span class="value">{setting.title || '—'}</span>
      </div>
      <div class="field">
        <span class="label">类型</span>
        <span class="badge">{typeNames[setting.novel_type] ?? '—'}</span>
      </div>
      {#if setting.tags?.length}
        <div class="field">
          <span class="label">标签</span>
          <div class="tags">
            {#each setting.tags as tag}
              <span class="tag">{tag}</span>
            {/each}
          </div>
        </div>
      {/if}
      <div class="field">
        <span class="label">灵感</span>
        <span class="value">{setting.inspiration || '—'}</span>
      </div>
      <div class="field">
        <span class="label">简介</span>
        <p class="desc">{setting.description || '—'}</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .panel { padding: 8px; }
  .panel-title { font-size: 13px; font-weight: 600; margin-bottom: 8px; color: var(--text-dim); }
  .empty { font-size: 12px; color: var(--text-dim); text-align: center; padding: 20px; }
  .card {
    padding: 12px;
    border-radius: 6px;
    background: var(--bg);
    border: 1px solid var(--border);
  }
  .field { margin: 8px 0; }
  .label { display: block; font-size: 11px; color: var(--text-dim); margin-bottom: 2px; }
  .value { font-size: 13px; }
  .badge {
    font-size: 12px; padding: 2px 8px; border-radius: 3px;
    background: var(--bg-panel); color: var(--accent);
  }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; }
  .tag {
    font-size: 11px; padding: 2px 6px; border-radius: 3px;
    background: var(--bg-panel); color: var(--text-dim);
  }
  .desc { font-size: 12px; line-height: 1.5; color: var(--text); white-space: pre-wrap; }
</style>
