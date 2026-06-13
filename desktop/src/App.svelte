<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import ProjectTree from './lib/ProjectTree.svelte';
  import Editor from './lib/Editor.svelte';
  import AgentPanel from './lib/AgentPanel.svelte';

  let projectPath = $state('');
  let novelId = $state('');
  let novelName = $state('');
  let novels = $state<any[]>([]);
  let outline = $state<any>(null);
  let currentChapterId = $state<string | null>(null);
  let currentChapter = $state<any>(null);
  let chapterContent = $state('');
  let agentMessages = $state<Array<{role: string, content: string, type?: string}>>([]);
  let stats = $state<any>(null);
  let showNewNovel = $state(false);
  let newNovelName = $state('');
  let currentStreamingId = $state<number>(0);
  let eventUnlisten: (() => void) | null = null;

  // 单一事件监听，组件生命周期内有效
  onMount(async () => {
    eventUnlisten = await listen<any>('llm-event', (event) => {
      const payload = event.payload;
      // 只处理当前活跃消息的事件
      if (agentMessages.length === 0) return;
      const lastMsg = agentMessages[agentMessages.length - 1];
      if (lastMsg.role !== 'assistant') return;

      switch (payload.type) {
        case 'token':
          lastMsg.content += payload.data;
          agentMessages = [...agentMessages];
          break;
        case 'thinking':
          lastMsg.content += `🧠 ${payload.data}`;
          agentMessages = [...agentMessages];
          break;
        case 'step':
          // 显示步骤状态
          lastMsg.content += `\n⚙️ ${payload.name}: ${payload.status}`;
          agentMessages = [...agentMessages];
          break;
        case 'tool_call':
          lastMsg.content += `\n🔧 调用工具: ${payload.name}`;
          agentMessages = [...agentMessages];
          break;
        case 'done':
          lastMsg.type = 'done';
          agentMessages = [...agentMessages];
          break;
        case 'error':
          lastMsg.content += `\n❌ ${payload.data}`;
          lastMsg.type = 'error';
          agentMessages = [...agentMessages];
          break;
      }
    });

    // Try to restore last project
    try {
      const p = await invoke<string | null>('get_project_path');
      if (p) {
        projectPath = p;
        await loadProject();
      }
    } catch {}
  });

  // 清理事件监听
  import { onDestroy } from 'svelte';
  onDestroy(() => {
    if (eventUnlisten) eventUnlisten();
  });

  async function loadProject() {
    try {
      const result = await invoke<any>('list_novels');
      const list = result?.NovelList || [];
      novels = list;
      if (list.length > 0) {
        novelId = list[0].id;
        novelName = list[0].name;
        await refreshOutline();
        await refreshStats();
      }
    } catch (e) {
      console.error('load project error:', e);
    }
  }

  async function refreshOutline() {
    if (!novelId) return;
    try {
      outline = await invoke<any>('get_outline', { novelId });
    } catch {}
  }

  async function refreshStats() {
    if (!novelId) return;
    try {
      stats = await invoke<any>('get_stats', { novelId });
    } catch {}
  }

  async function handleOpenProject() {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const dir = await open({ directory: true });
    if (dir) {
      try {
        projectPath = await invoke<string>('open_project', { path: dir });
        await loadProject();
      } catch (e) {
        agentMessages = [...agentMessages, { role: 'system', content: `打开项目失败: ${e}` }];
      }
    }
  }

  async function handleNewNovel() {
    if (!newNovelName.trim()) return;
    try {
      projectPath = await invoke<string>('new_project', { name: newNovelName.trim() });
      newNovelName = '';
      showNewNovel = false;
      await loadProject();
      agentMessages = [...agentMessages, { role: 'system', content: `✅ 已创建新项目` }];
    } catch (e) {
      agentMessages = [...agentMessages, { role: 'system', content: `创建失败: ${e}` }];
    }
  }

  async function handleSelectChapter(chapterId: string) {
    currentChapterId = chapterId;
    try {
      const result = await invoke<any>('get_chapter', { chapterId });
      currentChapter = result.chapter;
      chapterContent = result.content;
    } catch (e) {
      console.error('load chapter error:', e);
    }
  }

  async function handleSaveContent(content: string) {
    if (!currentChapterId) return;
    chapterContent = content;
    try {
      await invoke('save_chapter', { chapterId: currentChapterId, content });
    } catch (e) {
      console.error('save error:', e);
    }
  }

  function classifyCommand(msg: string): { commandType: string, chapterId?: string } {
    const lower = msg.toLowerCase();
    if (lower.startsWith('/evaluate') || lower.includes('评估') && !lower.includes('大纲')) {
      return { commandType: 'evaluate' };
    }
    if (lower.startsWith('/revise') || lower.includes('修改') || lower.includes('改写')) {
      return { commandType: 'revise', chapterId: currentChapterId || undefined };
    }
    if (lower.startsWith('/plan') || lower.includes('规划') && lower.includes('大纲')) {
      return { commandType: 'plan' };
    }
    return { commandType: 'write' };
  }

  async function handleSendMessage(msg: string) {
    if (!novelId || !msg.trim()) return;
    const { commandType, chapterId } = classifyCommand(msg);
    const placeholderId = ++currentStreamingId;
    agentMessages = [...agentMessages, { role: 'user', content: msg }];
    agentMessages = [...agentMessages, { role: 'assistant', content: '', type: 'streaming', _id: placeholderId }];

    try {
      const summary = await invoke<string>('agent_chat', {
        commandType,
        chapterId: chapterId || null,
        message: msg,
      });
      agentMessages[agentMessages.length - 1] = { role: 'assistant', content: summary, type: 'done' };
      agentMessages = [...agentMessages];
      await refreshOutline();
      await refreshStats();
    } catch (e) {
      agentMessages[agentMessages.length - 1] = { role: 'assistant', content: `错误: ${e}`, type: 'error' };
      agentMessages = [...agentMessages];
    }
  }
