use std::collections::HashMap;
use std::sync::Arc;

use taide_lsp::native::protocol::lsp_types::{Diagnostic, Uri};

const ESCAPE_BYTES: usize = 3;
const HEX_RADIX: u32 = 16;
const ROOTED_DRIVE_BYTES: usize = 3;
const ROOTLESS_DRIVE_BYTES: usize = 2;

#[derive(Clone, Default)]
pub(super) struct Store {
    batches: HashMap<Key, Arc<[Diagnostic]>>,
}

impl Store {
    pub(super) fn publish(&mut self, uri: &Uri, diagnostics: &[Diagnostic]) {
        let key = Key::new(uri);
        if diagnostics.is_empty() {
            self.batches.remove(&key);
            return;
        }
        if self
            .batches
            .get(&key)
            .is_some_and(|stored| stored.as_ref() == diagnostics)
        {
            return;
        }
        self.batches.insert(key, Arc::from(diagnostics));
    }

    pub(super) fn get(&self, uri: &str) -> &[Diagnostic] {
        let Ok(uri) = uri.parse::<Uri>() else {
            return &[];
        };
        self.batches
            .get(&Key::new(&uri))
            .map_or(&[], |batch| batch.as_ref())
    }

    pub(super) fn remove(&mut self, uri: &str) {
        if let Ok(uri) = uri.parse::<Uri>() {
            self.batches.remove(&Key::new(&uri));
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    scheme: String,
    authority: String,
    path: String,
    query: String,
    fragment: String,
}

pub(super) fn matches(uri: &str, incoming: &Uri) -> bool {
    uri.parse::<Uri>()
        .is_ok_and(|uri| Key::new(&uri) == Key::new(incoming))
}

impl Key {
    fn new(uri: &Uri) -> Self {
        let scheme = uri
            .scheme()
            .map_or("file", |scheme| scheme.as_str())
            .to_owned();
        let authority = uri
            .authority()
            .map_or_else(String::new, |authority| decode(authority.as_str()));
        let authority = match authority.split_once('@') {
            Some((user, host)) => format!("{user}@{}", host.to_lowercase()),
            None => authority.to_lowercase(),
        };
        let mut path = decode(uri.path().as_str());
        if matches!(scheme.as_str(), "file" | "http" | "https") && !path.starts_with('/') {
            path.insert(0, '/');
        }
        let bytes = path.as_bytes();
        if bytes.len() >= ROOTED_DRIVE_BYTES
            && bytes[0] == b'/'
            && bytes[2] == b':'
            && bytes[1].is_ascii_uppercase()
        {
            path = format!(
                "/{}{}",
                char::from(bytes[1].to_ascii_lowercase()),
                &path[2..]
            );
        } else if bytes.len() >= ROOTLESS_DRIVE_BYTES
            && bytes[1] == b':'
            && bytes[0].is_ascii_uppercase()
        {
            path = format!(
                "{}{}",
                char::from(bytes[0].to_ascii_lowercase()),
                &path[1..]
            );
        }
        Self {
            scheme,
            authority,
            path,
            query: uri
                .query()
                .map_or_else(String::new, |query| decode(query.as_str())),
            fragment: uri
                .fragment()
                .map_or_else(String::new, |fragment| decode(fragment.as_str())),
        }
    }
}

fn decode(component: &str) -> String {
    let mut result = String::with_capacity(component.len());
    let mut remaining = component;
    while let Some(start) = remaining.find('%') {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        let encoded_bytes = remaining
            .as_bytes()
            .as_chunks::<ESCAPE_BYTES>()
            .0
            .iter()
            .take_while(|triplet| {
                triplet[0] == b'%'
                    && triplet[1].is_ascii_hexdigit()
                    && triplet[2].is_ascii_hexdigit()
            })
            .count()
            * ESCAPE_BYTES;
        if encoded_bytes == 0 {
            result.push('%');
            remaining = &remaining[1..];
            continue;
        }
        let encoded = &remaining[..encoded_bytes];
        let bytes = encoded
            .as_bytes()
            .as_chunks::<ESCAPE_BYTES>()
            .0
            .iter()
            .map(|triplet| {
                let high = char::from(triplet[1])
                    .to_digit(HEX_RADIX)
                    .expect("validated hex digit");
                let low = char::from(triplet[2])
                    .to_digit(HEX_RADIX)
                    .expect("validated hex digit");
                u8::try_from(high * HEX_RADIX + low).expect("one decoded byte")
            })
            .collect::<Vec<_>>();
        let mut cursor = 0;
        let mut invalid_end = 0;
        for chunk in bytes.utf8_chunks() {
            cursor += chunk.valid().len() + chunk.invalid().len();
            if !chunk.invalid().is_empty() {
                invalid_end = cursor;
            }
        }
        result.push_str(&encoded[..invalid_end * ESCAPE_BYTES]);
        result
            .push_str(std::str::from_utf8(&bytes[invalid_end..]).expect("validated UTF-8 suffix"));
        remaining = &remaining[encoded_bytes..];
    }
    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_lsp::native::protocol::lsp_types::NumberOrString;

    #[test]
    fn 원본_raw는_미바인딩_uri와_정규화_빈발행_폐기_세션격리를_보존한다() {
        let wire = "file:///synthetic/a(b),%20%ED%95%9C.rs"
            .parse::<Uri>()
            .unwrap();
        let model = "file:///synthetic/a%28b%29%2C%20%ED%95%9C.rs";
        let diagnostic = Diagnostic {
            code: Some(NumberOrString::Number(42)),
            source: Some("synthetic server".into()),
            data: Some(serde_json::json!({"fix": {"token": "synthetic"}})),
            ..Default::default()
        };
        let mut first = Store::default();
        let mut second = Store::default();
        first.publish(&wire, std::slice::from_ref(&diagnostic));
        assert_eq!(first.get(model), std::slice::from_ref(&diagnostic));
        assert!(second.get(model).is_empty());
        second.publish(&wire, std::slice::from_ref(&diagnostic));
        let copied = first.clone();
        let identity = first.batches.values().next().unwrap().clone();
        first.publish(&wire, std::slice::from_ref(&diagnostic));
        assert!(Arc::ptr_eq(
            &identity,
            first.batches.values().next().unwrap()
        ));
        first.remove(model);
        assert!(first.get(model).is_empty());
        assert_eq!(copied.get(model), std::slice::from_ref(&diagnostic));
        assert_eq!(second.get(model), std::slice::from_ref(&diagnostic));
        second.publish(&wire, &[]);
        assert!(second.batches.is_empty());
        drop(copied);
        for (wire, model) in [
            ("file:/C:/a%28b%29.rs", "file:///c%3A/a(b).rs"),
            (
                "custom://USER:PASS@HOST/a?x%3Dy#%7A",
                "custom://USER:PASS@host/a?x=y#z",
            ),
            ("file:///a%2Fb", "file:///a/b"),
            ("custom:%61%FF%62", "custom:%2561%25FFb"),
        ] {
            first.publish(&wire.parse().unwrap(), std::slice::from_ref(&diagnostic));
            assert_eq!(
                first.get(model),
                std::slice::from_ref(&diagnostic),
                "{wire}"
            );
            first.remove(model);
        }
        assert!(first.batches.is_empty());
        first.publish(
            &"file:///a/../b".parse().unwrap(),
            std::slice::from_ref(&diagnostic),
        );
        assert!(first.get("file:///b").is_empty());
        assert!(first.get("file:///localhost/b").is_empty());
        assert!(first.get("%malformed").is_empty());
    }
}
