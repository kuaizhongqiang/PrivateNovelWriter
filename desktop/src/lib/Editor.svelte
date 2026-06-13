<script lang="ts">
  let { chapter, content, onsave } = $props<{
    chapter: any; content: string; onsave: (content: string) => void;
  }>();

  let editContent = $state('');
  let focusMode = $state(false);
  let showLineNumbers = $state(true);
  let fontScale = $state(1);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let saved = $state(true);

  let wordCount = $derived(editContent.split('').filter(c => !' \n\r\t'.includes(c)).length);
  let lines = $derived(editContent.split('\n'));
  let lineNumWidth = $derived(`${String(lines.length).length}ch`);

  $effect(() => { editContent = content; saved = true; });

  function handleInput() {
    saved = false;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => { onsave(editContent); saved = true; }, 2000);
  }

  function handleSave() { onsave(editContent); saved = true; }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') { e.preventDefault(); handleSave(); }
    if ((e.ctrlKey || e.metaKey) && e.key === 'f') { e.preventDefault(); focusMode = !focusMode; }
    if ((e.ctrlKey || e.metaKey) && e.key === '=') { e.preventDefault(); fontScale = Math.min(fontScale + 0.1, 1.5); }
    if ((e.ctrlKey || e.metaKey) && e.key === '-') { e.preventDefault(); fontScale = Math.max(fontScale - 0.1, 0.7); }
  }
</script>

<div class="editor" class:focus-mode={focusMode}>
  <!-- Header -->
  <div class="editor-header">
    <div class="header-left">
      <h2>{chapter.name}</h2>
      <span class="word-count">{wordCount} 字</span>
    </div>
    <div class="header-right">
      <button class="toolbar-btn" onclick={() => showLineNumbers = !showLineNumbers}
        title="行号">{showLineNumbers ? '☰' : '⊞'}</button>
      <button class="toolbar-btn" onclick={() => focusMode = !focusMode}
        title="专注模式 (Ctrl+F)">{focusMode ? '⛶' : '⛶'}</button>
      <button class="save-btn" class:saved onclick={handleSave}>{saved ? '✓ 已保存' : '保存'}</button>
    </div>
  </div>

  <!-- Editor body -->
  <div class="editor-body" style="font-size: {14 * fontScale}px">
    {#if showLineNumbers}
      <div class="line-numbers" style="width: {lineNumWidth}">
        {#each lines as _, i}
          <span class="line-num">{i + 1}</span>
        {/each}
      </div>
    {/if}
    <textarea
      class="editor-textarea"
      bind:value={editContent}
      oninput={handleInput}
      onkeydown={handleKeydown}
      placeholder="开始写作..."
      spellcheck="false"
    ></textarea>
  </div>
</div>

<style>
  .editor { height: 100%; display: flex; flex-direction: column; transition: all 0.3s ease; }
  .editor.focus-mode { background: var(--bg); }
  .editor-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 16px; border-bottom: 1px solid var(--border);
    background: var(--bg); transition: opacity 0.3s ease;
  }
  .focus-mode .editor-header { opacity: 0.1; }
  .focus-mode .editor-header:hover { opacity: 1; }
  .header-left { display: flex; align-items: baseline; gap: 12px; }
  .header-left h2 { font-size: 15px; font-weight: 600; }
  .word-count { font-size: 12px; color: var(--text-dim); }
  .header-right { display: flex; align-items: center; gap: 8px; }
  .toolbar-btn {
    padding: 4px 8px; background: var(--bg-panel); color: var(--text-dim);
    border: 1px solid var(--border); border-radius: 4px; font-size: 14px;
  }
  .toolbar-btn:hover { color: var(--text); border-color: var(--accent-dim); }
  .save-btn {
    padding: 6px 14px; border-radius: 4px; font-size: 12px; font-weight: 500;
    background: var(--bg-panel); color: var(--text); border: 1px solid var(--border);
    transition: all 0.2s ease;
  }
  .save-btn.saved { background: var(--success-bg); color: var(--success); border-color: var(--success); }
  .editor-body {
    flex: 1; display: flex; overflow: hidden; position: relative;
  }
  .line-numbers {
    padding: 16px 8px; text-align: right; user-select: none;
    color: var(--text-muted); border-right: 1px solid var(--border);
    overflow: hidden; background: var(--bg-secondary);
  }
  .line-num { display: block; font-family: var(--font-mono); font-size: 13px; line-height: 1.8; }
  .editor-textarea {
    flex: 1; padding: 16px 20px; background: var(--bg); color: var(--text);
    border: none; resize: none; font-family: var(--font-mono); font-size: 14px;
    line-height: 1.8; tab-size: 2;
  }
  .editor-textarea::placeholder { color: var(--text-dim); opacity: 0.4; }
  .editor-textarea:focus { outline: none; }
</style>
