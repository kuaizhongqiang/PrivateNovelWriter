<script lang="ts">
  let { messages, onsend } = $props<{
    messages: Array<{role: string; content: string; type?: string}>;
    onsend: (msg: string) => void;
  }>();

  let inputText = $state('');
  let messagesContainer: HTMLDivElement | undefined = $state();

  function handleSend() {
    if (!inputText.trim()) return;
    onsend(inputText.trim());
    inputText = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); }
  }

  function quickCmd(cmd: string) {
    onsend(cmd);
  }

  $effect(() => {
    if (messages.length && messagesContainer) {
      requestAnimationFrame(() => { messagesContainer!.scrollTop = messagesContainer!.scrollHeight; });
    }
  });
</script>

<div class="agent-panel">
  <div class="panel-header">
    <h3>Agent B</h3>
    <span class="badge">写作专家</span>
  </div>

  <div class="quick-commands">
    <button class="qc-btn" onclick={() => quickCmd('写下一章正文')}>✍ 写下一章</button>
    <button class="qc-btn" onclick={() => quickCmd('评估当前章节')}>📊 评估</button>
    <button class="qc-btn" onclick={() => quickCmd('规划大纲，十章左右')}>📋 规划大纲</button>
  </div>

  <div class="messages" bind:this={messagesContainer}>
    {#each messages as msg, i}
      <div class="message" class:user={msg.role === 'user'} class:system={msg.role === 'system'}>
        {#if msg.role === 'user'}
          <div class="avatar ua">U</div>
          <div class="bubble ub">{msg.content}</div>
        {:else if msg.role === 'system'}
          <div class="bubble sb">{msg.content}</div>
        {:else}
          <div class="avatar ba">B</div>
          <div class="bubble ab">
            {#if msg.type === 'streaming'}
              <span>{msg.content}<span class="cursor">▊</span></span>
            {:else if msg.type === 'done'}
              <span>{msg.content}</span>
            {:else if msg.type === 'error'}
              <span class="error-text">{msg.content}</span>
            {:else}
              <span>{msg.content}</span>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
    {#if messages.length === 0}
      <div class="empty-state">
        <p>向写作专家发送消息</p>
        <p class="hints">写正文 · 修改 · 评估 · 规划大纲</p>
      </div>
    {/if}
  </div>

  <div class="input-area">
    <textarea bind:value={inputText} onkeydown={handleKeydown}
      placeholder="输入指令..." rows={2}></textarea>
    <button class="send-btn" onclick={handleSend} disabled={!inputText.trim()}>发送</button>
  </div>
</div>

<style>
  .agent-panel { height: 100%; display: flex; flex-direction: column; }
  .panel-header { display: flex; align-items: center; gap: 8px; padding: 10px 16px; border-bottom: 1px solid var(--border); }
  .panel-header h3 { font-size: 14px; font-weight: 600; }
  .badge { font-size: 10px; padding: 2px 6px; border-radius: 3px; background: var(--accent); color: white; font-weight: 500; }

  .quick-commands { display: flex; gap: 4px; padding: 6px 8px; border-bottom: 1px solid var(--border); flex-wrap: wrap; }
  .qc-btn {
    padding: 3px 8px; border-radius: 4px; font-size: 11px; font-weight: 500;
    background: var(--bg-panel); color: var(--text-dim); border: 1px solid var(--border);
    transition: all 0.15s; white-space: nowrap;
  }
  .qc-btn:hover { color: var(--text); border-color: var(--accent-dim); }

  .messages { flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px; }
  .message { display: flex; gap: 6px; align-items: flex-start; animation: fadeIn 0.2s ease; }
  .message.user { flex-direction: row-reverse; }
  @keyframes fadeIn { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: translateY(0); } }

  .avatar {
    width: 24px; height: 24px; border-radius: 50%; display: flex; align-items: center;
    justify-content: center; font-size: 11px; font-weight: 700; flex-shrink: 0;
  }
  .ua { background: var(--accent); color: white; }
  .ba { background: var(--info); color: white; }

  .bubble { padding: 6px 10px; border-radius: 8px; font-size: 13px; line-height: 1.5; max-width: 85%; white-space: pre-wrap; word-break: break-word; }
  .ub { background: var(--accent); color: white; border-bottom-right-radius: 2px; }
  .ab { background: var(--bg-panel); color: var(--text); border-bottom-left-radius: 2px; }
  .sb { background: var(--success-bg); color: var(--success); font-size: 12px; width: 100%; text-align: center; border-radius: 4px; }
  .cursor { animation: blink 1s step-end infinite; color: var(--accent); }
  @keyframes blink { 50% { opacity: 0; } }
  .error-text { color: #ef4444; }

  .empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; color: var(--text-dim); text-align: center; padding: 20px; }
  .hints { font-size: 12px; margin-top: 8px; opacity: 0.6; }

  .input-area { padding: 8px; border-top: 1px solid var(--border); display: flex; gap: 6px; align-items: flex-end; }
  .input-area textarea { flex: 1; padding: 6px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 13px; resize: none; line-height: 1.4; }
  .input-area textarea:focus { border-color: var(--accent-dim); }
  .send-btn { padding: 6px 14px; background: var(--accent); color: white; border-radius: 6px; font-size: 13px; font-weight: 500; }
  .send-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .send-btn:not(:disabled):hover { background: var(--accent-hover); }
</style>
