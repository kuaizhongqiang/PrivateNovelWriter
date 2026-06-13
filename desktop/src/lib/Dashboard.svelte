<script lang="ts">
  let { stats, characters = [], setting = null } = $props<{
    stats: any; characters: any[]; setting: any;
  }>();

  const typeNames: Record<number, string> = { 0: '都市', 1: '玄幻', 2: '历史', 3: '奇幻', 4: '武侠', 5: '科幻' };
  let previewChars = $derived(characters.slice(0, 5));
  let pct = $derived(stats?.total_char_target > 0 ? Math.min(stats.total_written / stats.total_char_target * 100, 100) : 0);
</script>

<div class="dashboard">
  <h2 class="title">📊 项目总览</h2>

  <div class="grid">
    <!-- Project Info -->
    <div class="card">
      <h3>📖 项目</h3>
      {#if setting}
        <p class="novel-name">{setting.title || '—'}</p>
        <span class="badge">{typeNames[setting.novel_type] ?? '—'}</span>
        {#if setting.tags?.length}
          <div class="tags">{#each setting.tags as t}<span class="tag">{t}</span>{/each}</div>
        {/if}
        <p class="desc">{setting.description?.slice(0, 150)}{setting.description?.length > 150 ? '...' : ''}</p>
      {:else}
        <p class="dim">暂无设定</p>
      {/if}
    </div>

    <!-- Characters -->
    <div class="card">
      <h3>👤 角色 ({characters.length})</h3>
      {#if previewChars.length > 0}
        {#each previewChars as c}
          <div class="char-row">
            <span class="char-name">{c.name}</span>
            <span class="char-type">{['男主','女主','其他'][c.char_type] ?? ''}</span>
            <span class="char-rel">{c.relationship}</span>
          </div>
        {/each}
        {#if characters.length > 5}<p class="more">+{characters.length - 5} 更多...</p>{/if}
      {:else}
        <p class="dim">暂无角色</p>
      {/if}
    </div>

    <!-- Progress with ring + bars -->
    <div class="card">
      <h3>📈 进度</h3>
      {#if stats}
        <div class="ring-wrap">
          <svg viewBox="0 0 120 120" class="ring">
            <circle cx="60" cy="60" r="50" fill="none" stroke="var(--border)" stroke-width="8"/>
            <circle cx="60" cy="60" r="50" fill="none" stroke="var(--accent)" stroke-width="8"
              stroke-dasharray="314" stroke-dashoffset={314 - 314 * pct / 100}
              transform="rotate(-90 60 60)" stroke-linecap="round"/>
          </svg>
          <div class="ring-text">
            <span class="ring-pct">{pct.toFixed(0)}%</span>
            <span class="ring-label">完成</span>
          </div>
        </div>

        <div class="progress-list">
          <div class="prow"><span>已写字数</span><span class="pv">{stats.total_written}</span></div>
          <div class="prow"><span>目标字数</span><span class="pv">{stats.total_char_target}</span></div>
          <div class="prow"><span>章节进度</span><span class="pv">{stats.written_chapters}/{stats.planned_chapters}</span></div>
          <div class="prow"><span>卷数</span><span class="pv">{stats.phases}</span></div>
        </div>

        <!-- Mini bar -->
        <div class="mini-bar-track">
          <div class="mini-bar-fill" style="width: {pct}%"></div>
        </div>
      {:else}
        <p class="dim">暂无数据</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .dashboard { padding: 32px; overflow-y: auto; height: 100%; }
  .title { font-size: 22px; font-weight: 700; margin-bottom: 24px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; }
  .card { padding: 20px; border-radius: 8px; background: var(--bg-secondary); border: 1px solid var(--border); }

  .card h3 { font-size: 13px; font-weight: 600; margin-bottom: 12px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
  .novel-name { font-size: 18px; font-weight: 700; margin-bottom: 8px; }
  .badge { font-size: 11px; padding: 2px 7px; border-radius: 3px; background: var(--bg-panel); color: var(--accent); }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 8px; }
  .tag { font-size: 11px; padding: 2px 6px; border-radius: 3px; background: var(--bg-panel); color: var(--text-dim); }
  .desc { font-size: 13px; line-height: 1.5; margin-top: 8px; color: var(--text); }
  .dim { font-size: 12px; color: var(--text-dim); }

  .char-row { display: flex; gap: 8px; padding: 5px 0; border-bottom: 1px solid var(--border); font-size: 13px; align-items: center; }
  .char-row:last-child { border-bottom: none; }
  .char-name { font-weight: 500; }
  .char-type { font-size: 11px; color: var(--text-dim); background: var(--bg); padding: 0 4px; border-radius: 2px; }
  .char-rel { font-size: 11px; color: var(--text-muted); margin-left: auto; }
  .more { font-size: 12px; color: var(--accent); margin-top: 8px; }

  .ring-wrap { position: relative; width: 100px; height: 100px; margin: 0 auto 12px; }
  .ring { width: 100px; height: 100px; }
  .ring-text { position: absolute; top: 0; left: 0; width: 100%; height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .ring-pct { font-size: 18px; font-weight: 700; }
  .ring-label { font-size: 10px; color: var(--text-dim); }

  .progress-list { display: flex; flex-direction: column; gap: 4px; margin: 12px 0; }
  .prow { display: flex; justify-content: space-between; font-size: 12px; padding: 2px 0; }
  .pv { font-weight: 600; }

  .mini-bar-track { height: 4px; background: var(--border); border-radius: 2px; overflow: hidden; }
  .mini-bar-fill { height: 100%; background: var(--accent); border-radius: 2px; transition: width 0.5s ease; }
</style>
