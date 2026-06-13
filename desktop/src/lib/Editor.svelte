<script lang="ts">
  import { tick } from 'svelte';

  let { chapter, content, onsave } = $props<{
    chapter: any;
    content: string;
    onsave: (content: string) => void;
  }>();

  let editContent = $state('');
  let wordCount = $derived(
    editContent.split('').filter(c => !' \n\r\t'.includes(c)).length
  );
  let saved = $state(true);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    editContent = content;
    saved = true;
  });

  function handleInput() {
    saved = false;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      onsave(editContent);
      saved = true;
    }, 2000);
  }

  async function handleSave() {
    onsave(editContent);
    saved = true;
  }
</script>

<div class="editor">
  <div class="editor-header">
    <div class="chapter-info">
      <h2>{chapter.name}</h2>
      <span class="word-count">{wordCount} 字</span>
    </div>
    <button class="save-btn" class:saved onclick={handleSave}>
      {saved ? '✓ 已保存' : '保存'}
    </button>
  </div>

  <textarea
    class="editor-textarea"
    bind:value={editContent}
    oninput={handleInput}
    onkeydown={(e) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
    }}
    placeholder="开始写作..."
  ></textarea>
</div>

<style>
  .editor {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }

  .chapter-info h2 {
    font-size: 15px;
    font-weight: 600;
  }

  .word-count {
    font-size: 12px;
    color: var(--text-dim);
    margin-top: 2px;
    display: block;
  }

  .save-btn {
    padding: 6px 14px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
    background: var(--bg-panel);
    color: var(--text);
    border: 1px solid var(--border);
    transition: all 0.2s;
  }

  .save-btn.saved {
    background: rgba(34, 197, 94, 0.15);
    color: var(--success);
    border-color: var(--success);
  }

  .editor-textarea {
    flex: 1;
    width: 100%;
    padding: 20px;
    background: var(--bg);
    color: var(--text);
    border: none;
    resize: none;
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.8;
    tab-size: 2;
  }

  .editor-textarea::placeholder {
    color: var(--text-dim);
    opacity: 0.5;
  }
</style>
