use crate::command_registry::PaletteEntry;

#[derive(Debug, Clone)]
pub enum Intent {
    OpenPalette(PaletteEntry),
    OpenKeybindings,
    #[cfg(feature = "native-host")]
    OpenTaskRunner(taide_model::ids::ProjectId),
}
