use serde::{Deserialize, Serialize};

/// 代理所接入与服务的客户端协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProxyProtocol {
    OpenAIChat,
    OpenAIResponses,
    AnthropicClaude,
    GeminiNative,
}

impl ProxyProtocol {
    /// 该协议是否信任客户端回传的思考签名
    /// - OpenAIChat: false (官方规范无签名概念，客户端若夹带也属于不可信，入站统一擦除，由服务端全权参与回填)
    /// - OpenAIResponses / AnthropicClaude / GeminiNative: true (协议原生支持签名，校验长度与兼容性后采纳)
    pub fn trusts_client_signature(&self) -> bool {
        match self {
            ProxyProtocol::OpenAIChat => false,
            ProxyProtocol::OpenAIResponses
            | ProxyProtocol::AnthropicClaude
            | ProxyProtocol::GeminiNative => true,
        }
    }
}

/// 上游响应与错误在统一流水线中的唯一判定分类
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpstreamClassification {
    /// 真正的上游配额耗尽/速率限制（仅限 429 或 529）
    RateLimited { retry_after: Option<u64> },
    /// 模型不存在或不支持（404 或 模型不存在文本错误）
    ModelNotFound,
    /// 瞬时服务器故障（500/503等，属于服务故障，只换号/退避，绝不打入账号冷却池）
    TransientServerError,
    /// 网关内部生成的自产生错误（如 All accounts limited / Token pool is empty 等，绝不自噬锁定）
    InternalGatewayMessage,
    /// 思考签名失效或跨模型异构签名污染错误（400 Invalid signature in thinking block）
    ThoughtSignatureError,
    /// 上游多模态图像解码或处理失败错误（400 Unable to process input image 等）。
    /// 不是账号问题：不锁定账号、不换号，剔除图片后在同一账号原地重试。
    ImageProcessingError,
    /// 其他普通客户端错误
    OtherClientError(u16),
}

impl UpstreamClassification {
    /// 统一分类器：协议无关，全局唯一真理
    pub fn classify(status: u16, body: &str, retry_after_header: Option<&str>) -> Self {
        let lower = body.to_lowercase();
        // 1. 优先排除网关自身内部错误（防自噬死循环）
        if lower.contains("all accounts limited")
            || lower.contains("no accounts available")
            || lower.contains("all accounts failed")
            || lower.contains("token pool is empty")
            || lower.contains("all accounts exhausted")
            || lower.contains("all accounts unhealthy")
        {
            return UpstreamClassification::InternalGatewayMessage;
        }

        // 2. 判定模型不存在 (广义模式匹配：无论 status 是 404, 400 还是上游服务端抛出的 500/503)
        let has_model_not_found_cue = lower.contains("model not found")
            || lower.contains("unknown model")
            || lower.contains("does not exist")
            || lower.contains("is not found")
            || lower.contains("unsupported model")
            || lower.contains("not found for api version")
            || lower.contains("publisher model")
            || lower.contains("model_not_found")
            || lower.contains("no such model")
            || lower.contains("invalid model")
            || lower.contains("model is not available");

        if status == 404 || has_model_not_found_cue {
            return UpstreamClassification::ModelNotFound;
        }

        // 3. 判定 Thinking 签名失效或跨模型异构签名污染
        let has_signature_error_cue = lower.contains("invalid thought signature")
            || lower.contains("invalid `signature`")
            || lower.contains("invalid signature")
            || lower.contains("thought_signature")
            || lower.contains("thoughtsignature")
            || lower.contains("thinking.signature")
            || lower.contains("thinking.thinking")
            || lower.contains("corrupted thought signature");

        if status == 400 && has_signature_error_cue {
            return UpstreamClassification::ThoughtSignatureError;
        }

        // 4. 判定上游图像解码/处理失败 (400 Unable to process input image)
        if Self::is_image_processing_error_str(status, body) {
            return UpstreamClassification::ImageProcessingError;
        }

        // 5. 判定真正的限流 (仅限 429 和 529)
        if status == 429 || status == 529 {
            let delay = crate::proxy::upstream::retry::parse_retry_delay(body, retry_after_header)
                .map(|ms| ms.saturating_add(999) / 1000);
            return UpstreamClassification::RateLimited { retry_after: delay };
        }

        // 6. 瞬时故障 (500 / 503 等，绝非账号限流)
        if status == 500 || status == 503 {
            return UpstreamClassification::TransientServerError;
        }

        UpstreamClassification::OtherClientError(status)
    }

