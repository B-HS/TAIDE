pub trait PreviewSurfaces {
    /// Whether a file tab can be reopened between the editor and a preview (`resolvePreviewKind`).
    fn can_reopen_with(&self, _path: &str) -> bool {
        false
    }
}
