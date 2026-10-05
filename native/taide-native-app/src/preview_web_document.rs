use std::{
    io::{self, Write},
    path::Path,
};

use dom_query::{Document, SerializableNodeRef};
use encoding_rs::UTF_8;
use html5ever::serialize::{SerializeOpts, TraversalScope, serialize};
use taide_model::{error::AppResult, file::READ_ONLY_FILE_BYTES};
use url::Url;

use crate::preview::invalid;

pub const SCHEME: &str = "taide-preview";
pub const HOST: &str = "localhost";
pub const DOCUMENT_POLICY: &str =
    "script-src 'none'; object-src 'none'; frame-src 'none'; form-action 'none'";
pub const RESOURCE_POLICY: &str = "default-src 'none'; script-src 'none'; object-src 'none'; frame-src 'none'; form-action 'none'; connect-src 'none'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; media-src 'self'";
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
const OUTPUT_RESERVE_BYTES: usize = 64 * 1024;
const DOCTYPE: &[u8] = b"<!doctype html>";
const MAX_CAPABILITY_BYTES: usize = 64;

pub fn source_url(path: &Path) -> AppResult<Url> {
    let file = Url::from_file_path(path)
        .map_err(|_| invalid("HTML preview requires an absolute file path"))?;
    Url::parse(&format!("{SCHEME}://{HOST}{}", file.path()))
        .map_err(|_| invalid("HTML preview source URL is invalid"))
}

pub(crate) fn validate_source(source: &Url) -> AppResult<()> {
    if source.scheme() != SCHEME
        || source.host_str() != Some(HOST)
        || !source.username().is_empty()
        || source.password().is_some()
        || source.port().is_some()
    {
        return Err(invalid("HTML preview source is not an isolated asset URL"));
    }
    Ok(())
}

pub(crate) fn validate_document_source(source: &Url) -> AppResult<()> {
    if validate_source(source).is_ok() {
        return Ok(());
    }
    let capability = source
        .path_segments()
        .and_then(|mut segments| segments.next())
        .and_then(|segment| segment.strip_prefix("prj-"))
        .filter(|value| !value.is_empty() && value.len() <= MAX_CAPABILITY_BYTES)
        .filter(|value| {
            value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        });
    if source.scheme() != "http"
        || source.host_str() != Some("127.0.0.1")
        || source.port().is_none_or(|port| port == 0)
        || !source.username().is_empty()
        || source.password().is_some()
        || source.query().is_some()
        || source.fragment().is_some()
        || capability.is_none()
    {
        return Err(invalid(
            "HTML preview source is not an isolated document URL",
        ));
    }
    Ok(())
}

pub fn prepare_html(bytes: &[u8], source: &Url) -> AppResult<String> {
    validate_document_source(source)?;
    if bytes.len() as u64 > READ_ONLY_FILE_BYTES {
        return Err(invalid("HTML preview input exceeds the document budget"));
    }
    let (text, _) = UTF_8.decode_with_bom_removal(bytes);
    let document = Document::from(text.as_ref());
    let existing = document.select_single("base[href]").attr("href");
    let base = source
        .join(existing.as_deref().unwrap_or("."))
        .map_err(|_| invalid("HTML preview base URL is invalid"))?;
    document.select("base").remove();
    let head = document.select_single("head");
    head.prepend_html("<base>");
    document
        .select_single("base")
        .set_attr("href", base.as_str());
    head.prepend_html("<meta>");
    let policy = document.select_single("head > meta");
    policy.set_attr("http-equiv", "Content-Security-Policy");
    policy.set_attr("content", DOCUMENT_POLICY);
    let mut output = Output { bytes: Vec::new() };
    output
        .write_all(DOCTYPE)
        .map_err(|_| invalid("HTML preview output allocation failed"))?;
    serialize(
        &mut output,
        &SerializableNodeRef::from(document.html_root()),
        SerializeOpts {
            scripting_enabled: false,
            traversal_scope: TraversalScope::IncludeNode,
            ..Default::default()
        },
    )
    .map_err(|_| invalid("HTML preview output exceeds its budget or failed allocation"))?;
    String::from_utf8(output.bytes)
        .map_err(|_| invalid("HTML preview serializer produced invalid UTF-8"))
}

