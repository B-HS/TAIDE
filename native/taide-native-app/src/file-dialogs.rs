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
