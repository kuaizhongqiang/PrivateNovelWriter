<script lang="ts">
  let { stats, characters = [], setting = null } = $props<{
    stats: any; characters: any[]; setting: any;
  }>();

  const typeNames: Record<number, string> = { 0: '都市', 1: '玄幻', 2: '历史', 3: '奇幻', 4: '武侠', 5: '科幻' };
  let previewChars = $derived(characters.slice(0, 5));
</script>

<div class="dashboard">
  <h2 class="title">项目总览</h2>

  <div class="grid">
    <!-- Left: Project Info -->
    <div class="card">
      <h3>📖 项目信息</h3>
      {#if setting}
        <p class="novel-name">{setting.title || '—'}</p>
        <span class="badge">{typeNames[setting.novel_type] ?? '—'}</span>
        {#if setting.tags?.length}
          <div class="tags">{#each setting.tags as t}<span class="tag">{t}</span>{/each}</div>
        {/if}
        <p class="desc">{setting.description?.slice(0, 150)}{setting.description?.length > 150 ? '...' : ''}</p>
      {:else}
        <p class="dim">暂无设定 — 使用 CLI <code>pnw setting update</code> 设置</p>
      {/if}
    </div>

    <!-- Middle: Characters Preview -->
    <div class="card">
      <h3>👤 角色 ({characters.length})</h3>
      {#if previewChars.length > 0}
        {#each previewChars as c}
          <div class="char-row">
            <span class="char-name">{c.name}</span>
            <span class="char-type">{['男主','女主','其他'][c.char_type] ?? ''}</span>
          </div>
        {/each}
        {#if characters.length > 5}<p class="more">+{characters.length - 5} 更多...</p>{/if}
      {:else}
        <p class="dim">暂无角色</p>
      {/if}
    </div>

    <!-- Right: Progress -->
    <div class="card">
      <h3>📊 写作进度</h3>
      {#if stats}
        <div class="progress-ring-wrap">
          <svg viewBox="0 0 120 120" class="progress-ring">
            <circle cx="60" cy="60" r="50" fill="none" stroke="var(--border)" stroke-width="8"/>
            <circle cx="60" cy="60" r="50" fill="none" stroke="var(--accent)" stroke-width="8"
              stroke-dasharray="314" stroke-dashoffset={314 - 314 * Math.min(stats.total_written / Math.max(stats.total_char_target, 1), 1)}
              transform="rotate(-90 60 60)" stroke-linecap="round"/>
          </svg>
          <div class="ring-text">
            <span class="ring-pct">{stats.total_char_target > 0 ? (stats.total_written / stats.total_char_target * 100).toFixed(0) : '—'}%</span>
            <span class="ring-label">已完成</span>
          </div>
        </div>
        <div class="progress-stats">
          <div class="pstat"><span class="pval">{stats.total_written}</span><span class="plabel">已写字数</span></div>
          <div class="pstat"><span class="pval">{stats.written_chapters}/{stats.planned_chapters}</span><span class="plabel">章节</span></div>
        </div>
      {:else}
        <p class="dim">暂无写作数据</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .dashboard { padding: 32px; overflow-y: auto; height: 100%; }
  .title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; }
  .card {
    padding: 20px; border-radius: 8px; background: var(--bg-secondary);
    border: 1px solid var(--border);
  }
  .card h3 { font-size: 14px; font-weight: 600; margin-bottom: 12px; color: var(--text-dim); }
  .novel-name { font-size: 16px; font-weight: 700; margin-bottom: 8px; }
  .badge { font-size: 12px; padding: 2px 8px; border-radius: 3px; background: var(--bg-panel); color: var(--accent); }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 8px; }
  .tag { font-size: 11px; padding: 2px 6px; border-radius: 3px; background: var(--bg-panel); color: var(--text-dim); }
  .desc { font-size: 13px; line-height: 1.5; margin-top: 8px; color: var(--text); }
  .dim { font-size: 12px; color: var(--text-dim); }
  .dim code { font-size: 11px; background: var(--bg); padding: 1px 4px; border-radius: 2px; }
  .char-row { display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .char-row:last-child { border-bottom: none; }
  .char-type { font-size: 11px; color: var(--text-dim); }
  .more { font-size: 12px; color: var(--accent); margin-top: 8px; }
  .progress-ring-wrap { position: relative; width: 120px; height: 120px; margin: 0 auto 16px; }
  .progress-ring { width: 120px; height: 120px; }
  .ring-text { position: absolute; top: 0; left: 0; width: 100%; height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .ring-pct { font-size: 22px; font-weight: 700; }
  .ring-label { font-size: 11px; color: var(--text-dim); }
  .progress-stats { display: flex; justify-content: space-around; }
  .pstat { text-align: center; }
  .pval { display: block; font-size: 16px; font-weight: 600; }
  .plabel { font-size: 11px; color: var(--text-dim); }
</style>
