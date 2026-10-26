use crate::proxy::mappers::claude::models::{ClaudeRequest, ContentBlock, MessageContent};
use crate::proxy::mappers::openai::models::{OpenAIContent, OpenAIContentBlock, OpenAIRequest};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// 会话管理器工具
pub struct SessionManager;

/// 清理用户消息中混入的系统提示词（如 Claude Code / Cursor / AtomCode 的 <system-reminder>、<environment>、[System...] 等）
/// 提取真正的用户输入内容，保证同一对话无论经过多少轮、携带什么动态时间戳，会话指纹都绝对稳定。
pub fn sanitize_user_text_for_fingerprint(raw: &str) -> String {
    let mut s = raw.to_string();

    // 1. 移除 <system-reminder>...</system-reminder> 标签及内部内容
    while let Some(start) = s.find("<system-reminder>") {
        if let Some(end) = s[start..].find("</system-reminder>") {
            s.replace_range(start..start + end + "</system-reminder>".len(), "");
        } else {
            s.truncate(start);
            break;
        }
    }

    // 2. 移除 <environment>...</environment> 标签及内部内容
    while let Some(start) = s.find("<environment>") {
        if let Some(end) = s[start..].find("</environment>") {
            s.replace_range(start..start + end + "</environment>".len(), "");
        } else {
            s.truncate(start);
            break;
        }
    }

    // 3. 移除 [System: ...] 或 [System ...]
    //    只有 "[System" 後緊接 ':'、空白或 ']' 才視為系統標籤；找不到收尾 ']' 時
    //    不截斷（避免吃掉整段使用者正文），也不誤刪 `[System.out.println(...)]` 這類程式碼。
    const SYSTEM_TAG: &str = "[System";
    let mut search_from = 0;
    while let Some(rel) = s[search_from..].find(SYSTEM_TAG) {
        let start = search_from + rel;
        let after = start + SYSTEM_TAG.len();
        let looks_like_tag = matches!(s[after..].chars().next(), Some(':' | ' ' | ']'));
        if looks_like_tag {
            if let Some(end_rel) = s[after..].find(']') {
                s.replace_range(start..after + end_rel + 1, "");
                search_from = start;
                continue;
            }
        }
        search_from = after;
    }

    s.trim().to_string()
}

/// 媒體引用的穩定摘要：data URL 只取 MIME 與長度；一般 URL 去掉 query/fragment
/// （簽名 URL 每次請求都會變，納入指紋會造成漂移）。
fn media_ref_summary(kind: &str, url: &str) -> String {
    if let Some(rest) = url.strip_prefix("data:") {
        let mime = rest.split([';', ',']).next().unwrap_or("");
        format!("[{}:{}:{}]", kind, mime, url.len())
    } else {
        let stable = url.split(['?', '#']).next().unwrap_or(url);
        format!("[{}:{}]", kind, stable)
    }
}

/// 會話指紋錨點：優先使用清理後的文字（與既有指紋相容）；沒有文字時退回多模態摘要，
/// 讓純圖片等首輪訊息也能在後續輪次保持同一錨點。
fn fingerprint_anchor(text: &str, media_summary: &str) -> Option<String> {
    let clean = sanitize_user_text_for_fingerprint(text);
    if !clean.trim().is_empty() {
        return Some(clean);
    }
    let media = media_summary.trim();
    (!media.is_empty()).then(|| media.to_string())
}

