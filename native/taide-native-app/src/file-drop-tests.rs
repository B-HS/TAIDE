use super::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
struct SyntheticDrop(PathBuf);

impl egui::DroppedFile for SyntheticDrop {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        Err("synthetic drop has no contents".into())
    }
}

fn observe(input: egui::RawInput) -> (bool, Vec<String>) {
    let context = egui::Context::default();
    let mut observed = (false, Vec::new());
    context
        .run_ui(input, |ui| {
            observed = (is_hovering(ui.ctx()), dropped_paths(ui.ctx()));
        })
        .textures_delta
        .clear();
    observed
}

#[test]
fn 끄는_중인_파일이_있으면_안내_상태이고_놓은_경로는_os_순서대로_읽는다() {
    let hovering = egui::RawInput {
        hovered_files: vec![egui::HoveredFile {
            path: Some(PathBuf::from("/synthetic/alpha")),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert_eq!(observe(hovering), (true, Vec::new()));
    assert_eq!(observe(egui::RawInput::default()), (false, Vec::new()));

    let dropped = egui::RawInput {
        dropped_files: ["/synthetic/alpha", "/synthetic/beta.txt"]
            .into_iter()
            .map(|path| Arc::new(SyntheticDrop(PathBuf::from(path))) as egui::DroppedFileHandle)
            .collect(),
        ..Default::default()
    };
    assert_eq!(
        observe(dropped),
        (
            false,
            vec![
                "/synthetic/alpha".to_owned(),
                "/synthetic/beta.txt".to_owned()
            ]
        )
    );
}

#[cfg(unix)]
#[test]
fn utf8가_아닌_경로는_건너뛰고_나머지는_연다() {
    use std::os::unix::ffi::OsStrExt as _;

    let invalid = PathBuf::from(std::ffi::OsStr::from_bytes(b"/synthetic/\xff"));
    let dropped = egui::RawInput {
        dropped_files: [
            PathBuf::from("/synthetic/alpha"),
            invalid,
            PathBuf::from("/synthetic/beta"),
        ]
        .into_iter()
        .map(|path| Arc::new(SyntheticDrop(path)) as egui::DroppedFileHandle)
        .collect(),
        ..Default::default()
    };
    assert_eq!(
        observe(dropped).1,
        vec!["/synthetic/alpha".to_owned(), "/synthetic/beta".to_owned()]
    );
}
