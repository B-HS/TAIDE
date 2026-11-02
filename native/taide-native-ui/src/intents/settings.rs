#[derive(Debug, Clone)]
pub enum Intent {
    OpenSettings,
    OpenSettingsFile,
    ChangeEditorFontSize {
        increase: bool,
    },
    #[cfg(feature = "native-host")]
    ToggleEditorMinimap,
}