/// Claude 訊息的純文字部分
fn claude_content_text(content: &MessageContent) -> String {
    match content {
        MessageContent::String(s) => s.clone(),
        MessageContent::Array(blocks) => blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Claude 訊息的多模態摘要（不序列化 base64 本體，只記類型、MIME 與長度）
fn claude_content_media_summary(content: &MessageContent) -> String {
    let MessageContent::Array(blocks) = content else {
        return String::new();
    };
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Image { source, .. } => Some(format!(
                "[image:{}:{}]",
                source.media_type.as_deref().unwrap_or("image"),
                source.data.as_ref().map_or(0, |d| d.len())
            )),
            ContentBlock::Document { source, .. } => Some(format!(
                "[document:{}:{}]",
                source.media_type.as_deref().unwrap_or("document"),
                source.data.as_ref().map_or(0, |d| d.len())
            )),
            ContentBlock::ToolResult {
                tool_use_id,
                is_error,
                ..
            } => Some(format!(
                "[tool_result:{}:{}]",
                tool_use_id,
                is_error.unwrap_or(false)
            )),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// OpenAI 訊息的純文字部分
fn openai_content_text(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::String(s) => s.clone(),
        OpenAIContent::Array(blocks) => blocks
            .iter()
            .filter_map(|block| match block {
                OpenAIContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// OpenAI 訊息的多模態摘要
fn openai_content_media_summary(content: &OpenAIContent) -> String {
    let OpenAIContent::Array(blocks) = content else {
        return String::new();
    };
    blocks
        .iter()
        .filter_map(|block| match block {
            OpenAIContentBlock::Text { .. } => None,
            OpenAIContentBlock::ImageUrl { image_url } => {
                Some(media_ref_summary("image", &image_url.url))
            }
            OpenAIContentBlock::AudioUrl { audio_url } => {
                Some(media_ref_summary("audio", &audio_url.url))
            }
            OpenAIContentBlock::InputAudio { input_audio } => Some(format!(
                "[audio:{}:{}]",
                input_audio.format.as_deref().unwrap_or("audio"),
                input_audio.data.len()
            )),
            OpenAIContentBlock::VideoUrl { video_url } => {
                Some(media_ref_summary("video", &video_url.url))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Gemini parts 的純文字部分
fn gemini_parts_text(parts: &[Value]) -> String {
    parts
        .iter()
        .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Gemini parts 的多模態摘要（inlineData 只記 MIME 與長度，不序列化 base64）
fn gemini_parts_media_summary(parts: &[Value]) -> String {
    parts
        .iter()
        .filter_map(|part| {
            if part.get("text").is_some() {
                return None;
            }
            if let Some(inline) = part.get("inlineData").or_else(|| part.get("inline_data")) {
                let mime = inline
                    .get("mimeType")
                    .or_else(|| inline.get("mime_type"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("inline");
                let len = inline
                    .get("data")
                    .and_then(|d| d.as_str())
                    .map_or(0, |d| d.len());
                return Some(format!("[inlineData:{}:{}]", mime, len));
            }
            if let Some(file) = part.get("fileData").or_else(|| part.get("file_data")) {
                let uri = file
                    .get("fileUri")
                    .or_else(|| file.get("file_uri"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("file");
                return Some(media_ref_summary("fileData", uri));
            }
            if let Some(call) = part
                .get("functionCall")
                .or_else(|| part.get("function_call"))
            {
                let name = call.get("name").and_then(|n| n.as_str()).unwrap_or("func");
                return Some(format!("[functionCall:{}]", name));
            }
            if let Some(resp) = part
                .get("functionResponse")
                .or_else(|| part.get("function_response"))
            {
                let name = resp.get("name").and_then(|n| n.as_str()).unwrap_or("func");
                return Some(format!("[functionResponse:{}]", name));
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl SessionManager {
    /// 根据 Claude 请求生成稳定的会话指纹 (Session Fingerprint)
    ///
    /// 设计理念:
    /// - 只哈希第一条用户消息内容,不混入模型名称或时间戳
    /// - 确保同一对话的所有轮次使用相同的 session_id
    /// - 最大化 prompt caching 的命中率
    ///
    /// 优先级:
    /// 1. metadata.user_id (客户端显式提供)
    /// 2. 第一条用户消息的 SHA256 哈希
    pub fn extract_session_id(request: &ClaudeRequest) -> String {
        // 基于第一条用户消息的 SHA256 哈希作为会话锚点
        let mut hasher = Sha256::new();

        let mut content_found = false;
        for msg in &request.messages {
            if msg.role != "user" {
                continue;
            }

            if let Some(anchor) = fingerprint_anchor(
                &claude_content_text(&msg.content),
                &claude_content_media_summary(&msg.content),
            ) {
                hasher.update(anchor.as_bytes());
                content_found = true;
                break; // 始终锚定第一条有效用户消息
            }
        }

        if !content_found {
            // 兜底：没有任何有效 user 内容时，对最后一条消息的摘要（非 Debug 全序列化）进行哈希
            if let Some(last_msg) = request.messages.last() {
                hasher.update(claude_content_text(&last_msg.content).as_bytes());
                hasher.update(claude_content_media_summary(&last_msg.content).as_bytes());
            }
        }

        // [NEW] 融合多维环境特征 (System Prompt 完整摘要 + 可用 Tools 列表摘要)
        // 彻底解决多窗口首条消息完全相同（如均输入“你好”）的碰撞问题！
        hasher.update([0xff]);
        if let Some(sys) = &request.system {
            let sys_text = match sys {
                crate::proxy::mappers::claude::models::SystemPrompt::String(s) => s.clone(),
                crate::proxy::mappers::claude::models::SystemPrompt::Array(blocks) => blocks
                    .iter()
                    .map(|b| b.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            let clean_sys = sanitize_user_text_for_fingerprint(&sys_text);
            if !clean_sys.is_empty() {
                hasher.update(clean_sys.as_bytes());
            }
        }

        hasher.update([0xfe]);
        if let Some(tools) = &request.tools {
            for tool in tools {
                if let Some(name) = &tool.name {
                    hasher.update(name.as_bytes());
                    hasher.update([0xfd]);
                }
            }
        }

        let hash = format!("{:x}", hasher.finalize());
        let sid = format!("sid-{}", &hash[..16]);

        tracing::debug!(
            "[SessionManager] Generated session_id: {} (content_found: {}, model: {})",
            sid,
            content_found,
            request.model
        );
        sid
    }

    /// 根据 OpenAI 请求生成稳定的会话指纹
    pub fn extract_openai_session_id(request: &OpenAIRequest) -> String {
        if let Some(explicit) = request.session_id.as_ref() {
            let trimmed = explicit.trim();
            if !trimmed.is_empty() {
                return crate::proxy::thinking_store::sanitize_session_id(trimmed);
            }
        }
        let mut hasher = Sha256::new();

        let mut content_found = false;
        for msg in &request.messages {
            if msg.role != "user" {
                continue;
            }
            if let Some(content) = &msg.content {
                if let Some(anchor) = fingerprint_anchor(
                    &openai_content_text(content),
                    &openai_content_media_summary(content),
                ) {
                    hasher.update(anchor.as_bytes());
                    content_found = true;
                    break;
                }
            }
        }

        if !content_found {
            // 兜底：对最后一条消息的摘要（非 Debug 全序列化）进行哈希
            if let Some(content) = request.messages.last().and_then(|m| m.content.as_ref()) {
                hasher.update(openai_content_text(content).as_bytes());
                hasher.update(openai_content_media_summary(content).as_bytes());
            }
        }

        // [NEW] 融合多维环境特征 (System/Developer 消息摘要 + Tools 列表摘要)
        hasher.update([0xff]);
        for msg in &request.messages {
            if msg.role == "system" || msg.role == "developer" {
                if let Some(content) = &msg.content {
                    let text = match content {
                        OpenAIContent::String(s) => s.clone(),
                        OpenAIContent::Array(blocks) => blocks
                            .iter()
                            .filter_map(|block| {
                                match block {
                                crate::proxy::mappers::openai::models::OpenAIContentBlock::Text {
                                    text,
                                } => Some(text.as_str()),
                                _ => None,
                            }
                            })
                            .collect::<Vec<_>>()
                            .join(" "),
                    };
                    let clean_sys = sanitize_user_text_for_fingerprint(&text);
                    if !clean_sys.is_empty() {
                        hasher.update(clean_sys.as_bytes());
                        break;
                    }
                }
            }
        }

        hasher.update([0xfe]);
        if let Some(tools) = &request.tools {
            for tool in tools {
                let name = tool
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .or_else(|| tool.get("name").and_then(|n| n.as_str()));
                if let Some(name) = name {
                    hasher.update(name.as_bytes());
                    hasher.update([0xfd]);
                }
            }
        }

        let hash = format!("{:x}", hasher.finalize());
        let sid = format!("sid-{}", &hash[..16]);
        tracing::debug!("[SessionManager-OpenAI] Generated fingerprint: {}", sid);
        sid
    }

    /// 根据 Gemini 原生请求 (JSON) 生成稳定的会话指纹
    pub fn extract_gemini_session_id(request: &Value, _model_name: &str) -> String {
        if let Some(explicit) = request.get("session_id").and_then(|v| v.as_str()) {
            let trimmed = explicit.trim();
            if !trimmed.is_empty() {
                return crate::proxy::thinking_store::sanitize_session_id(trimmed);
            }
        }
        let mut hasher = Sha256::new();

        let mut content_found = false;
        if let Some(contents) = request.get("contents").and_then(|v| v.as_array()) {
            for content in contents {
                if content.get("role").and_then(|v| v.as_str()) != Some("user") {
                    continue;
                }

                if let Some(parts) = content.get("parts").and_then(|v| v.as_array()) {
                    if let Some(anchor) = fingerprint_anchor(
                        &gemini_parts_text(parts),
                        &gemini_parts_media_summary(parts),
                    ) {
                        hasher.update(anchor.as_bytes());
                        content_found = true;
                        break;
                    }
                }
            }
        }

        if !content_found {
            // 兜底：不再对整个 Body 做哈希（多轮对话每轮都会变）；
            // 改取最后一个 content 的 parts 摘要。
            if let Some(parts) = request
                .get("contents")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.last())
                .and_then(|c| c.get("parts"))
                .and_then(|v| v.as_array())
            {
                hasher.update(gemini_parts_text(parts).as_bytes());
                hasher.update(gemini_parts_media_summary(parts).as_bytes());
            }
        }

        // [NEW] 融合多维环境特征 (systemInstruction 摘要 + tools 列表摘要)
        hasher.update([0xff]);
        if let Some(sys_inst) = request
            .get("system_instruction")
            .or_else(|| request.get("systemInstruction"))
        {
            if let Some(parts) = sys_inst.get("parts").and_then(|p| p.as_array()) {
                let mut sys_texts = Vec::new();
                for part in parts {
                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                        sys_texts.push(text);
                    }
                }
                let clean_sys = sanitize_user_text_for_fingerprint(&sys_texts.join(" "));
                if !clean_sys.is_empty() {
                    hasher.update(clean_sys.as_bytes());
                }
            }
        }

        hasher.update([0xfe]);
        if let Some(tools) = request.get("tools").and_then(|t| t.as_array()) {
            for tool in tools {
                if let Some(decls) = tool.get("functionDeclarations").and_then(|d| d.as_array()) {
                    for decl in decls {
                        if let Some(name) = decl.get("name").and_then(|n| n.as_str()) {
                            hasher.update(name.as_bytes());
                            hasher.update([0xfd]);
                        }
                    }
                }
            }
        }

        let hash = format!("{:x}", hasher.finalize());
        let sid = format!("sid-{}", &hash[..16]);
        tracing::debug!("[SessionManager-Gemini] Generated fingerprint: {}", sid);
        sid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::mappers::claude::models::{Message, SystemPrompt, Tool};

    #[test]
    fn test_sanitize_user_text_for_fingerprint() {
        let text1 =
            "你好啊\n\n<system-reminder>\nCurrent date: 2026-09-08 (Tue)\n</system-reminder>";
        let text2 =
            "你好啊\n\n<system-reminder>\nCurrent date: 2026-09-09 (Wed)\n</system-reminder>";
        let text3 = "你好啊 [System: Tool execution completed successfully]";

        assert_eq!(sanitize_user_text_for_fingerprint(text1), "你好啊");
        assert_eq!(sanitize_user_text_for_fingerprint(text2), "你好啊");
        assert_eq!(sanitize_user_text_for_fingerprint(text3), "你好啊");
    }

    #[test]
    fn test_same_user_text_different_system_prompt_isolated() {
        let req_project_a = ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::String("你好".to_string()),
            }],
            system: Some(SystemPrompt::String(
                "Project A Workspace: /src/backend".to_string(),
            )),
            tools: None,
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        };

        let req_project_b = ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::String("你好".to_string()),
            }],
            system: Some(SystemPrompt::String(
                "Project B Workspace: /src/frontend".to_string(),
            )),
            tools: None,
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        };

        let sid_a = SessionManager::extract_session_id(&req_project_a);
        let sid_b = SessionManager::extract_session_id(&req_project_b);

        // Even though both messages are "你好", different workspace/system instructions yield completely isolated sessions!
        assert_ne!(sid_a, sid_b);
    }

    #[test]
    fn test_same_user_text_different_tools_isolated() {
        let req_with_tools = ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::String("你好".to_string()),
            }],
            system: None,
            tools: Some(vec![Tool {
                name: Some("bash".to_string()),
                description: None,
                input_schema: None,
                type_: None,
            }]),
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        };

        let req_pure_chat = ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::String("你好".to_string()),
            }],
            system: None,
            tools: None,
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        };

        let sid_tools = SessionManager::extract_session_id(&req_with_tools);
        let sid_chat = SessionManager::extract_session_id(&req_pure_chat);

        assert_ne!(sid_tools, sid_chat);
    }

    #[test]
    fn test_same_conversation_multi_turn_stability() {
        let req_turn1 = ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::String("你好".to_string()),
            }],
            system: Some(SystemPrompt::String("Project A".to_string())),
            tools: None,
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        };

        let mut req_turn2 = req_turn1.clone();
        req_turn2.messages.push(Message {
            role: "assistant".to_string(),
            content: MessageContent::String("你好！请问有什么可以帮助你？".to_string()),
        });
        req_turn2.messages.push(Message {
            role: "user".to_string(),
            content: MessageContent::String("写个斐波那契数列".to_string()),
        });

        let sid1 = SessionManager::extract_session_id(&req_turn1);
        let sid2 = SessionManager::extract_session_id(&req_turn2);

        // Within the same conversation, multi-turn session ID remains 100% stable!
        assert_eq!(sid1, sid2);
    }

    #[test]
    fn test_utf8_char_boundary_at_512_bytes_never_panics() {
        // Construct a system prompt where byte index 512 lands exactly inside a 3-byte Chinese character '单' (bytes 511..514)
        let prefix = "a".repeat(511);
        let malicious_sys = format!("{}单清单清单", prefix);
        assert!(
            !malicious_sys.is_char_boundary(512),
            "Byte 512 must be inside '单' to test the regression"
        );

        // 1. Claude Request
        let claude_req = ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::String("你好".to_string()),
            }],
            system: Some(SystemPrompt::String(malicious_sys.clone())),
            tools: None,
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        };
        let sid_claude = SessionManager::extract_session_id(&claude_req);
        assert!(sid_claude.starts_with("sid-"));

        // 2. OpenAI Request
        let openai_req: OpenAIRequest = serde_json::from_value(serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                { "role": "system", "content": malicious_sys },
                { "role": "user", "content": "你好" }
            ]
        }))
        .unwrap();
        let sid_openai = SessionManager::extract_openai_session_id(&openai_req);
        assert!(sid_openai.starts_with("sid-"));

        // 3. Gemini Native Request
        let gemini_req = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [{ "text": "你好" }]
            }],
            "system_instruction": {
                "parts": [{ "text": malicious_sys }]
            }
        });
        let sid_gemini = SessionManager::extract_gemini_session_id(&gemini_req, "gemini-2.5-pro");
        assert!(sid_gemini.starts_with("sid-"));

        // 4. Real User Prompt reported in issue
        let user_prompt =
            "你是一个远程服务器运维专家。\n\n当前纳管的 LXC 容器清单如下....".repeat(20);
        let claude_user_req = ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::String("你好".to_string()),
            }],
            system: Some(SystemPrompt::String(user_prompt)),
            tools: None,
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        };
        let sid_real = SessionManager::extract_session_id(&claude_user_req);
        assert!(sid_real.starts_with("sid-"));
    }

    fn task_claude_req(messages: Vec<Message>) -> ClaudeRequest {
        ClaudeRequest {
            model: "claude-3-7-sonnet".to_string(),
            messages,
            system: None,
            tools: None,
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            thinking: None,
            metadata: None,
            output_config: None,
            size: None,
            quality: None,
        }
    }

    fn task_user(text: &str) -> Message {
        Message {
            role: "user".to_string(),
            content: MessageContent::String(text.to_string()),
        }
    }

    fn task_assistant(text: &str) -> Message {
        Message {
            role: "assistant".to_string(),
            content: MessageContent::String(text.to_string()),
        }
    }

    fn task_claude_image(data: &str) -> Message {
        use crate::proxy::mappers::claude::models::ImageSource;
        Message {
            role: "user".to_string(),
            content: MessageContent::Array(vec![ContentBlock::Image {
                source: ImageSource {
                    source_type: "base64".to_string(),
                    media_type: Some("image/png".to_string()),
                    data: Some(data.to_string()),
                },
                cache_control: None,
            }]),
        }
    }

    fn task_openai_req(messages: serde_json::Value) -> OpenAIRequest {
        serde_json::from_value(serde_json::json!({
            "model": "gpt-4o",
            "messages": messages
        }))
        .unwrap()
    }

    #[test]
    fn task_unclosed_system_tag_and_code_preserved() {
        assert_eq!(
            sanitize_user_text_for_fingerprint("Hello [System architecture and concurrency design"),
            "Hello [System architecture and concurrency design"
        );
        assert_eq!(
            sanitize_user_text_for_fingerprint("Please inspect [System.out.println(\"x\")] now"),
            "Please inspect [System.out.println(\"x\")] now"
        );
        assert_eq!(
            sanitize_user_text_for_fingerprint("Hello [System: Tool execution ok] world"),
            "Hello  world"
        );
        // Multi-byte text around the tag must not panic or be cut.
        assert_eq!(
            sanitize_user_text_for_fingerprint("你好 [System] 世界 [System"),
            "你好  世界 [System"
        );
    }

    #[test]
    fn task_single_char_first_message_is_a_stable_anchor() {
        // Claude
        let t1 = task_claude_req(vec![task_user("1")]);
        let t2 = task_claude_req(vec![
            task_user("1"),
            task_assistant("You selected option 1"),
            task_user("Next step"),
        ]);
        assert_eq!(
            SessionManager::extract_session_id(&t1),
            SessionManager::extract_session_id(&t2)
        );

        // OpenAI
        let t1 = task_openai_req(serde_json::json!([{ "role": "user", "content": "y" }]));
        let t2 = task_openai_req(serde_json::json!([
            { "role": "user", "content": "y" },
            { "role": "assistant", "content": "ok" },
            { "role": "user", "content": "continue" }
        ]));
        assert_eq!(
            SessionManager::extract_openai_session_id(&t1),
            SessionManager::extract_openai_session_id(&t2)
        );

        // Gemini
        let t1 = serde_json::json!({
            "contents": [{ "role": "user", "parts": [{ "text": "1" }] }]
        });
        let t2 = serde_json::json!({
            "contents": [
                { "role": "user", "parts": [{ "text": "1" }] },
                { "role": "model", "parts": [{ "text": "You chose 1" }] },
                { "role": "user", "parts": [{ "text": "Continue" }] }
            ]
        });
        assert_eq!(
            SessionManager::extract_gemini_session_id(&t1, "gemini-2.5-pro"),
            SessionManager::extract_gemini_session_id(&t2, "gemini-2.5-pro")
        );
    }

    const TASK_PNG_B64: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    #[test]
    fn task_image_only_first_message_is_a_stable_anchor() {
        // Claude: image-only first turn, text follow-up on the second turn.
        let t1 = task_claude_req(vec![task_claude_image(TASK_PNG_B64)]);
        let t2 = task_claude_req(vec![
            task_claude_image(TASK_PNG_B64),
            task_assistant("I see the image"),
            task_user("What color is it?"),
        ]);
        assert_eq!(
            SessionManager::extract_session_id(&t1),
            SessionManager::extract_session_id(&t2)
        );

        // OpenAI: data URL image-only first turn.
        let image_msg = serde_json::json!({
            "role": "user",
            "content": [{
                "type": "image_url",
                "image_url": { "url": format!("data:image/png;base64,{}", TASK_PNG_B64) }
            }]
        });
        let t1 = task_openai_req(serde_json::json!([image_msg.clone()]));
        let t2 = task_openai_req(serde_json::json!([
            image_msg,
            { "role": "assistant", "content": "I see it" },
            { "role": "user", "content": "Tell me more" }
        ]));
        assert_eq!(
            SessionManager::extract_openai_session_id(&t1),
            SessionManager::extract_openai_session_id(&t2)
        );

        // Gemini: inlineData image-only first turn.
        let image_content = serde_json::json!({
            "role": "user",
            "parts": [{ "inlineData": { "mimeType": "image/png", "data": TASK_PNG_B64 } }]
        });
        let t1 = serde_json::json!({ "contents": [image_content.clone()] });
        let t2 = serde_json::json!({
            "contents": [
                image_content,
                { "role": "model", "parts": [{ "text": "I see a pixel" }] },
                { "role": "user", "parts": [{ "text": "Tell me more" }] }
            ]
        });
        assert_eq!(
            SessionManager::extract_gemini_session_id(&t1, "gemini-2.5-pro"),
            SessionManager::extract_gemini_session_id(&t2, "gemini-2.5-pro")
        );
    }

    #[test]
    fn task_media_is_summarized_not_serialized() {
        // Two payloads with the same type and length but different bytes map to the
        // same summary, which proves the base64 body is not what gets hashed.
        let a = "A".repeat(4096);
        let b = "B".repeat(4096);
        let sid_a =
            SessionManager::extract_session_id(&task_claude_req(vec![task_claude_image(&a)]));
        let sid_b =
            SessionManager::extract_session_id(&task_claude_req(vec![task_claude_image(&b)]));
        assert_eq!(sid_a, sid_b);

        // A different payload length is a different anchor.
        let c = "A".repeat(8192);
        let sid_c =
            SessionManager::extract_session_id(&task_claude_req(vec![task_claude_image(&c)]));
        assert_ne!(sid_a, sid_c);

        let parts = vec![serde_json::json!({
            "inlineData": { "mimeType": "image/png", "data": a }
        })];
        let summary = gemini_parts_media_summary(&parts);
        assert_eq!(summary, "[inlineData:image/png:4096]");
        assert!(!summary.contains("AAAA"));
    }

    #[test]
    fn task_signed_media_url_query_does_not_change_anchor() {
        let req = |url: &str| {
            task_openai_req(serde_json::json!([{
                "role": "user",
                "content": [{ "type": "image_url", "image_url": { "url": url } }]
            }]))
        };
        let a = SessionManager::extract_openai_session_id(&req(
            "https://cdn.example.com/a.png?X-Signature=aaa&Expires=1",
        ));
        let b = SessionManager::extract_openai_session_id(&req(
            "https://cdn.example.com/a.png?X-Signature=bbb&Expires=2",
        ));
        let c = SessionManager::extract_openai_session_id(&req(
            "https://cdn.example.com/other.png?X-Signature=aaa&Expires=1",
        ));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn task_text_anchor_is_unchanged_by_attached_media() {
        // Existing fingerprints for text messages must not move, with or without an image.
        let text_only = task_claude_req(vec![task_user("你好")]);
        let with_image = task_claude_req(vec![Message {
            role: "user".to_string(),
            content: MessageContent::Array(vec![
                ContentBlock::Text {
                    text: "你好".to_string(),
                },
                ContentBlock::Image {
                    source: crate::proxy::mappers::claude::models::ImageSource {
                        source_type: "base64".to_string(),
                        media_type: Some("image/png".to_string()),
                        data: Some(TASK_PNG_B64.to_string()),
                    },
                    cache_control: None,
                },
            ]),
        }]);
        let sid = SessionManager::extract_session_id(&text_only);
        assert_eq!(sid, SessionManager::extract_session_id(&with_image));
        // Value computed independently: sha256("你好" 0xff 0xfe) first 16 hex chars.
        assert_eq!(sid, "sid-3850c860ffb923cf");
    }

    #[test]
    fn task_gemini_fallback_ignores_unrelated_body_fields() {
        // No user content at all: the fingerprint must not depend on the whole request body.
        let base = serde_json::json!({
            "contents": [{ "role": "model", "parts": [{ "text": "hello" }] }]
        });
        let with_config = serde_json::json!({
            "contents": [{ "role": "model", "parts": [{ "text": "hello" }] }],
            "generationConfig": { "temperature": 0.3 }
        });
        assert_eq!(
            SessionManager::extract_gemini_session_id(&base, "gemini-2.5-pro"),
            SessionManager::extract_gemini_session_id(&with_config, "gemini-2.5-pro")
        );
    }
}
