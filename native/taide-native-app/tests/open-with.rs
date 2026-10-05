use std::collections::HashMap;

use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{AuxWindowLayout, PaneNode, Tab, TabKind};
use taide_native_app::open_with::{
    MAX_OVERRIDES, Mode, PreviewKind, Registry, Surface, preview_kind,
};

fn file_tab(path: &str) -> Tab {
    Tab {
        id: TabId::new(),
        kind: TabKind::File { path: path.into() },
        title: path.into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

#[test]
fn open_with는_원본_확장자와_쓰기순서_상한_창격리를_보존한다() {
    for (kind, extensions) in [
        (PreviewKind::Image, "png jpg jpeg gif webp bmp svg avif"),
        (PreviewKind::Video, "mp4 webm mov m4v"),
        (PreviewKind::Audio, "mp3 wav flac m4a ogg"),
        (PreviewKind::Pdf, "pdf"),
        (PreviewKind::Html, "html htm"),
        (PreviewKind::Spreadsheet, "xlsx xls csv"),
        (PreviewKind::Presentation, "pptx"),
        (PreviewKind::Hwp, "hwp hwpx"),
    ] {
        for extension in extensions.split_whitespace() {
            assert_eq!(
                preview_kind(&format!("한글.{}", extension.to_uppercase())),
                Some(kind)
            );
            assert_eq!(preview_kind(&format!(".{extension}")), None);
            assert_eq!(preview_kind(&format!(".hidden.{extension}")), Some(kind));
        }
    }
    for path in ["image", "image.", "image.png.tmp", "image.md"] {
        assert_eq!(preview_kind(path), None);
    }
    let mut registry = Registry::default();
    for index in 0..MAX_OVERRIDES {
        registry.set(format!("/root/{index}.png"), Mode::Editor);
    }
    assert_eq!(registry.surface("/root/0.png"), Surface::Editor);
    registry.set("/root/new.png".into(), Mode::Editor);
    assert_eq!(
        registry.surface("/root/0.png"),
        Surface::Preview(PreviewKind::Image)
    );
    registry.set("/root/1.png".into(), Mode::Editor);
    registry.set("/root/extra.png".into(), Mode::Editor);
    assert_eq!(
        registry.surface("/root/2.png"),
        Surface::Preview(PreviewKind::Image)
    );
    assert_eq!(registry.surface("/root/1.png"), Surface::Editor);
    assert_eq!(
        registry.surface("/root/../root/1.png"),
        Surface::Preview(PreviewKind::Image)
    );
    assert_eq!(
        Registry::default().surface("/root/1.png"),
        Surface::Preview(PreviewKind::Image)
    );
    registry.set("/root/1.png".into(), Mode::Preview);
    assert_eq!(
        registry.surface("/root/1.png"),
        Surface::Preview(PreviewKind::Image)
    );
}

#[test]
fn open_with는_실제_탭경로변경과_project_및_aux_close만_정리한다() {
    let first = ProjectId::new();
    let second = ProjectId::new();
    let old = "/root/old.png";
    let new = "/root/new.png";
    let other = "/other/shared.png";
    let mut layout = taide_layout::service::default_layout();
    let tab = file_tab(old);
    layout.root = PaneNode::Leaf {
        id: layout.focused_pane.clone(),
        tabs: vec![tab.clone()],
        active: Some(tab.id.clone()),
    };
    let mut layouts = HashMap::from([
        (first.clone(), layout.clone()),
        (second.clone(), layout.clone()),
    ]);
    let mut registry = Registry::default();
    registry.reconcile(&layouts);
    registry.set(old.into(), Mode::Editor);
    let next = layouts.get_mut(&first).unwrap();
    next.revision += 1;
    let PaneNode::Leaf { tabs, .. } = &mut next.root else {
        unreachable!()
    };
    tabs[0].kind = TabKind::File { path: new.into() };
    registry.reconcile(&layouts);
    assert_eq!(registry.surface(old), Surface::Preview(PreviewKind::Image));
    assert_eq!(registry.surface(new), Surface::Editor);
    registry.set(other.into(), Mode::Editor);
    registry.set(old.into(), Mode::Editor);
    layouts.remove(&first);
    registry.reconcile(&layouts);
    assert_eq!(registry.surface(new), Surface::Preview(PreviewKind::Image));
    assert_eq!(
        registry.surface(other),
        Surface::Preview(PreviewKind::Image)
    );
    assert_eq!(registry.surface(old), Surface::Editor);
    registry.close_path(old, &layout);
    assert_eq!(registry.surface(old), Surface::Editor);
    let mut empty = taide_layout::service::default_layout();
    empty.auxiliary_windows.push(AuxWindowLayout {
        slot: 1,
        root: layout.root.clone(),
        focused_pane: layout.focused_pane.clone(),
    });
    registry.close_path(old, &empty);
    assert_eq!(registry.surface(old), Surface::Editor);
    empty.auxiliary_windows.clear();
    registry.close_path(old, &empty);
    assert_eq!(registry.surface(old), Surface::Preview(PreviewKind::Image));
    registry.set(old.into(), Mode::Editor);
    let next = layouts.get_mut(&second).unwrap();
    next.revision += 1;
    next.root = empty.root;
    registry.reconcile(&layouts);
    assert_eq!(registry.surface(old), Surface::Preview(PreviewKind::Image));
}
