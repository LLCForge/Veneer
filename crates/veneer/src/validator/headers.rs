//! Request headers from a HAR log, for `compare_headers` and `validate_with`.

use serde_json::Value;

use crate::headers::Header;
use crate::Error;

/// Request headers of the top-level document request in a HAR 1.2 log (the first HTML response,
/// else the first request).
pub fn headers_from_har(har: &Value) -> Result<Vec<Header>, Error> {
    let bad = |m: &str| Error::InvalidConstraint(format!("HAR: {m}"));
    let entries = har
        .pointer("/log/entries")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("missing log.entries"))?;
    let is_document = |e: &&Value| {
        e.pointer("/_resourceType").and_then(Value::as_str) == Some("document")
            || e.pointer("/response/content/mimeType")
                .and_then(Value::as_str)
                .is_some_and(|m| m.starts_with("text/html"))
    };
    let entry = entries
        .iter()
        .find(is_document)
        .or_else(|| entries.first())
        .ok_or_else(|| bad("no entries"))?;
    let list = entry
        .pointer("/request/headers")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("entry has no request.headers"))?;
    let headers = list
        .iter()
        .map(|h| {
            match (
                h.get("name").and_then(Value::as_str),
                h.get("value").and_then(Value::as_str),
            ) {
                (Some(name), Some(value)) => Ok(Header {
                    name: name.to_owned(),
                    value: value.to_owned(),
                }),
                _ => Err(bad("header without a string name and value")),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    super::check_headers(&headers)?;
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, navigation_headers, Constraints};
    use serde_json::json;

    #[test]
    fn har_document_request_is_found() {
        let p = generate(&Constraints {
            browser: Some("chrome".into()),
            seed: Some("har".into()),
            ..Default::default()
        })
        .unwrap();
        let headers: Vec<Value> = navigation_headers(&p)
            .iter()
            .map(|h| json!({ "name": h.name, "value": h.value }))
            .collect();
        let har = json!({ "log": { "version": "1.2", "entries": [
            { "request": { "headers": [{ "name": "accept", "value": "image/*" }] },
              "response": { "content": { "mimeType": "image/png" } } },
            { "request": { "headers": headers },
              "response": { "content": { "mimeType": "text/html; charset=utf-8" } } }
        ] } });
        assert_eq!(headers_from_har(&har).unwrap(), navigation_headers(&p));
        assert!(headers_from_har(&json!({ "log": {} })).is_err());
    }
}
