<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let { outline, onselect, currentChapterId, onrefresh } = $props<{
    outline: any; onselect: (id: string) => void;
    currentChapterId: string | null; onrefresh: () => void;
  }>();

  let expanded = $state<Record<string, boolean>>({});
  let adding = $state<string | null>(null); // phase id or 'phase' for new phase
  let newName = $state('');

  async function addPhase() {
    if (!newName.trim()) return;
    try {
      // 需要 novel_id，从第一个 phase 获取
      await invoke('add_outline_phase', { name: newName.trim() });
      newName = ''; adding = null;
      onrefresh();
    } catch (e) { console.error(e); }
  }

  async function addChapter(phaseId: string) {
    if (!newName.trim()) return;
    try {
      await invoke('add_outline_chapter', { phaseId, name: newName.trim() });
      newName = ''; adding = null;
      expanded[phaseId] = true;
      onrefresh();
    } catch (e) { console.error(e); }
  }
</script>

<div class="tree">
  {#if outline?.OutlineTree}
    {#each outline.OutlineTree as phase}
      <div class="phase-item">
        <div class="phase-row">
          <button class="phase-header" onclick={() => expanded[phase.phase.id] = !expanded[phase.phase.id]}>
            <span class="arrow">{expanded[phase.phase.id] ? '▼' : '▶'}</span>
            <span class="phase-name">{phase.phase.name}</span>
          </button>
          <button class="mini-btn" onclick={() => { adding = phase.phase.id; newName = ''; }} title="添加章节">+</button>
        </div>
        {#if adding === phase.phase.id}
          <div class="inline-form">
            <input bind:value={newName} placeholder="章名"
              onkeydown={(e) => e.key === 'Enter' && addChapter(phase.phase.id)} />
            <button class="mini-btn ok" onclick={() => addChapter(phase.phase.id)}>✓</button>
          </div>
        {/if}
        {#if expanded[phase.phase.id]}
          <div class="chapters">
            {#each phase.chapters as chapter}
              <button
                class="chapter-item"
                class:active={chapter.id === currentChapterId}
                onclick={() => onselect(chapter.text_chapter_id || chapter.id)}
              >
                <span class="status-dot" class:planned={!chapter.text_chapter_id} class:written={!!chapter.text_chapter_id}></span>
                <span class="chapter-name">{chapter.chapter_name}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  {/if}

  <!-- Add Phase button -->
  {#if adding === 'phase'}
    <div class="inline-form" style="padding: 4px 12px">
      <input bind:value={newName} placeholder="卷名"
        onkeydown={(e) => e.key === 'Enter' && addPhase()} />
      <button class="mini-btn ok" onclick={addPhase}>✓</button>
    </div>
  {/if}
  <button class="add-phase-btn" onclick={() => { adding = 'phase'; newName = ''; }}>
    + 添加卷
  </button>
</div>

<style>
  .tree { flex: 1; overflow-y: auto; padding: 4px 0; }
  .phase-row { display: flex; align-items: center; }
  .phase-header {
    flex: 1; display: flex; align-items: center; gap: 6px;
    padding: 8px 12px; background: none; color: var(--text);
    font-size: 13px; font-weight: 600; text-align: left; transition: background 0.15s;
  }
  .phase-header:hover { background: rgba(255,255,255,0.05); }
  .arrow { font-size: 10px; color: var(--text-dim); width: 12px; flex-shrink: 0; }
  .mini-btn {
    padding: 2px 8px; background: none; color: var(--text-dim); font-size: 16px; font-weight: 700;
    border: none; border-radius: 3px; margin-right: 4px; line-height: 1;
  }
  .mini-btn:hover { color: var(--accent); background: rgba(255,255,255,0.05); }
  .mini-btn.ok { color: var(--success); font-size: 14px; }
  .inline-form { display: flex; gap: 4px; padding: 2px 12px 6px; }
  .inline-form input { flex: 1; padding: 4px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 3px; color: var(--text); font-size: 12px; }
  .chapters { padding-left: 8px; }
  .chapter-item {
    width: 100%; display: flex; align-items: center; gap: 6px;
    padding: 6px 12px 6px 20px; background: none;
    color: var(--text-dim); font-size: 13px; text-align: left; transition: all 0.15s;
  }
  .chapter-item:hover { background: rgba(255,255,255,0.05); color: var(--text); }
  .chapter-item.active { background: var(--bg-panel); color: var(--accent); }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; transition: background 0.2s; }
  .status-dot.planned { background: var(--text-dim); opacity: 0.4; }
  .status-dot.written { background: var(--success); }
  .chapter-name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .add-phase-btn {
    width: calc(100% - 16px); margin: 8px; padding: 6px;
    background: var(--bg-panel); color: var(--text-dim); border: 1px dashed var(--border);
    border-radius: 4px; font-size: 12px; text-align: center;
  }
  .add-phase-btn:hover { color: var(--text); border-color: var(--accent-dim); }
</style>
