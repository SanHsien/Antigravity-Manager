// Fingerprint blacklist for images the upstream vision decoder refused.
//
// Why this exists: when Google answers `400 Unable to process input image`, a client that
// speaks a stateless chat protocol (Claude Code, Cursor, LibreChat, ...) replays the whole
// history -- bad image included -- on its next turn. Without a guard every later turn of the
// conversation dies with the same 400.
//
// How it is wired (see the handlers for the retry side):
//   1. A handler classifies the 400 as `UpstreamClassification::ImageProcessingError`.
//   2. It calls `record_corrupted_images_in_gemini_payload` with the payload it just sent.
//   3. It retries on the same account (no lockout, no rotation, prefix cache preserved).
//   4. `validate_and_sanitize_inline_data` consults `is_corrupted_image`, and the single
//      outgoing choke point (`UpstreamClient::call_v1_internal_with_headers`) runs that
//      sanitizer on every payload, so the retry -- and every later replay -- goes out with a
//      text placeholder instead of the image, for all four client protocols at once.
//
// The upstream error does not say WHICH image is bad, so every image in the failed payload is
// fingerprinted. That is deliberate: the alternative is bisecting with extra requests.
//
// This module depends only on serde_json / sha2 / once_cell / parking_lot so its logic can be
// unit-tested without building the rest of the application.

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};

/// Upper bound on remembered fingerprints. When full the OLDEST entry is evicted, so a busy
/// gateway keeps recent immunity instead of losing all of it at once.
pub const MAX_FINGERPRINTS: usize = 2000;

/// Insertion-ordered, capacity-bounded set of fingerprints.
#[derive(Debug, Default)]
struct FingerprintSet {
    seen: HashSet<String>,
    order: VecDeque<String>,
}

impl FingerprintSet {
    fn contains(&self, fingerprint: &str) -> bool {
        self.seen.contains(fingerprint)
    }

    fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.seen.len()
    }

    /// Returns true when the fingerprint was not already present.
    fn insert(&mut self, fingerprint: String, capacity: usize) -> bool {
        if self.seen.contains(&fingerprint) {
            return false;
        }
        while self.seen.len() >= capacity.max(1) {
            match self.order.pop_front() {
                Some(oldest) => {
                    self.seen.remove(&oldest);
                }
                None => break,
            }
        }
        self.order.push_back(fingerprint.clone());
        self.seen.insert(fingerprint);
        true
    }
}

static CORRUPTED_IMAGES: Lazy<Mutex<FingerprintSet>> =
    Lazy::new(|| Mutex::new(FingerprintSet::default()));

/// SHA-256 fingerprint of a base64 image payload (surrounding whitespace ignored).
pub fn fingerprint(b64_data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b64_data.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// True when this exact image was previously rejected by upstream.
///
/// The common case is an empty blacklist; that returns before hashing so healthy traffic does
/// not pay a SHA-256 over multi-megabyte payloads on every validation.
pub fn is_corrupted_image(b64_data: &str) -> bool {
    {
        let set = CORRUPTED_IMAGES.lock();
        if set.is_empty() {
            return false;
        }
    }
    let fingerprint = fingerprint(b64_data);
    CORRUPTED_IMAGES.lock().contains(&fingerprint)
}

/// Remember an image as unprocessable. Returns true when it was newly recorded.
pub fn record_corrupted_image(b64_data: &str) -> bool {
    if b64_data.trim().is_empty() {
        return false;
    }
    CORRUPTED_IMAGES
        .lock()
        .insert(fingerprint(b64_data), MAX_FINGERPRINTS)
}

fn is_image_mime(mime: Option<&str>) -> bool {
    match mime.map(str::trim).filter(|m| !m.is_empty()) {
        // A missing mime type is treated like the sanitizer does: it falls back to an image.
        None => true,
        Some(m) => m.to_ascii_lowercase().starts_with("image/"),
    }
}

fn collect_from_parts<'a>(parts: &'a [Value], out: &mut Vec<&'a str>) {
    for part in parts {
        let Some(inline) = part.get("inlineData").or_else(|| part.get("inline_data")) else {
            continue;
        };
        let mime = inline
            .get("mimeType")
            .or_else(|| inline.get("mime_type"))
            .and_then(Value::as_str);
        if !is_image_mime(mime) {
            continue;
        }
        if let Some(data) = inline.get("data").and_then(Value::as_str) {
            if !data.trim().is_empty() {
                out.push(data);
            }
        }
    }
}

/// Every inline image payload in a Gemini request body, in document order. Accepts both the
/// v1internal wrapper (`{"request": {...}}`) and a bare Gemini body.
pub fn collect_inline_image_payloads(body: &Value) -> Vec<&str> {
    let root = body.get("request").unwrap_or(body);
    let mut out = Vec::new();
    if let Some(contents) = root.get("contents").and_then(Value::as_array) {
        for content in contents {
            if let Some(parts) = content.get("parts").and_then(Value::as_array) {
                collect_from_parts(parts, &mut out);
            }
        }
    }
    for key in ["systemInstruction", "system_instruction"] {
        if let Some(parts) = root
            .get(key)
            .and_then(|s| s.get("parts"))
            .and_then(Value::as_array)
        {
            collect_from_parts(parts, &mut out);
        }
    }
    out
}

