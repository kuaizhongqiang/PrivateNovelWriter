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
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  // Auto scroll on new messages
  $effect(() => {
    if (messages.length && messagesContainer) {
      requestAnimationFrame(() => {
        messagesContainer!.scrollTop = messagesContainer!.scrollHeight;
      });
    }
  });
</script>

<div class="agent-panel">
  <div class="panel-header">
    <h3>Agent B</h3>
    <span class="badge">写作专家</span>
  </div>

  <div class="messages" bind:this={messagesContainer}>
    {#each messages as msg, i}
      <div class="message" class:user={msg.role === 'user'} class:system={msg.role === 'system'}>
        {#if msg.role === 'user'}
          <div class="avatar user-avatar">U</div>
          <div class="bubble user-bubble">{msg.content}</div>
        {:else if msg.role === 'system'}
          <div class="bubble system-bubble">{msg.content}</div>
        {:else}
          <div class="avatar agent-avatar">B</div>
          <div class="bubble agent-bubble">
            {#if msg.type === 'streaming'}
              <span class="streaming-text">{msg.content}<span class="cursor">▊</span></span>
            {:else if msg.type === 'has-thinking'}
              <div class="thinking-bubble">
                <span class="thinking-icon">🧠</span>
                {msg.content}
              </div>
            {:else}
              {msg.content}
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    {#if messages.length === 0}
      <div class="empty-state">
        <p>向写作专家发送消息</p>
        <p class="hints">
          例如: "写第五章正文"<br>
          "修改第三章的打斗场景"<br>
          "评估第四章"<br>
          "规划大纲"
        </p>
      </div>
    {/if}
  </div>

  <div class="input-area">
    <textarea
      bind:value={inputText}
      onkeydown={handleKeydown}
      placeholder="输入指令..."
      rows={2}
    ></textarea>
    <button class="send-btn" onclick={handleSend} disabled={!inputText.trim()}>
      发送
    </button>
  </div>
</div>

<style>
  .agent-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .panel-header h3 {
    font-size: 14px;
    font-weight: 600;
  }

  .badge {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--accent);
    color: white;
    font-weight: 500;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .message {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }

  .message.user {
    flex-direction: row-reverse;
  }

  .avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .user-avatar {
    background: var(--accent);
    color: white;
  }

  .agent-avatar {
    background: var(--tool-call);
    color: white;
  }

  .bubble {
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.5;
    max-width: 85%;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .user-bubble {
    background: var(--accent);
    color: white;
    border-bottom-right-radius: 2px;
  }

  .agent-bubble {
    background: var(--bg-panel);
    color: var(--text);
    border-bottom-left-radius: 2px;
  }

  .system-bubble {
    background: rgba(34, 197, 94, 0.1);
    color: var(--success);
    font-size: 12px;
    width: 100%;
    text-align: center;
    border-radius: 4px;
  }

  .streaming-text {
    animation: fade-in 0.1s;
  }

  .cursor {
    animation: blink 1s step-end infinite;
    color: var(--accent);
  }

  @keyframes blink {
    50% { opacity: 0; }
  }

  .thinking-bubble {
    background: rgba(245, 158, 11, 0.1);
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--thinking);
    margin-bottom: 8px;
    display: flex;
    gap: 6px;
    align-items: flex-start;
  }

  .thinking-icon {
    flex-shrink: 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-dim);
    text-align: center;
    padding: 20px;
  }

  .empty-state p {
    margin: 0;
  }

  .hints {
    font-size: 12px;
    margin-top: 12px !important;
    opacity: 0.6;
    line-height: 1.8;
  }

  .input-area {
    padding: 12px;
    border-top: 1px solid var(--border);
    display: flex;
    gap: 8px;
    align-items: flex-end;
  }

  .input-area textarea {
    flex: 1;
    padding: 8px 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 13px;
    resize: none;
    line-height: 1.4;
  }

  .input-area textarea:focus {
    border-color: var(--accent);
  }

  .send-btn {
    padding: 8px 16px;
    background: var(--accent);
    color: white;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    transition: opacity 0.15s;
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .send-btn:not(:disabled):hover {
    background: var(--accent-hover);
  }
</style>
