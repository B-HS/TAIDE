use std::{path::Path, sync::Arc};

use dom_query::Document;
use taide_model::error::{AppError, AppResult};
use taide_runtime::AppServices;
use url::Url;

use crate::{
    open_with::{PreviewKind, preview_kind},
    preview::invalid,
    preview_web::{Prepared, Request},
    preview_web_document::{prepare_html, validate_document_source},
    preview_web_helper::MAX_SOURCE_BYTES,
    preview_web_http::Server,
    preview_web_resource::media_owner,
};

pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024;
const MAX_FILE_NAME_BYTES: usize = 4096;
const VIDEO_TEMPLATE: &str = "<!doctype html><html><head><style></style></head><body class='video'><video controls></video></body></html>";
const AUDIO_TEMPLATE: &str = "<!doctype html><html><head><style></style></head><body class='audio'><svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round' aria-hidden='true'><path d='M9 18V5l12-2v13'></path><circle cx='6' cy='18' r='3'></circle><circle cx='18' cy='16' r='3'></circle></svg><span></span><audio controls></audio></body></html>";
const STYLES: &str = "*{box-sizing:border-box}html,body{width:100%;height:100%;margin:0;overflow:hidden}body{display:flex;align-items:center;justify-content:center;padding:16px;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,'Noto Sans KR',sans-serif;font-size:13px;-webkit-font-smoothing:antialiased;user-select:none}.video video{max-height:100%;max-width:100%}.audio{flex-direction:column;gap:16px}.audio svg{width:40px;height:40px;flex-shrink:0}.audio span{font-size:14px;line-height:20px;max-width:448px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.audio audio{width:100%;max-width:448px}";

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Appearance {
    pub background: [u8; 4],
    pub foreground: [u8; 4],
    pub muted: [u8; 4],
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            background: eframe::egui::Color32::BLACK.to_array(),
            foreground: eframe::egui::Color32::WHITE.to_array(),
            muted: eframe::egui::Color32::GRAY.to_array(),
        }
    }
}

fn color([red, green, blue, alpha]: [u8; 4]) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
}

pub fn document(
    kind: PreviewKind,
    source: &Url,
    file_name: &str,
    appearance: Appearance,
) -> AppResult<String> {
    validate_document_source(source)?;
    if source.scheme() != "http"
        || source.as_str().len() > MAX_SOURCE_BYTES as usize
        || file_name.len() > MAX_FILE_NAME_BYTES
    {
        return Err(invalid(
            "media document fields exceed their isolated bounds",
        ));
    }
    let (template, selector) = match kind {
        PreviewKind::Audio => (AUDIO_TEMPLATE, "audio"),
        PreviewKind::Video => (VIDEO_TEMPLATE, "video"),
        _ => return Err(invalid("media document requires an audio or video source")),
    };
    let dom = Document::from(template);
    dom.select_single(selector).set_attr("src", source.as_str());
    dom.select_single("span").set_text(file_name);
    dom.select_single("style").set_text(&format!(
        "{STYLES}body{{background:{};color:{}}}.audio svg{{color:{}}}",
        color(appearance.background),
        color(appearance.foreground),
        color(appearance.muted)
    ));
    let html = prepare_html(dom.html().as_bytes(), source)?;
    if html.len() > MAX_DOCUMENT_BYTES {
        return Err(invalid("media document exceeds its output budget"));
    }
    Ok(html)
}

pub async fn read(
    services: &AppServices,
    request: &Request,
    server: Arc<Server>,
    appearance: Appearance,
    on_source_ready: impl FnOnce() + Send + 'static,
) -> AppResult<Prepared> {
    let kind = preview_kind(&request.path)
        .filter(|kind| matches!(kind, PreviewKind::Audio | PreviewKind::Video))
        .ok_or_else(|| invalid("media preview kind is not supported"))?;
    let owner = media_owner(services, request.path.clone()).await?;
    on_source_ready();
    let path = request.path.clone();
    services
        .tasks
        .run_blocking_result("native-media-document", move || {
            let scope = owner.scope();
            scope.check()?;
            let canonical = scope.canonical().to_path_buf();
            let ticket = server.register(scope.clone())?;
            let source = ticket.url(&scope.source_url()?)?;
            let file_name = Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| AppError::InvalidArgument("media filename is not UTF-8".into()))?;
            let html = Arc::new(document(kind, &source, file_name, appearance)?);
            scope.check()?;
            let source = ticket.publish_media(html.clone())?;
            Ok(Prepared {
                canonical,
                source,
                html,
                owner,
                ticket: Some(ticket),
            })
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_media_document는_원본_controls_style과_비실행_filename을_재현한다() {
        let source =
            Url::parse("http://127.0.0.1:32101/prj-synthetic/root/synthetic%20%23%20%3F.mp3")
                .unwrap();
        let name = "한글 <script>notExecuted()</script> & .mp3";
        let appearance = Appearance {
            background: [1, 0, 0, 1],
            foreground: [0, 1, 0, 1],
            muted: [0, 0, 1, 1],
        };
        let html = document(PreviewKind::Audio, &source, name, appearance).unwrap();
        let dom = Document::from(html.as_str());
        assert_eq!(dom.select("script").length(), 0);
        assert_eq!(dom.select_single("span").text().as_ref(), name);
        assert_eq!(
            dom.select_single("audio").attr("src").unwrap().as_ref(),
            source.as_str()
        );
        assert!(dom.select_single("audio").attr("controls").is_some());
        assert!(dom.select_single("audio").attr("autoplay").is_none());
        assert!(dom.select_single("audio").attr("preload").is_none());
        assert!(
            dom.select_single("style")
                .text()
                .contains("max-width:448px")
        );
        assert!(
            dom.select_single("style")
                .text()
                .contains("background:#01000001")
        );
        assert!(html.len() < MAX_DOCUMENT_BYTES);
        let html = document(PreviewKind::Video, &source, "unused", appearance).unwrap();
        let dom = Document::from(html.as_str());
        assert_eq!(dom.select("audio,svg,span").length(), 0);
        assert!(dom.select_single("video").attr("controls").is_some());
        assert!(
            dom.select_single("style")
                .text()
                .contains("max-height:100%;max-width:100%")
        );
        assert!(document(PreviewKind::Html, &source, "unused", appearance).is_err());
        assert!(
            document(
                PreviewKind::Audio,
                &Url::parse("https://external.invalid").unwrap(),
                name,
                appearance
            )
            .is_err()
        );
        assert!(
            document(
                PreviewKind::Audio,
                &source,
                &"x".repeat(MAX_FILE_NAME_BYTES + 1),
                appearance
            )
            .is_err()
        );
    }
}
