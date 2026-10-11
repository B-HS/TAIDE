use egui::{Pos2, ViewportId};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Command(&'static str),
    Cut,
    Copy,
    Paste,
    ToggleMinimap,
    RunSelectedText,
}

impl Action {
    pub fn id(self) -> &'static str {
        match self {
            Self::Command(id) => id,
            Self::Cut => "editor.action.clipboardCutAction",
            Self::Copy => "editor.action.clipboardCopyAction",
            Self::Paste => "editor.action.clipboardPasteAction",
            Self::ToggleMinimap => "taide.toggleMinimap",
            Self::RunSelectedText => "taide.runSelectedTextInTerminal",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub action: Action,
    pub label: String,
    pub shortcut: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Item(Item),
    Submenu { label: String, items: Vec<Item> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: &'static str,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Copy, Default)]
pub struct Context {
    pub read_only: bool,
    pub embedded: bool,
    pub composite: bool,
    pub has_selection: bool,
    pub definition: bool,
    pub declaration: bool,
    pub type_definition: bool,
    pub implementation: bool,
    pub references: bool,
    pub rename: bool,
    pub document_formatting: bool,
    pub range_formatting: bool,
    pub refactor: bool,
    pub source_actions: bool,
}

pub fn groups(
    context: Context,
    peek_label: String,
    mut resolve: impl FnMut(Action, &'static str) -> (String, Option<String>),
) -> Vec<Group> {
    let mut item = |action, label| {
        let (label, shortcut) = resolve(action, label);
        Item {
            action,
            label,
            shortcut,
        }
    };
    let mut navigation = Vec::new();
    let mut peek = Vec::new();
    for (available, command, label, peek_command, peek_title) in [
        (
            context.definition,
            "editor.action.revealDefinition",
            "Go to Definition",
            "editor.action.peekDefinition",
            "Peek Definition",
        ),
        (
            context.declaration && !context.embedded,
            "editor.action.revealDeclaration",
            "Go to Declaration",
            "editor.action.peekDeclaration",
            "Peek Declaration",
        ),
        (
            context.type_definition,
            "editor.action.goToTypeDefinition",
            "Go to Type Definition",
            "editor.action.peekTypeDefinition",
            "Peek Type Definition",
        ),
        (
            context.implementation,
            "editor.action.goToImplementation",
            "Go to Implementations",
            "editor.action.peekImplementation",
            "Peek Implementations",
        ),
        (
            context.references && !context.embedded,
            "editor.action.goToReferences",
            "Go to References",
            "editor.action.referenceSearch.trigger",
            "Peek References",
        ),
    ] {
        if available {
            navigation.push(Entry::Item(item(Action::Command(command), label)));
        }
        if available && !context.embedded {
            peek.push(item(Action::Command(peek_command), peek_title));
        }
    }
    if !context.embedded {
        navigation.push(Entry::Item(item(Action::ToggleMinimap, "Toggle Minimap")));
    }
    if !peek.is_empty() {
        navigation.push(Entry::Submenu {
            label: peek_label,
            items: peek,
        });
    }
    let mut modification = Vec::new();
    if !context.read_only {
        for (available, command, label) in [
            (context.rename, "editor.action.rename", "Rename Symbol"),
            (true, "editor.action.changeAll", "Change All Occurrences"),
            (
                context.document_formatting && !context.composite,
                "editor.action.formatDocument",
                "Format Document",
            ),
            (
                context.range_formatting && context.has_selection,
                "editor.action.formatSelection",
                "Format Selection",
            ),
            (context.refactor, "editor.action.refactor", "Refactor..."),
            (
                context.source_actions,
                "editor.action.sourceAction",
                "Source Action...",
            ),
        ] {
            if available {
                modification.push(Entry::Item(item(Action::Command(command), label)));
            }
        }
    }
    let mut clipboard = Vec::new();
    if !context.read_only {
        clipboard.push(Entry::Item(item(Action::Cut, "Cut")));
    }
    clipboard.push(Entry::Item(item(Action::Copy, "Copy")));
    if !context.read_only {
        clipboard.push(Entry::Item(item(Action::Paste, "Paste")));
    }
    [
        Group {
            id: "navigation",
            entries: navigation,
        },
        Group {
            id: "1_modification",
            entries: modification,
        },
        Group {
            id: "9_cutcopypaste",
            entries: clipboard,
        },
        Group {
            id: "9_terminal",
            entries: if context.embedded {
                Vec::new()
            } else {
                vec![Entry::Item(item(
                    Action::RunSelectedText,
                    "Run Selected Text in Terminal",
                ))]
            },
        },
    ]
    .into_iter()
    .filter(|group| !group.entries.is_empty())
    .collect()
}

pub fn right_click(
    store: &mut EditorStore,
    source: ViewId,
    head: usize,
) -> Result<(), EditorError> {
    let view = store.views().get(source).ok_or(EditorError::NotFound)?;
    let document = store.documents().snapshot(view.document)?;
    if head > document.rope.len_bytes()
        || document.rope.char_to_byte(document.rope.byte_to_char(head)) != head
    {
        return Err(EditorError::InvalidBoundary);
    }
    if view.selection.selections.iter().any(|selection| {
        (selection.anchor.min(selection.head)..=selection.anchor.max(selection.head))
            .contains(&head)
    }) {
        return Ok(());
    }
    let scroll = view.scroll.clone();
    let folds = view.folds.clone();
    store.set_view_state(
        source,
        SelectionSet {
            primary: 0,
            selections: vec![Selection { anchor: head, head }],
        },
        scroll,
        folds,
    )
}

#[derive(Clone)]
pub struct Session {
    pub source: ViewId,
    pub owner: ViewId,
    pub viewport: ViewportId,
    pub anchor: Pos2,
    pub groups: Vec<Group>,
    owner_key: ViewKey,
    document: DocumentSnapshot,
    selection: SelectionSet,
    scroll: ScrollPosition,
    composing: bool,
}

impl Session {
    pub fn open(
        store: &EditorStore,
        source: ViewId,
        owner: ViewId,
        viewport: ViewportId,
        anchor: Pos2,
        groups: Vec<Group>,
    ) -> Result<Self, EditorError> {
        let view = store.views().get(source).ok_or(EditorError::NotFound)?;
        let owner_key = store
            .views()
            .get(owner)
            .ok_or(EditorError::NotFound)?
            .key
            .clone();
        Ok(Self {
            source,
            owner,
            viewport,
            anchor,
            groups,
            owner_key,
            document: store.documents().snapshot(view.document)?,
            selection: view.selection.clone(),
            scroll: view.scroll.clone(),
            composing: view.composition.is_some(),
        })
    }

    pub fn matches(
        &self,
        store: &EditorStore,
        source: ViewId,
        owner: ViewId,
        viewport: ViewportId,
    ) -> bool {
        if source != self.source || owner != self.owner || viewport != self.viewport {
            return false;
        }
        let Some(view) = store.views().get(source) else {
            return false;
        };
        let Some(owner) = store.views().get(owner) else {
            return false;
        };
        let Ok(document) = store.documents().snapshot(view.document) else {
            return false;
        };
        owner.key == self.owner_key
            && document.id == self.document.id
            && document.key == self.document.key
            && document.revision == self.document.revision
            && document.metadata.language_id == self.document.metadata.language_id
            && document.metadata.tier == self.document.metadata.tier
            && document.metadata.read_only == self.document.metadata.read_only
            && document.metadata.lossy == self.document.metadata.lossy
            && view.selection == self.selection
            && view.scroll.x == self.scroll.x
            && view.scroll.y == self.scroll.y
            && view.composition.is_some() == self.composing
    }
}
