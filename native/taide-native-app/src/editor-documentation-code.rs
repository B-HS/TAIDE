use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use eframe::egui::{Color32, FontId, Stroke, TextFormat, text::LayoutJob};
use taide_model::ids::TabId;
use taide_model::plugin::LoadedPlugin;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::documentation::{Block, RichDocument};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::store::EditorStore;
use taide_native_ui::editor_surface::EditorAppearance;

use crate::editor_syntax::{EditorSyntax, PeekTokens};

const SPAN_FIELDS: usize = 2;
const DECORATION_WIDTH: f32 = 1.0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    language: String,
    text: String,
}

struct Snippet {
    document: DocumentId,
    snapshot: DocumentSnapshot,
    tokens: Option<Arc<PeekTokens>>,
}

#[derive(Default)]
pub(crate) struct Cache {
    snippets: HashMap<Key, Snippet>,
    aliases: HashMap<String, String>,
}

fn builtin_aliases() -> &'static HashMap<String, String> {
    static ALIASES: OnceLock<HashMap<String, String>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        let mut aliases: HashMap<String, String> =
            serde_json::from_str(include_str!("editor-documentation-language-aliases.json"))
                .expect("bundled Monaco language aliases are valid");
        for id in taide_native_syntax::bundled_language_ids().unwrap_or_default() {
            aliases.insert(id.to_lowercase(), id.into());
        }
        aliases
    })
}

impl Cache {
    fn key(&self, language: &str, text: &str, fallback: &str) -> Key {
        let language = if language.is_empty() {
            fallback.into()
        } else {
            self.aliases
                .get(&language.to_lowercase())
                .cloned()
                .unwrap_or_else(|| "plaintext".into())
        };
        Key {
            language,
            text: text.into(),
        }
    }

    pub(crate) fn prepare<'a>(
        &mut self,
        store: &mut EditorStore,
        syntax: &mut EditorSyntax,
        documents: impl Iterator<Item = (&'a RichDocument, &'a str)>,
        plugins: &[LoadedPlugin],
    ) -> Result<(), EditorError> {
        self.aliases.clone_from(builtin_aliases());
        for language in plugins
            .iter()
            .filter(|plugin| plugin.enabled)
            .flat_map(|plugin| &plugin.manifest.contributes.languages)
        {
            for alias in std::iter::once(&language.id).chain(&language.aliases) {
                self.aliases
                    .insert(alias.to_lowercase(), language.id.clone());
            }
        }
        let mut keys = HashSet::new();
        for (document, language) in documents {
            self.collect(&document.blocks, language, &mut keys);
        }
        let retired = self
            .snippets
            .iter()
            .filter(|(key, _)| !keys.contains(*key))
            .map(|(key, snippet)| (key.clone(), snippet.document, snippet.snapshot.revision))
            .collect::<Vec<_>>();
        for (key, document, revision) in retired {
            store.discard_document(document, revision)?;
            self.snippets.remove(&key);
        }
        for key in keys {
            if !self.snippets.contains_key(&key) {
                let document =
                    store.open_untitled(TabId::new(), &key.text, key.language.clone())?;
                self.snippets.insert(
                    key.clone(),
                    Snippet {
                        document,
                        snapshot: store.documents().snapshot(document)?,
                        tokens: None,
                    },
                );
            }
            let snippet = self.snippets.get_mut(&key).ok_or(EditorError::NotFound)?;
            snippet.tokens = syntax.peek_tokens(store, snippet.document);
            syntax.show_lines(snippet.document, 0..snippet.snapshot.rope.len_lines());
        }
        Ok(())
    }

    pub(crate) fn job(
        &self,
        language: &str,
        text: &str,
        fallback: &str,
        appearance: &EditorAppearance,
    ) -> Option<LayoutJob> {
        let snippet = self.snippets.get(&self.key(language, text, fallback))?;
        let tokens = snippet.tokens.as_ref().map(|tokens| tokens.frame());
        let mut job = LayoutJob::default();
        for line_index in 0..snippet.snapshot.rope.len_lines() {
            if line_index > 0 {
                job.append(
                    "\n",
                    0.0,
                    TextFormat {
                        font_id: appearance.font.clone(),
                        color: appearance.foreground,
                        ..Default::default()
                    },
                );
            }
            let line = snippet
                .snapshot
                .rope
                .byte_slice(line_content_range(&snippet.snapshot, line_index))
                .to_string();
            let spans = tokens
                .as_ref()
                .filter(|tokens| tokens.lines.has_accurate_tokens(line_index))
                .map(|tokens| tokens.lines.spans(line_index).as_chunks::<SPAN_FIELDS>().0)
                .unwrap_or_default();
            let mut boundaries = spans
                .iter()
                .filter_map(|[byte, _]| {
                    let byte = *byte as usize;
                    (byte > 0 && byte < line.len() && line.is_char_boundary(byte)).then_some(byte)
                })
                .collect::<Vec<_>>();
            boundaries.insert(0, 0);
            boundaries.push(line.len());
            for pair in boundaries.windows(SPAN_FIELDS) {
                let style = tokens.as_ref().map(|tokens| {
                    spans
                        .iter()
                        .rev()
                        .find(|[byte, _]| *byte as usize <= pair[0])
                        .map_or(tokens.styles.default_style(), |[_, style]| {
                            tokens.styles.style(*style)
                        })
                });
                let color = style.map_or(appearance.foreground, |style| {
                    Color32::from_rgba_unmultiplied(
                        style.foreground[0],
                        style.foreground[1],
                        style.foreground[2],
                        style.foreground[3],
                    )
                });
                let font_id = style.filter(|style| style.is_bold).map_or_else(
                    || appearance.font.clone(),
                    |_| {
                        FontId::new(
                            appearance.font.size,
                            eframe::egui::FontFamily::Name(
                                taide_native_ui::font_families::EDITOR_BOLD_FAMILY.into(),
                            ),
                        )
                    },
                );
                let decoration = Stroke::new(DECORATION_WIDTH, color);
                job.append(
                    &line[pair[0]..pair[1]],
                    0.0,
                    TextFormat {
                        font_id,
                        color,
                        italics: style.is_some_and(|style| style.is_italic),
                        underline: if style.is_some_and(|style| style.is_underlined) {
                            decoration
                        } else {
                            Stroke::NONE
                        },
                        strikethrough: if style.is_some_and(|style| style.is_struck_through) {
                            decoration
                        } else {
                            Stroke::NONE
                        },
                        line_height: Some(appearance.line_height),
                        ..Default::default()
                    },
                );
            }
        }
        Some(job)
    }

    fn collect(&self, blocks: &[Block], fallback: &str, keys: &mut HashSet<Key>) {
        for block in blocks {
            match block {
                Block::Code { language, text } => {
                    keys.insert(self.key(language, text, fallback));
                }
                Block::Quote(blocks) => self.collect(blocks, fallback, keys),
                Block::List { items, .. } => {
                    for item in items {
                        self.collect(&item.blocks, fallback, keys);
                    }
                }
                _ => {}
            }
        }
    }
}