/// Fingerprint every image in the payload that upstream just refused. Returns how many images
/// the payload carried (not how many were new), so callers can log it and decide whether a
/// retry is worthwhile: zero means there was no image to blame and a retry would be identical.
pub fn record_corrupted_images_in_gemini_payload(body: &Value) -> usize {
    let images = collect_inline_image_payloads(body);
    for data in &images {
        record_corrupted_image(data);
    }
    images.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Each test below uses its own payload string: the blacklist is process-global and the
    // test harness runs tests in parallel, so shared fixtures would make them order-dependent.

    #[test]
    fn task_fingerprint_ignores_surrounding_whitespace_but_not_content() {
        assert_eq!(fingerprint("abc"), fingerprint("  abc\n"));
        assert_ne!(fingerprint("abc"), fingerprint("abd"));
        assert_eq!(fingerprint("abc").len(), 64);
    }

    #[test]
    fn task_fingerprint_set_evicts_oldest_instead_of_forgetting_everything() {
        let mut set = FingerprintSet::default();
        assert!(set.insert("a".into(), 3));
        assert!(set.insert("b".into(), 3));
        assert!(set.insert("c".into(), 3));
        assert!(!set.insert("a".into(), 3), "re-inserting is a no-op");
        assert_eq!(set.len(), 3);

        assert!(set.insert("d".into(), 3));
        assert_eq!(set.len(), 3, "capacity is a hard bound");
        assert!(!set.contains("a"), "the oldest entry is the one evicted");
        assert!(set.contains("b") && set.contains("c") && set.contains("d"));
    }

    #[test]
    fn task_fingerprint_set_zero_capacity_still_bounded() {
        let mut set = FingerprintSet::default();
        assert!(set.insert("a".into(), 0));
        assert!(set.insert("b".into(), 0));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn task_record_then_detect_global_blacklist() {
        let bad = "guard-test-record-then-detect-AAAA";
        assert!(!is_corrupted_image(bad));
        assert!(record_corrupted_image(bad));
        assert!(is_corrupted_image(bad));
        assert!(
            is_corrupted_image(&format!("  {bad}\n")),
            "whitespace-insensitive"
        );
        assert!(!record_corrupted_image(bad), "second record is not new");
        assert!(!is_corrupted_image("guard-test-some-other-image-BBBB"));
    }

    #[test]
    fn task_record_ignores_empty_payload() {
        assert!(!record_corrupted_image(""));
        assert!(!record_corrupted_image("   \n"));
    }

    #[test]
    fn task_collect_handles_wrapped_bare_and_snake_case_bodies() {
        let wrapped = json!({
            "project": "p",
            "request": {
                "contents": [
                    {"role": "user", "parts": [
                        {"text": "look"},
                        {"inlineData": {"mimeType": "image/png", "data": "IMG1"}},
                        {"inline_data": {"mime_type": "image/jpeg", "data": "IMG2"}}
                    ]},
                    {"role": "model", "parts": [{"text": "ok"}]}
                ],
                "systemInstruction": {"parts": [
                    {"inlineData": {"mimeType": "image/webp", "data": "IMG3"}}
                ]}
            }
        });
        assert_eq!(
            collect_inline_image_payloads(&wrapped),
            vec!["IMG1", "IMG2", "IMG3"]
        );

        let bare = json!({"contents": [
            {"role": "user", "parts": [{"inlineData": {"mimeType": "image/png", "data": "IMG4"}}]}
        ]});
        assert_eq!(collect_inline_image_payloads(&bare), vec!["IMG4"]);
    }

    #[test]
    fn task_collect_only_blames_images_not_other_media_or_empty_data() {
        let body = json!({"contents": [{"role": "user", "parts": [
            {"inlineData": {"mimeType": "application/pdf", "data": "PDF1"}},
            {"inlineData": {"mimeType": "audio/wav", "data": "WAV1"}},
            {"inlineData": {"mimeType": "video/mp4", "data": "MP41"}},
            {"inlineData": {"mimeType": "image/png", "data": "   "}},
            {"inlineData": {"mimeType": "IMAGE/PNG", "data": "UPPER"}},
            {"inlineData": {"data": "NOMIME"}},
            {"functionResponse": {"name": "x", "response": {}}}
        ]}]});
        assert_eq!(
            collect_inline_image_payloads(&body),
            vec!["UPPER", "NOMIME"]
        );
    }

    #[test]
    fn task_record_from_payload_blacklists_every_image_and_reports_count() {
        let body = json!({"request": {"contents": [{"role": "user", "parts": [
            {"text": "two screenshots"},
            {"inlineData": {"mimeType": "image/png", "data": "guard-test-payload-img-CCCC"}},
            {"inlineData": {"mimeType": "image/png", "data": "guard-test-payload-img-DDDD"}}
        ]}]}});
        assert_eq!(record_corrupted_images_in_gemini_payload(&body), 2);
        assert!(is_corrupted_image("guard-test-payload-img-CCCC"));
        assert!(is_corrupted_image("guard-test-payload-img-DDDD"));
    }

    #[test]
    fn task_payload_without_images_reports_zero_so_caller_does_not_pointlessly_retry() {
        let body = json!({"contents": [{"role": "user", "parts": [{"text": "hello"}]}]});
        assert_eq!(record_corrupted_images_in_gemini_payload(&body), 0);
        assert_eq!(record_corrupted_images_in_gemini_payload(&json!({})), 0);
    }

    #[test]
    fn task_record_does_not_mutate_the_payload() {
        let body = json!({"contents": [{"role": "user", "parts": [
            {"inlineData": {"mimeType": "image/png", "data": "guard-test-immutable-EEEE"}}
        ]}]});
        let before = body.clone();
        record_corrupted_images_in_gemini_payload(&body);
        assert_eq!(body, before);
    }
}
