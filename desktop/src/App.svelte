<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import Dashboard from './lib/Dashboard.svelte';
  import ProjectTree from './lib/ProjectTree.svelte';
  import Editor from './lib/Editor.svelte';
  import AgentPanel from './lib/AgentPanel.svelte';
  import CharacterPanel from './lib/CharacterPanel.svelte';
  import SettingPanel from './lib/SettingPanel.svelte';
  import SamplePanel from './lib/SamplePanel.svelte';

  let projectPath = $state('');
  let novelId = $state('');
  let outline = $state<any>(null);
  let currentChapterId = $state<string | null>(null);
  let currentChapter = $state<any>(null);
  let chapterContent = $state('');
  let agentMessages = $state<Array<{role: string; content: string; type?: string}>>([]);
  let stats = $state<any>(null);
  let characters = $state<any[]>([]);
  let setting = $state<any>(null);
  let samples = $state<any[]>([]);
  let showNewNovel = $state(false);
  let newNovelName = $state('');
  let sidebarTab = $state<'outline' | 'characters' | 'setting' | 'samples'>('outline');
  let eventUnlisten: (() => void) | null = null;
  let showShortcuts = $state(false);

  onMount(async () => {
    eventUnlisten = await listen<any>('llm-event', (event) => {
      const payload = event.payload;
      if (agentMessages.length === 0) return;
      const lastMsg = agentMessages[agentMessages.length - 1];
      if (lastMsg.role !== 'assistant') return;
      switch (payload.type) {
        case 'token': lastMsg.content += payload.data; agentMessages = [...agentMessages]; break;
        case 'thinking': lastMsg.content += `🧠 ${payload.data}`; agentMessages = [...agentMessages]; break;
        case 'step': lastMsg.content += `\n⚙️ ${payload.name}: ${payload.status}`; agentMessages = [...agentMessages]; break;
        case 'tool_call': lastMsg.content += `\n🔧 调用工具: ${payload.name}`; agentMessages = [...agentMessages]; break;
        case 'done': lastMsg.type = 'done'; agentMessages = [...agentMessages]; break;
        case 'error': lastMsg.content += `\n❌ ${payload.data}`; lastMsg.type = 'error'; agentMessages = [...agentMessages]; break;
      }
    });
    // Global keyboard shortcuts
    document.addEventListener('keydown', handleGlobalKeydown);

    try {
      const p = await invoke<string | null>('get_project_path');
      if (p) { projectPath = p; await loadProject(); }
    } catch {}
  });

  onDestroy(() => {
    if (eventUnlisten) eventUnlisten();
    document.removeEventListener('keydown', handleGlobalKeydown);
  });

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.key === '?' && !e.ctrlKey && !e.metaKey) { showShortcuts = !showShortcuts; e.preventDefault(); }
    if (e.key === 'Escape' && showShortcuts) { showShortcuts = false; }
    if ((e.ctrlKey || e.metaKey) && e.key === '1') { sidebarTab = 'outline'; }
    if ((e.ctrlKey || e.metaKey) && e.key === '2') { sidebarTab = 'characters'; }
    if ((e.ctrlKey || e.metaKey) && e.key === '3') { sidebarTab = 'setting'; }
    if ((e.ctrlKey || e.metaKey) && e.key === '4') { sidebarTab = 'samples'; }
  }

  async function loadProject() {
    try {
      const result = await invoke<any>('get_outline');
      outline = result;
      await refreshStats();
      await refreshCharacters();
      await refreshSetting();
      await refreshSamples();
    } catch (e) { console.error(e); }
  }

  async function refreshStats() {
    try { stats = await invoke<any>('get_stats'); } catch {}
  }
  async function refreshCharacters() {
    try { characters = await invoke<any>('list_characters'); characters = characters?.CharacterList ?? []; } catch {}
  }
  async function refreshSetting() {
    try { setting = await invoke<any>('get_setting'); setting = setting?.Setting ?? null; } catch {}
  }
  async function refreshSamples() {
    try { samples = await invoke<any>('list_samples'); samples = samples?.SampleList ?? []; } catch {}
  }

  async function handleOpenProject() {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const dir = await open({ directory: true });
    if (dir) {
      try {
        projectPath = await invoke<string>('open_project', { path: dir });
        await loadProject();
      } catch (e) { agentMessages = [...agentMessages, { role: 'system', content: `打开失败: ${e}` }]; }
    }
  }

  async function handleNewNovel() {
    if (!newNovelName.trim()) return;
    try {
      projectPath = await invoke<string>('new_project', { name: newNovelName.trim() });
      newNovelName = ''; showNewNovel = false;
      await loadProject();
      agentMessages = [...agentMessages, { role: 'system', content: '✅ 已创建新项目' }];
    } catch (e) { agentMessages = [...agentMessages, { role: 'system', content: `创建失败: ${e}` }]; }
  }

  async function handleSelectChapter(chapterId: string) {
    currentChapterId = chapterId;
    try {
      const result = await invoke<any>('get_chapter', { chapterId });
      currentChapter = result.chapter;
      chapterContent = result.content;
    } catch (e) { console.error(e); }
  }

  async function handleSaveContent(content: string) {
    if (!currentChapterId) return;
    chapterContent = content;
    try { await invoke('save_chapter', { chapterId: currentChapterId, content }); } catch (e) { console.error(e); }
  }

  function classifyCommand(msg: string): { commandType: string, chapterId?: string } {
    const lower = msg.toLowerCase();
    if (lower.startsWith('/eval') || lower.includes('评估')) return { commandType: 'evaluate' };
    if (lower.startsWith('/rev') || lower.includes('修改') || lower.includes('改写')) return { commandType: 'revise', chapterId: currentChapterId || undefined };
    if (lower.startsWith('/plan') || (lower.includes('规划') && lower.includes('大纲'))) return { commandType: 'plan' };
    return { commandType: 'write' };
  }

  async function handleSendMessage(msg: string) {
    if (!msg.trim()) return;
    const { commandType, chapterId } = classifyCommand(msg);
    agentMessages = [...agentMessages, { role: 'user', content: msg }, { role: 'assistant', content: '', type: 'streaming' }];
    try {
      const summary = await invoke<string>('agent_chat', { commandType, chapterId: chapterId || null, message: msg });
      agentMessages[agentMessages.length - 1] = { role: 'assistant', content: summary, type: 'done' };
      agentMessages = [...agentMessages];
      await Promise.all([refreshOutline(), refreshStats()]);
    } catch (e) {
      agentMessages[agentMessages.length - 1] = { role: 'assistant', content: `错误: ${e}`, type: 'error' };
      agentMessages = [...agentMessages];
    }
  }

  async function refreshOutline() {
    try { outline = await invoke<any>('get_outline'); } catch {}
  }
