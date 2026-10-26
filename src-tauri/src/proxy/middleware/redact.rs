//! Redaction of credentials that clients put in the URL query string.
//!
//! The Google REST convention allows `?key=<API key>`. The proxy accepts it (see
//! `auth::extract_api_key`), so every place that stores or prints a request URI
//! (monitor database, security access log, ...) must pass it through
//! [`redact_query_secret`] first.

/// Query parameter names whose values are credentials (compared case-insensitively
/// after percent-decoding).
const SENSITIVE_QUERY_PARAMS: &[&str] = &[
    "key",
    "api_key",
    "apikey",
    "api-key",
    "x-goog-api-key",
    "access_token",
    "token",
];

fn is_sensitive_query_name(raw_name: &str) -> bool {
    let decoded = url::form_urlencoded::parse(raw_name.as_bytes())
        .next()
        .map(|(name, _)| name.into_owned())
        .unwrap_or_default();
    SENSITIVE_QUERY_PARAMS
        .iter()
        .any(|s| decoded.eq_ignore_ascii_case(s))
}

/// Replace the value of credential-bearing query parameters with `***`.
///
/// Accepts a path-and-query, an absolute URI, or a bare path. Everything else
/// (path, other parameters, parameter order, fragment) is left untouched.
pub fn redact_query_secret(uri: &str) -> String {
    let Some((base, rest)) = uri.split_once('?') else {
        return uri.to_string();
    };
    let (query, fragment) = match rest.split_once('#') {
        Some((q, f)) => (q, Some(f)),
        None => (rest, None),
    };

    let redacted = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, _)) if is_sensitive_query_name(name) => format!("{}=***", name),
            _ => pair.to_string(),
        })
        .collect::<Vec<_>>()
        .join("&");

    match fragment {
        Some(f) => format!("{}?{}#{}", base, redacted, f),
        None => format!("{}?{}", base, redacted),
    }
}

#[cfg(test)]
mod tests {
    use super::redact_query_secret;

    #[test]
    fn task_redact_gemini_key_param() {
        assert_eq!(
            redact_query_secret("/v1beta/models/x:streamGenerateContent?alt=sse&key=abc"),
            "/v1beta/models/x:streamGenerateContent?alt=sse&key=***"
        );
        assert_eq!(
            redact_query_secret("/v1beta/models?key=AIzaSecret&pageSize=5"),
            "/v1beta/models?key=***&pageSize=5"
        );
    }

    #[test]
    fn task_redact_leaves_clean_uris_alone() {
        for uri in [
            "/v1/chat/completions",
            "/v1beta/models?alt=sse",
            "/v1beta/models?alt=sse&flag",
            "/v1beta/models?",
            "/v1beta/models?key",
            "/v1beta/models?monkey=banana",
            "/v1beta/models/gemini-2.5-flash:generateContent",
        ] {
            assert_eq!(redact_query_secret(uri), uri, "uri {uri}");
        }
    }

    #[test]
    fn task_redact_variants_and_encodings() {
        assert_eq!(
            redact_query_secret("/p?API_KEY=s1&Key=s2&access_token=s3&token=s4&a=1"),
            "/p?API_KEY=***&Key=***&access_token=***&token=***&a=1"
        );
        // Percent-encoded parameter name must not bypass redaction.
        assert_eq!(redact_query_secret("/p?%6bey=secret"), "/p?%6bey=***");
        // Empty value and repeated parameter.
        assert_eq!(redact_query_secret("/p?key=&key=two"), "/p?key=***&key=***");
        // Absolute URI and fragment.
        assert_eq!(
            redact_query_secret("http://127.0.0.1:8045/v1beta/models?key=abc#frag"),
            "http://127.0.0.1:8045/v1beta/models?key=***#frag"
        );
    }

    #[test]
    fn task_redact_secret_value_never_survives() {
        let secret = "AIzaSyDoNotLeak-1234567890";
        for (index, uri) in [
            format!("/v1beta/models?key={secret}"),
            format!("/v1beta/models/m:generateContent?alt=sse&key={secret}"),
            format!("/v1beta/models?x=1&key={secret}&y=2"),
            format!("http://h/v1beta/models?key={secret}#f"),
        ]
        .iter()
        .enumerate()
        {
            // Index only: the failure message must not echo the secret-bearing URI.
            assert!(
                !redact_query_secret(uri).contains(secret),
                "secret survived redaction in case {index}"
            );
        }
    }
}