    /// 该分类是否应该触发 TokenManager 的账号冷却锁定
    pub fn should_lock_account(&self) -> bool {
        matches!(self, UpstreamClassification::RateLimited { .. })
    }

    /// 该分类是否为模型不存在
    pub fn is_model_not_found(&self) -> bool {
        matches!(self, UpstreamClassification::ModelNotFound)
    }

    /// 该分类是否为思考签名失效/异构签名污染
    pub fn is_thought_signature_error(&self) -> bool {
        matches!(self, UpstreamClassification::ThoughtSignatureError)
    }

    /// 该分类是否为上游图像解码/处理失败
    pub fn is_image_processing_error(&self) -> bool {
        matches!(self, UpstreamClassification::ImageProcessingError)
    }

    /// 该分类是否为网关自身内部消息
    pub fn is_internal_gateway_message(&self) -> bool {
        matches!(self, UpstreamClassification::InternalGatewayMessage)
    }

    /// 判断是否为上游多模态图像解码或处理错误 (400 "Unable to process input image")。
    /// 这是该判定的唯一来源：`classify` 与任何需要字串判定的呼叫端都走这里，避免两份线索清单漂移。
    pub fn is_image_processing_error_str(status: u16, body: &str) -> bool {
        if status != 400 {
            return false;
        }
        let lower = body.to_lowercase();
        lower.contains("unable to process input image")
            || lower.contains("failed to process input image")
            || lower.contains("cannot process input image")
            || lower.contains("could not process image")
            || lower.contains("error processing input image")
            || lower.contains("invalid input image")
            || lower.contains("unable to decode image")
            || lower.contains("unsupported image format")
            || lower.contains("image decoding failed")
            || (lower.contains("process input image")
                && lower.contains("developers.generativeai.google"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOGLE_IMAGE_400: &str = r#"API Error: 400 Unable to process input image. Please retry or report in https://developers.generativeai.google/guide/troubleshooting"#;

    #[test]
    fn task_image_processing_error_classification() {
        let classification = UpstreamClassification::classify(400, GOOGLE_IMAGE_400, None);
        assert_eq!(classification, UpstreamClassification::ImageProcessingError);
        assert!(classification.is_image_processing_error());
        // 坏图不是账号的错：不得进入账号冷却池
        assert!(!classification.should_lock_account());
        assert!(!classification.is_model_not_found());
        assert!(!classification.is_thought_signature_error());
    }

    #[test]
    fn task_image_processing_error_is_gated_on_status_400() {
        // 同样的文字出现在非 400 状态码不能被当成坏图（例如 429 / 500 的包装文字）
        for status in [200u16, 429, 500, 503] {
            assert!(
                !UpstreamClassification::is_image_processing_error_str(status, GOOGLE_IMAGE_400),
                "status {status} must not be classified as an image error"
            );
        }
        assert_eq!(
            UpstreamClassification::classify(429, GOOGLE_IMAGE_400, None),
            UpstreamClassification::RateLimited { retry_after: None }
        );
        assert_eq!(
            UpstreamClassification::classify(500, GOOGLE_IMAGE_400, None),
            UpstreamClassification::TransientServerError
        );
    }

    #[test]
    fn task_image_processing_error_cue_variants_and_negatives() {
        assert!(UpstreamClassification::is_image_processing_error_str(
            400,
            r#"{"error":{"message":"Failed to process input image due to decoding error"}}"#
        ));
        assert!(UpstreamClassification::is_image_processing_error_str(
            400,
            "UNSUPPORTED IMAGE FORMAT"
        ));
        // 无关的 400 不得被误判为坏图，否则会无谓地剔除用户图片
        assert!(!UpstreamClassification::is_image_processing_error_str(
            400,
            "Invalid signature in thinking block"
        ));
        assert!(!UpstreamClassification::is_image_processing_error_str(
            400,
            "Normal 400 bad request"
        ));
    }

    #[test]
    fn task_image_processing_error_does_not_shadow_signature_or_model_errors() {
        assert_eq!(
            UpstreamClassification::classify(400, "Invalid signature in thinking block", None),
            UpstreamClassification::ThoughtSignatureError
        );
        assert_eq!(
            UpstreamClassification::classify(404, "models/foo is not found", None),
            UpstreamClassification::ModelNotFound
        );
    }
}
