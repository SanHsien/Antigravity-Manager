// Read the top-level `model` string of a JSON request body without building the whole tree.
//
// The monitor only needs the model name for its log line. Parsing the body into a
// `serde_json::Value` for that allocates a node for every element of the request (an
// order of magnitude more memory than the body text for array-heavy input) and the handler
// then parses the same bytes again. This reads the one field and skips everything else in
// place.
use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use std::fmt;

/// Map key that only remembers whether it is `model`; no allocation per key.
struct IsModelKey(bool);

impl<'de> Deserialize<'de> for IsModelKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;
        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = IsModelKey;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object key")
            }
            fn visit_str<E>(self, v: &str) -> Result<IsModelKey, E> {
                Ok(IsModelKey(v == "model"))
            }
        }
        deserializer.deserialize_str(KeyVisitor)
    }
}

/// The value of `model`: kept when it is a string, skipped (but still consumed) otherwise.
struct StringOrSkipped(Option<String>);

impl<'de> Deserialize<'de> for StringOrSkipped {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = StringOrSkipped;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_str<E>(self, v: &str) -> Result<StringOrSkipped, E> {
                Ok(StringOrSkipped(Some(v.to_owned())))
            }
            fn visit_string<E>(self, v: String) -> Result<StringOrSkipped, E> {
                Ok(StringOrSkipped(Some(v)))
            }
            fn visit_bool<E>(self, _: bool) -> Result<StringOrSkipped, E> {
                Ok(StringOrSkipped(None))
            }
            fn visit_i64<E>(self, _: i64) -> Result<StringOrSkipped, E> {
                Ok(StringOrSkipped(None))
            }
            fn visit_u64<E>(self, _: u64) -> Result<StringOrSkipped, E> {
                Ok(StringOrSkipped(None))
            }
            fn visit_f64<E>(self, _: f64) -> Result<StringOrSkipped, E> {
                Ok(StringOrSkipped(None))
            }
            fn visit_unit<E>(self) -> Result<StringOrSkipped, E> {
                Ok(StringOrSkipped(None))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<StringOrSkipped, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(StringOrSkipped(None))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<StringOrSkipped, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(StringOrSkipped(None))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

/// A JSON object reduced to its top-level `model` string. Anything that is not an object
/// is a deserialization error, which the caller maps to `None`.
struct TopLevelModel(Option<String>);

impl<'de> Deserialize<'de> for TopLevelModel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = TopLevelModel;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<TopLevelModel, A::Error> {
                let mut model = None;
                while let Some(IsModelKey(is_model)) = map.next_key()? {
                    if is_model {
                        // A repeated key: the last one wins, as in `serde_json::Value`.
                        model = map.next_value::<StringOrSkipped>()?.0;
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
                Ok(TopLevelModel(model))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

/// The top-level `model` string of a JSON request body, or `None` when the body is not a
/// JSON object, is not valid JSON, or has no string `model`.
pub fn top_level_model(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<TopLevelModel>(body)
        .ok()
        .and_then(|peek| peek.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// What the monitor did before: parse everything, then look the field up.
    fn model_via_full_tree(body: &[u8]) -> Option<String> {
        serde_json::from_slice::<Value>(body).ok().and_then(|v| {
            v.get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        })
    }

    #[test]
    fn task_model_peek_matches_the_full_parse_it_replaces() {
        let cases: &[&str] = &[
            r#"{"model":"claude-sonnet-4-5","messages":[{"role":"user","content":"hi"}]}"#,
            r#"{"messages":[{"role":"user","content":[{"type":"text","text":"x"}]}],"model":"gpt-4o","stream":true}"#,
            // Escapes and non-ASCII in the model name.
            r#"{"model":"a\"b\\cé😀模型"}"#,
            r#"{"model":""}"#,
            // `model` nested deeper is not the request's model.
            r#"{"metadata":{"model":"inner"},"input":[{"model":"inner2"}]}"#,
            r#"{"metadata":{"model":"inner"},"model":"outer"}"#,
            // Non-string `model`.
            r#"{"model":42}"#,
            r#"{"model":null}"#,
            r#"{"model":true}"#,
            r#"{"model":1.5e3}"#,
            r#"{"model":["a","b"]}"#,
            r#"{"model":{"id":"x","nested":[1,{"a":null}]}}"#,
            // Repeated key: last one wins in both.
            r#"{"model":"first","model":"second"}"#,
            r#"{"model":"first","model":7}"#,
            r#"{"model":7,"model":"second"}"#,
            // No model at all.
            r#"{}"#,
            r#"{"messages":[]}"#,
            // Not an object.
            r#"[{"model":"x"}]"#,
            r#""model""#,
            r#"123"#,
            r#"null"#,
            // Not JSON, truncated, or trailing garbage.
            r#""#,
            r#"   "#,
            r#"{"model":"x""#,
            r#"{"model":"x"} trailing"#,
            r#"{"model":"x",}"#,
            r#"{'model':'x'}"#,
            r#"{"model":"x","messages":[1,2,}"#,
            // Whitespace around tokens.
            " \n\t{ \"model\" :\n \"spaced\" , \"n\" : [ 1 , 2 ] } \n",
        ];
        for case in cases {
            assert_eq!(
                top_level_model(case.as_bytes()),
                model_via_full_tree(case.as_bytes()),
                "diverged on {case:?}"
            );
        }

        assert_eq!(
            top_level_model(br#"{"messages":[],"model":"claude-opus-4-6-thinking"}"#).as_deref(),
            Some("claude-opus-4-6-thinking")
        );
        assert_eq!(top_level_model(b"\xff\xfe{}"), None);
    }

    #[test]
    fn task_model_peek_handles_a_body_of_many_small_elements() {
        // The shape that made the full parse expensive: millions of tiny array elements.
        let mut body = String::from(r#"{"input":["#);
        for i in 0..200_000 {
            if i > 0 {
                body.push(',');
            }
            body.push('1');
        }
        body.push_str(r#"],"model":"after-the-array"}"#);
        assert_eq!(
            top_level_model(body.as_bytes()).as_deref(),
            Some("after-the-array")
        );
    }
}