</script>

<div class="app-layout">
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1>NovelWriter</h1>
    </div>
    <div class="sidebar-actions">
      <button class="btn btn-primary" onclick={() => showNewNovel = true}>➕ 新建</button>
      <button class="btn btn-secondary" onclick={handleOpenProject}>📂 打开</button>
    </div>
    {#if showNewNovel}
      <div class="new-novel-form">
        <input type="text" bind:value={newNovelName} placeholder="小说名称"
               onkeydown={(e) => (e.key === 'Enter') && handleNewNovel()} />
        <button class="btn btn-small" onclick={handleNewNovel}>创建</button>
      </div>
    {/if}

    <!-- Sidebar Tabs -->
    <div class="sidebar-tabs">
      <button class="tab" class:active={sidebarTab === 'outline'} onclick={() => sidebarTab = 'outline'}>📂</button>
      <button class="tab" class:active={sidebarTab === 'characters'} onclick={() => sidebarTab = 'characters'}>👤</button>
      <button class="tab" class:active={sidebarTab === 'setting'} onclick={() => sidebarTab = 'setting'}>⚙</button>
      <button class="tab" class:active={sidebarTab === 'samples'} onclick={() => sidebarTab = 'samples'}>📋</button>
    </div>

    <!-- Tab Content -->
    <div class="tab-content">
      {#if sidebarTab === 'outline'}
        {#if outline}
          <ProjectTree outline={outline} onselect={handleSelectChapter} currentChapterId={currentChapterId} />
        {/if}
      {:else if sidebarTab === 'characters'}
        <CharacterPanel {characters} />
      {:else if sidebarTab === 'setting'}
        <SettingPanel {setting} />
      {:else if sidebarTab === 'samples'}
        <SamplePanel {samples} />
      {/if}
    </div>

    <!-- Stats (always visible) -->
    {#if stats}
      <div class="stats-bar">
        <div class="stat"><span class="stat-label">已写</span><span class="stat-val">{stats.total_written}字</span></div>
        <div class="stat"><span class="stat-label">章节</span><span class="stat-val">{stats.written_chapters}/{stats.planned_chapters}</span></div>
      </div>
    {/if}
  </aside>

  <!-- Editor / Dashboard -->
  <main class="editor-area">
    {#if currentChapter}
      <Editor chapter={currentChapter} content={chapterContent} onsave={handleSaveContent} />
    {:else}
      <Dashboard {stats} {characters} {setting} />
    {/if}
  </main>

  <!-- Agent Panel -->
  <aside class="agent-panel">
    <AgentPanel messages={agentMessages} onsend={handleSendMessage} />
  </aside>
</div>

<!-- Shortcuts Help -->
{#if showShortcuts}
  <div class="shortcuts-overlay" onclick={() => showShortcuts = false}>
    <div class="shortcuts-panel" onclick={(e) => e.stopPropagation()}>
      <h2>⌨ 快捷键</h2>
      <div class="shortcut-grid">
        <div class="sc-group"><h3>全局</h3></div>
        <span class="sc-key">?</span><span class="sc-desc">打开快捷键帮助</span>
        <span class="sc-key">Ctrl+1-4</span><span class="sc-desc">切换侧栏 Tab</span>
        <span class="sc-key">Ctrl+F</span><span class="sc-desc">专注模式</span>
        <span class="sc-key">Ctrl+S</span><span class="sc-desc">保存</span>
        <span class="sc-key">Ctrl++/-</span><span class="sc-desc">字体缩放</span>
        <div class="sc-group"><h3>Agent B</h3></div>
        <span class="sc-key">Ctrl+Enter</span><span class="sc-desc">发送消息</span>
      </div>
      <p class="shortcuts-hint">点击任意处关闭</p>
    </div>
  </div>
{/if}

<style>
  .app-layout { display: grid; grid-template-columns: 260px 1fr 320px; height: 100vh; overflow: hidden; }
  .sidebar { background: var(--bg-secondary); border-right: 1px solid var(--border); display: flex; flex-direction: column; overflow: hidden; }
  .sidebar-header { padding: 12px 16px; border-bottom: 1px solid var(--border); }
  .sidebar-header h1 { font-size: 16px; font-weight: 700; color: var(--accent); }
  .sidebar-actions { padding: 10px 12px; display: flex; gap: 6px; }
  .new-novel-form { padding: 0 12px 10px; display: flex; gap: 6px; }
  .new-novel-form input { flex: 1; padding: 6px 10px; background: var(--bg); border: 1px solid var(--border); border-radius: 4px; color: var(--text); font-size: 13px; }

  /* Tabs */
  .sidebar-tabs { display: flex; border-bottom: 1px solid var(--border); padding: 0 4px; }
  .tab {
    flex: 1; padding: 8px 0; background: none; color: var(--text-dim); font-size: 14px;
    border: none; border-bottom: 2px solid transparent; transition: all 0.15s;
  }
  .tab:hover { color: var(--text); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }

  .tab-content { flex: 1; overflow-y: auto; }

  .stats-bar {
    display: flex; gap: 12px; padding: 8px 12px;
    border-top: 1px solid var(--border);
  }
  .stat { display: flex; flex-direction: column; }
  .stat-label { font-size: 10px; color: var(--text-dim); text-transform: uppercase; }
  .stat-val { font-size: 13px; font-weight: 600; }

  .editor-area { background: var(--bg); overflow: hidden; display: flex; flex-direction: column; }
  .editor-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; color: var(--text-dim); gap: 8px; }
  .agent-panel { background: var(--bg-secondary); border-left: 1px solid var(--border); overflow: hidden; }

  .btn { padding: 6px 12px; border-radius: 4px; font-size: 13px; font-weight: 500; }
  .btn-primary { background: var(--accent); color: white; }
  .btn-primary:hover { background: var(--accent-hover); }
  .btn-secondary { background: var(--bg-panel); color: var(--text); border: 1px solid var(--border); }
  .btn-secondary:hover { background: var(--border); }
  .btn-small { padding: 4px 10px; background: var(--accent); color: white; font-size: 12px; border-radius: 3px; }
  .text-dim { color: var(--text-dim); }

  /* Shortcuts */
  .shortcuts-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 100; animation: fadeIn 0.15s ease;
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  .shortcuts-panel {
    background: var(--bg-secondary); border: 1px solid var(--border);
    border-radius: 12px; padding: 24px; max-width: 400px; width: 90%;
    max-height: 80vh; overflow-y: auto;
  }
  .shortcuts-panel h2 { font-size: 18px; margin-bottom: 16px; }
  .shortcut-grid { display: grid; grid-template-columns: auto 1fr; gap: 8px 16px; align-items: center; }
  .sc-group { grid-column: 1 / -1; margin-top: 8px; }
  .sc-group h3 { font-size: 12px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
  .sc-key {
    font-family: var(--font-mono); font-size: 12px; padding: 3px 8px;
    background: var(--bg); border: 1px solid var(--border); border-radius: 4px;
    text-align: center; white-space: nowrap;
  }
  .sc-desc { font-size: 13px; }
  .shortcuts-hint { text-align: center; font-size: 12px; color: var(--text-dim); margin-top: 16px; }
</style>
