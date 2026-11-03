use std::path::PathBuf;
use std::sync::Arc;

pub(crate) struct SaveRequest {
    pub directory: PathBuf,
    pub name: String,
    pub title: String,
}

pub(crate) type SavePicker =
    Arc<dyn Fn(&eframe::Frame, SaveRequest) -> Option<PathBuf> + Send + Sync>;

pub(crate) fn save(frame: &eframe::Frame, request: SaveRequest) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_parent(frame)
        .set_directory(request.directory)
        .set_file_name(request.name)
        .set_title(request.title)
        .save_file()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenRequest {
    pub filter_name: &'static str,
    pub extensions: &'static [&'static str],
    pub title: String,
}

pub(crate) type OpenPicker =
    Arc<dyn Fn(&eframe::Frame, OpenRequest) -> Option<PathBuf> + Send + Sync>;

pub(crate) fn open(frame: &eframe::Frame, request: OpenRequest) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_parent(frame)
        .add_filter(request.filter_name, request.extensions)
        .set_title(request.title)
        .pick_file()
}
