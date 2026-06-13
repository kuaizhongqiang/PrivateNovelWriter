<script lang="ts">
  let { outline, onselect, currentChapterId } = $props<{
    outline: any;
    onselect: (id: string) => void;
    currentChapterId: string | null;
  }>();

  let expanded = $state<Record<string, boolean>>({});
</script>

<div class="tree">
  {#if outline.OutlineTree}
    {#each outline.OutlineTree as phase}
      <div class="phase-item">
        <button
          class="phase-header"
          onclick={() => expanded[phase.phase.id] = !expanded[phase.phase.id]}
        >
          <span class="arrow">{expanded[phase.phase.id] ? '▼' : '▶'}</span>
          <span class="phase-name">{phase.phase.name}</span>
        </button>

        {#if expanded[phase.phase.id]}
          <div class="chapters">
            {#each phase.chapters as chapter}
              <button
                class="chapter-item"
                class:active={chapter.id === currentChapterId}
                onclick={() => onselect(chapter.id)}
              >
                <span class="chapter-status">
                  {chapter.text_chapter_id ? '📝' : '📄'}
                </span>
                <span class="chapter-name">{chapter.chapter_name}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .tree {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .phase-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: none;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    transition: background 0.15s;
  }

  .phase-header:hover {
    background: rgba(255,255,255,0.05);
  }

  .arrow {
    font-size: 10px;
    color: var(--text-dim);
    width: 12px;
  }

  .chapters {
    padding-left: 8px;
  }

  .chapter-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px 6px 20px;
    background: none;
    color: var(--text-dim);
    font-size: 12px;
    text-align: left;
    border-radius: 0;
    transition: all 0.15s;
  }

  .chapter-item:hover {
    background: rgba(255,255,255,0.05);
    color: var(--text);
  }

  .chapter-item.active {
    background: var(--bg-panel);
    color: var(--accent);
  }

  .chapter-status {
    font-size: 12px;
  }
</style>
