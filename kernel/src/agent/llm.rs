use futures::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("No LLM provider configured")]
    NoProvider,
    #[error("Stream error: {0}")]
    Stream(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

/// 流式事件，供 Desktop 实时展示
#[derive(Debug, Clone, Serialize)]
pub enum LlmEvent {
    /// 文本 token
    Token(String),
    /// 思考/推理过程
    Thinking(String),
    /// Agent B 正在执行的步骤
    Step { name: String, status: String },
    /// 工具调用
    ToolCall { name: String, args: serde_json::Value },
    /// 工具调用结果
    ToolResult { name: String, result: String },
    /// 完成
    Done,
    /// 错误
    Error(String),
}

/// LLM 提供商抽象
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// 非流式聊天
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse, LlmError>;

    /// 流式聊天，通过 sender 发送事件
    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        sender: tokio::sync::mpsc::UnboundedSender<LlmEvent>,
    ) -> Result<(), LlmError>;
}

/// OpenAI 兼容 API 提供商 (DeepSeek / LM Studio)
pub struct OpenAiCompatible {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiCompatible {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::Api(format!("Client build error: {}", e)))?;
        let body = build_request_body(&self.model, messages, tools, false);

        let resp = client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            return Err(LlmError::Api(format!(
                "API returned {}: {}",
                status,
                json.get("error").map(|e| e.to_string()).unwrap_or_default()
            )));
        }

        extract_response(json)
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        sender: tokio::sync::mpsc::UnboundedSender<LlmEvent>,
    ) -> Result<(), LlmError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::Api(format!("Client build error: {}", e)))?;
        let body = build_request_body(&self.model, messages, tools, true);

        let resp = client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let json: serde_json::Value = resp.json().await?;
            return Err(LlmError::Api(format!(
                "API returned {}: {}",
                status,
                json.get("error").map(|e| e.to_string()).unwrap_or_default()
            )));
        }

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();
        let mut full_content = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| LlmError::Stream(e.to_string()))?;
            buf.extend_from_slice(&chunk);

            // 处理缓冲区中所有完整的 SSE 行（以 \n 结尾）
            // 未完成的部分留在 buf 中等待下一个 chunk
            let buf_str = std::str::from_utf8(&buf).unwrap_or("");
            let mut last_newline = 0;

            for (i, _) in buf_str.match_indices('\n') {
                let line = &buf_str[last_newline..i].trim();
                last_newline = i + 1;
                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }
                let data = &line[6..]; // 去掉 "data: "
                if data == "[DONE]" {
                    sender.send(LlmEvent::Done).ok();
                    continue;
                }

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choice) = json["choices"][0].as_object() {
                        let delta = choice.get("delta");

                        // 文本 token
                        if let Some(content) = delta.and_then(|d| d.get("content")).and_then(|c| c.as_str()) {
                            if !content.is_empty() {
                                full_content.push_str(content);
                                sender.send(LlmEvent::Token(content.to_string())).ok();
                            }
                        }

                        // 思考/reasoning token（DeepSeek 的 reasoning_content）
                        if let Some(thinking) = delta
                            .and_then(|d| d.get("reasoning_content"))
                            .and_then(|c| c.as_str())
                        {
                            if !thinking.is_empty() {
                                sender.send(LlmEvent::Thinking(thinking.to_string())).ok();
                            }
                        }

                        // 工具调用
                        if let Some(tool_calls) = delta.and_then(|d| d.get("tool_calls")).and_then(|tc| tc.as_array()) {
                            for tc in tool_calls {
                                if let Some(name) = tc["function"]["name"].as_str() {
                                    let args = tc["function"]["arguments"]
                                        .as_str()
                                        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                                        .unwrap_or_default();
                                    sender
                                        .send(LlmEvent::ToolCall {
                                            name: name.to_string(),
                                            args,
                                        })
                                        .ok();
                                }
                            }
                        }

                        // finish_reason
                        if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                            if reason != "null" && !reason.is_empty() {
                                sender.send(LlmEvent::Done).ok();
                            }
                        }
                    }
                }
            }
            // 移除已处理的行，保留未完成的字节
            if last_newline > 0 {
                buf.drain(..last_newline);
            }
        }

        // 确保 Done 事件发送（即使 API 没发 finish_reason）
        sender.send(LlmEvent::Done).ok();
        Ok(())
    }
}

fn build_request_body(
    model: &str,
    messages: &[Message],
    tools: &[ToolDef],
    stream: bool,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": stream,
    });

    if !tools.is_empty() {
        let tool_defs: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            })
            .collect();
        body["tools"] = serde_json::Value::Array(tool_defs);
    }

    body
}

fn extract_response(json: serde_json::Value) -> Result<LlmResponse, LlmError> {
    let choice = json["choices"][0]
        .as_object()
        .ok_or_else(|| LlmError::Api("No choices in response".into()))?;

    let msg = choice["message"]
        .as_object()
        .ok_or_else(|| LlmError::Api("No message in choice".into()))?;

    let content = msg
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();

    let tool_calls = msg
        .get("tool_calls")
        .map(|tc| {
            tc.as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|tc| {
                            let name = tc["function"]["name"].as_str()?.to_string();
                            let arguments = tc["function"]["arguments"]
                                .as_str()
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or_default();
                            Some(ToolCall { name, arguments })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();

    Ok(LlmResponse { content, tool_calls })
}

/// 从 .env 环境变量创建 LLM provider
pub fn create_provider_from_env() -> Result<Box<dyn LlmProvider>, LlmError> {
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "deepseek".to_string());

    match provider.as_str() {
        "lmstudio" => {
            let base_url = std::env::var("LMSTUDIO_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:1234/v1".to_string());
            let api_key = std::env::var("LLM_API_KEY").unwrap_or_else(|_| "not-needed".to_string());
            let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "local-model".to_string());
            Ok(Box::new(OpenAiCompatible {
                base_url,
                api_key,
                model,
            }))
        }
        _ => {
            let api_key = std::env::var("LLM_API_KEY").map_err(|_| {
                LlmError::NoProvider
            })?;
            let model =
                std::env::var("LLM_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
            Ok(Box::new(OpenAiCompatible {
                base_url: "https://api.deepseek.com/v1".to_string(),
                api_key,
                model,
            }))
        }
    }
}