struct Output {
    bytes: Vec<u8>,
}

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|length| *length <= MAX_OUTPUT_BYTES)
            .ok_or_else(|| io::Error::other("HTML output budget exceeded"))?;
        if length > self.bytes.capacity() {
            let target = length
                .saturating_add(OUTPUT_RESERVE_BYTES)
                .min(MAX_OUTPUT_BYTES);
            self.bytes
                .try_reserve_exact(target - self.bytes.len())
                .map_err(|_| io::Error::other("HTML output allocation failed"))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_served_document_urlは_loopback_capabilityだけを_helper_originに認める() {
        let id = taide_model::ids::ProjectId::new();
        let source = Url::parse(&format!(
            "http://127.0.0.1:32101/{id}/synthetic%20project/pages/index.html"
        ))
        .unwrap();
        let prepared = prepare_html(b"<base href='../assets/'><p>synthetic", &source).unwrap();
        let document = Document::from(prepared.as_str());
        assert_eq!(
            document
                .select_single("base")
                .attr("href")
                .unwrap()
                .as_ref(),
            format!("http://127.0.0.1:32101/{id}/synthetic%20project/assets/")
        );
        for source in [
            "http://external.invalid:32101/prj-x/synthetic.html",
            "http://localhost:32101/prj-x/synthetic.html",
            "http://127.0.0.1/prj-x/synthetic.html",
            "http://127.0.0.1:0/prj-x/synthetic.html",
            "http://user@127.0.0.1:32101/prj-x/synthetic.html",
            "http://127.0.0.1:32101/missing/synthetic.html",
            "http://127.0.0.1:32101/prj-x/synthetic.html?q=x",
            "http://127.0.0.1:32101/prj-x/synthetic.html#fragment",
        ] {
            assert!(prepare_html(b"synthetic", &Url::parse(source).unwrap()).is_err());
        }
        assert!(validate_source(&source).is_err());
    }

    #[test]
    fn html_document는_원본_utf8_base_dom_policy와_출력_상한을_보존한다() {
        let source = source_url(Path::new("/synthetic project/pages/index.html")).unwrap();
        assert_eq!(
            source.as_str(),
            "taide-preview://localhost/synthetic%20project/pages/index.html"
        );
        let bytes = b"\xef\xbb\xbf<html><head><base target='_blank'><base href='../assets/?q=&quot;'><base href='/ignored'><link rel='stylesheet' href='../styles/main.css'></head><body>\xe2\x82<script>alert('not executed')</script><img src='photo.png'><template><base href='/nested'></template></body></html>";
        let result = prepare_html(bytes, &source).unwrap();
        assert!(result.starts_with("<!doctype html><html>"));
        let parsed = Document::from(result.as_str());
        assert_eq!(parsed.select("base").length(), 1);
        let base = parsed.select_single("base").attr("href").unwrap();
        assert_eq!(
            base.as_ref(),
            "taide-preview://localhost/synthetic%20project/assets/?q=%22"
        );
        assert!(parsed.select_single("base").attr("target").is_none());
        assert_eq!(
            parsed
                .select_single("meta")
                .attr("content")
                .unwrap()
                .as_ref(),
            DOCUMENT_POLICY
        );
        assert!(result.find("Content-Security-Policy").unwrap() < result.find("<base").unwrap());
        assert!(parsed.select_single("body").text().contains('\u{fffd}'));
        assert_eq!(
            parsed.select_single("img").attr("src").unwrap().as_ref(),
            "photo.png"
        );
        assert_eq!(parsed.select("script").length(), 1);
        let relative = prepare_html(b"<p>synthetic", &source).unwrap();
        let parsed = Document::from(relative.as_str());
        assert_eq!(
            parsed.select_single("base").attr("href").unwrap().as_ref(),
            "taide-preview://localhost/synthetic%20project/pages/"
        );
        assert_eq!(
            source.join("../styles/main.css").unwrap().as_str(),
            "taide-preview://localhost/synthetic%20project/styles/main.css"
        );
        assert!(source_url(Path::new("relative.html")).is_err());
        assert!(prepare_html(b"safe", &Url::parse("https://external.invalid/").unwrap()).is_err());
        assert!(
            prepare_html(
                b"safe",
                &Url::parse("taide-preview://other/synthetic").unwrap()
            )
            .is_err()
        );
        let mut output = Output { bytes: Vec::new() };
        let oversized = vec![0; MAX_OUTPUT_BYTES + 1];
        assert!(output.write_all(&oversized).is_err());
        assert_eq!(output.bytes.capacity(), 0);
        assert!(prepare_html(&oversized, &source).is_err());
    }
}
