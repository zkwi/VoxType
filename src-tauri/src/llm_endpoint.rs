pub fn chat_completions_endpoint(configured_url: &str) -> String {
    let url = configured_url.trim().trim_end_matches('/');
    if url.ends_with("/chat/completions") {
        return url.to_string();
    }
    if has_no_path(url) {
        return format!("{}/v1/chat/completions", url);
    }
    format!("{}/chat/completions", url)
}

/// 归一化模型列表接口地址，用于自动探测服务商声明的模型思考能力。
pub fn models_endpoint(configured_url: &str) -> String {
    let url = configured_url.trim().trim_end_matches('/');
    if let Some(prefix) = url.strip_suffix("/chat/completions") {
        return format!("{}/models", prefix.trim_end_matches('/'));
    }
    if url.ends_with("/models") {
        return url.to_string();
    }
    if has_no_path(url) {
        return format!("{}/v1/models", url);
    }
    format!("{}/models", url)
}

fn has_no_path(url: &str) -> bool {
    let Some((_, rest)) = url.split_once("://") else {
        return !url.contains('/');
    };
    !rest.contains('/')
}

#[cfg(test)]
mod tests {
    use super::{chat_completions_endpoint, models_endpoint};

    #[test]
    fn normalizes_deepseek_equivalent_urls() {
        let expected = "https://api.deepseek.com/v1/chat/completions";

        assert_eq!(
            chat_completions_endpoint("https://api.deepseek.com"),
            expected
        );
        assert_eq!(
            chat_completions_endpoint("https://api.deepseek.com/v1/"),
            expected
        );
        assert_eq!(
            chat_completions_endpoint("https://api.deepseek.com/v1/chat/completions"),
            expected
        );
    }

    #[test]
    fn keeps_provider_specific_v1_paths() {
        assert_eq!(
            chat_completions_endpoint("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_endpoint("https://openrouter.ai/api/v1"),
            "https://openrouter.ai/api/v1/chat/completions"
        );
    }

    #[test]
    fn normalizes_models_endpoint_across_base_url_forms() {
        assert_eq!(
            models_endpoint("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/models"
        );
        assert_eq!(
            models_endpoint("https://api.deepseek.com/v1/"),
            "https://api.deepseek.com/v1/models"
        );
        assert_eq!(
            models_endpoint("https://api.deepseek.com/v1/chat/completions"),
            "https://api.deepseek.com/v1/models"
        );
        assert_eq!(
            models_endpoint("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            "https://dashscope.aliyuncs.com/compatible-mode/v1/models"
        );
        assert_eq!(
            models_endpoint("https://openrouter.ai/api/v1"),
            "https://openrouter.ai/api/v1/models"
        );
        assert_eq!(
            models_endpoint("http://127.0.0.1:3425"),
            "http://127.0.0.1:3425/v1/models"
        );
        assert_eq!(
            models_endpoint("http://127.0.0.1:3425/v1"),
            "http://127.0.0.1:3425/v1/models"
        );
        assert_eq!(
            models_endpoint("http://127.0.0.1:3425/v1/models/"),
            "http://127.0.0.1:3425/v1/models"
        );
    }
}
