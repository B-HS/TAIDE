use taide_native_editor::document::{DocumentId, DocumentKey, EditorError};
use taide_native_editor::indent::IndentConfiguration;
use taide_native_editor::store::EditorStore;
use taide_native_ui::command_registry::IndentationCommand;

pub(crate) struct Request {
    document: DocumentId,
    key: DocumentKey,
    command: IndentationCommand,
}

impl Request {
    pub(crate) fn new(
        store: &mut EditorStore,
        document: DocumentId,
        configuration: IndentConfiguration,
        command: IndentationCommand,
    ) -> Result<(Self, u32), EditorError> {
        let options = store.configure_indentation(document, configuration)?;
        let snapshot = store.documents().snapshot(document)?;
        Ok((
            Self {
                document,
                key: snapshot.key,
                command,
            },
            options.tab_size,
        ))
    }

    pub(crate) fn apply(
        self,
        store: &mut EditorStore,
        configuration: IndentConfiguration,
        size: u32,
    ) -> Result<bool, EditorError> {
        let Ok(snapshot) = store.documents().snapshot(self.document) else {
            return Ok(false);
        };
        if snapshot.key != self.key {
            return Ok(false);
        }
        store.set_indentation(self.document, configuration, self.command.change(size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_model::ids::{PaneId, TabId};
    use taide_native_editor::indent::IndentOptions;
    use taide_native_editor::store::EditorLimits;
    use taide_native_editor::view::ViewKey;

    const BASE_SIZE: u32 = 4;
    const PICKED_SIZE: u32 = 2;
    const DISPLAY_SIZE: u32 = 8;
    const DOCUMENT_LIMIT: usize = 3;
    const VIEW_LIMIT: usize = 4;
    const BYTE_LIMIT: usize = 1024;

    fn configuration(size: u32) -> IndentConfiguration {
        IndentConfiguration {
            defaults: IndentOptions {
                tab_size: size,
                insert_spaces: true,
            },
            detect_indentation: false,
        }
    }

    fn store() -> EditorStore {
        EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: VIEW_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap()
    }

    #[test]
    fn 원래_뷰가_닫혀도_살아있는_mirror의_문서에_옵션만_적용한다() {
        let mut store = store();
        let document = store
            .open_untitled(TabId::new(), "  source", "plaintext".into())
            .unwrap();
        let other = store
            .open_untitled(TabId::new(), "target", "plaintext".into())
            .unwrap();
        let mut view_key = ViewKey {
            window: "main".into(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let view = store.attach_view(view_key.clone(), document).unwrap();
        view_key.pane = PaneId::new();
        let mirror = store.attach_view(view_key, document).unwrap();
        let config = configuration(BASE_SIZE);
        let (request, current) =
            Request::new(&mut store, document, config, IndentationCommand::Tabs).unwrap();
        assert_eq!(current, BASE_SIZE);
        let before = store.documents().snapshot(document).unwrap();
        store.detach_view(view).unwrap();
        assert!(request.apply(&mut store, config, PICKED_SIZE).unwrap());
        let after = store.documents().snapshot(document).unwrap();
        assert_eq!(
            after.model_indentation(config.defaults).tab_size,
            PICKED_SIZE
        );
        assert!(!after.model_indentation(config.defaults).insert_spaces);
        assert_eq!(after.rope, before.rope);
        assert_eq!(
            (after.revision, after.dirty),
            (before.revision, before.dirty)
        );
        assert_eq!(store.views().get(mirror).unwrap().document, document);
        assert_eq!(
            store.documents().snapshot(other).unwrap().indent_options,
            None
        );
    }

    #[test]
    fn 폐기_후_같은_키로_다시_연_문서에는_선택을_적용하지_않는다() {
        let mut store = store();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), "source", "plaintext".into())
            .unwrap();
        let config = configuration(BASE_SIZE);
        let (request, _) =
            Request::new(&mut store, document, config, IndentationCommand::Spaces).unwrap();
        let revision = store.documents().snapshot(document).unwrap().revision;
        store.discard_document(document, revision).unwrap();
        let replacement = store
            .open_untitled(tab, "replacement", "plaintext".into())
            .unwrap();
        assert_ne!(replacement, document);
        assert!(!request.apply(&mut store, config, PICKED_SIZE).unwrap());
        assert_eq!(
            store
                .documents()
                .snapshot(replacement)
                .unwrap()
                .indent_options,
            None
        );
    }

    #[test]
    fn 선택_중_변경한_설정을_되돌리지_않고_표시_폭만_변경한다() {
        let mut store = store();
        let document = store
            .open_untitled(TabId::new(), "source", "plaintext".into())
            .unwrap();
        let original = configuration(BASE_SIZE);
        let (request, _) =
            Request::new(&mut store, document, original, IndentationCommand::Spaces).unwrap();
        let latest = configuration(PICKED_SIZE);
        assert!(request.apply(&mut store, latest, BASE_SIZE).unwrap());
        assert_eq!(
            store
                .configure_indentation(document, latest)
                .unwrap()
                .tab_size,
            BASE_SIZE
        );
        let (request, _) =
            Request::new(&mut store, document, latest, IndentationCommand::Display).unwrap();
        assert!(request.apply(&mut store, latest, DISPLAY_SIZE).unwrap());
        let options = store
            .documents()
            .snapshot(document)
            .unwrap()
            .model_indentation(latest.defaults);
        assert_eq!(
            (options.tab_size, options.indent_size, options.insert_spaces),
            (DISPLAY_SIZE, BASE_SIZE, true)
        );
    }
}