</script>

<div class="app-layout">
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1>NovelWriter</h1>
      {#if novelName}
        <span class="novel-name">{novelName}</span>
      {/if}
    </div>

    <div class="sidebar-actions">
      <button class="btn btn-primary" onclick={() => showNewNovel = true}>➕ 新建</button>
      <button class="btn btn-secondary" onclick={handleOpenProject}>📂 打开</button>
    </div>

    {#if showNewNovel}
      <div class="new-novel-form">
        <input
          type="text"
          bind:value={newNovelName}
          placeholder="小说名称"
          onkeydown={(e) => e.key === 'Enter' && handleNewNovel()}
        />
        <button class="btn btn-small" onclick={handleNewNovel}>创建</button>
      </div>
    {/if}

    {#if outline}
      <ProjectTree
        outline={outline}
        onselect={handleSelectChapter}
        currentChapterId={currentChapterId}
      />
    {/if}

    {#if stats}
      <div class="stats-panel">
        <div class="stat-item">
          <span class="stat-label">已写</span>
          <span class="stat-value">{stats.total_written}字</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">章节</span>
          <span class="stat-value">{stats.written_chapters}/{stats.planned_chapters}</span>
        </div>
      </div>
    {/if}
  </aside>

  <!-- Editor -->
  <main class="editor-area">
    {#if currentChapter}
      <Editor
        chapter={currentChapter}
        content={chapterContent}
        onsave={handleSaveContent}
      />
    {:else}
      <div class="editor-empty">
        <p>从左侧选择一个章节开始写作</p>
        <p class="text-dim">或使用 Agent B 对话生成</p>
      </div>
    {/if}
  </main>

  <!-- Agent Panel -->
  <aside class="agent-panel">
    <AgentPanel
      messages={agentMessages}
      onsend={handleSendMessage}
    />
  </aside>
</div>

<style>
  .app-layout {
    display: grid;
    grid-template-columns: 260px 1fr 320px;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 16px;
    border-bottom: 1px solid var(--border);
  }

  .sidebar-header h1 {
    font-size: 16px;
    font-weight: 700;
    color: var(--accent);
  }

  .novel-name {
    font-size: 12px;
    color: var(--text-dim);
    margin-top: 4px;
    display: block;
  }

  .sidebar-actions {
    padding: 12px;
    display: flex;
    gap: 8px;
  }

  .new-novel-form {
    padding: 0 12px 12px;
    display: flex;
    gap: 8px;
  }

  .new-novel-form input {
    flex: 1;
    padding: 6px 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-size: 13px;
  }

  .stats-panel {
    padding: 12px;
    border-top: 1px solid var(--border);
    display: flex;
    gap: 16px;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
  }

  .stat-label {
    font-size: 11px;
    color: var(--text-dim);
  }

  .stat-value {
    font-size: 14px;
    font-weight: 600;
  }

  .editor-area {
    background: var(--bg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .editor-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-dim);
    gap: 8px;
  }

  .editor-empty p { margin: 0; }

  .agent-panel {
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    overflow: hidden;
  }

  .btn {
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 13px;
    font-weight: 500;
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-secondary {
    background: var(--bg-panel);
    color: var(--text);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover {
    background: var(--border);
  }

  .btn-small {
    padding: 4px 10px;
    background: var(--accent);
    color: white;
    font-size: 12px;
    border-radius: 3px;
  }

  .text-dim { color: var(--text-dim); }
</style>
