use std::ops::Range;

use ropey::Rope;

mod lsp_coordinates;

pub const LARGE_DOCUMENT_LINES: usize = 50_000;
pub const FONT_SIZE: f32 = 16.0;
pub const LINE_HEIGHT: f32 = 22.0;
pub const VIEW_WIDTH: f32 = 900.0;
pub const VIEW_HEIGHT: f32 = 440.0;
pub const MAX_VISIBLE_RUNS: usize = 24;

pub struct Edit {
    pub bytes: Range<usize>,
    pub text: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidBoundary,
    Overlap,
    StaleRevision,
    RevisionOverflow,
}

pub struct DocumentProbe {
    pub rope: Rope,
    pub revision: u64,
    undo: Vec<Rope>,
    redo: Vec<Rope>,
}

impl DocumentProbe {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn byte_to_scalar(&self, byte: usize) -> Result<usize, EditError> {
        let scalar = self
            .rope
            .try_byte_to_char(byte)
            .map_err(|_| EditError::InvalidBoundary)?;
        if self.rope.char_to_byte(scalar) != byte {
            return Err(EditError::InvalidBoundary);
        }
        Ok(scalar)
    }

    pub fn utf16_to_scalar(&self, unit: usize) -> Result<usize, EditError> {
        let scalar = self
            .rope
            .try_utf16_cu_to_char(unit)
            .map_err(|_| EditError::InvalidBoundary)?;
        if self.rope.char_to_utf16_cu(scalar) != unit {
            return Err(EditError::InvalidBoundary);
        }
        Ok(scalar)
    }

    pub fn apply(&mut self, revision: u64, mut edits: Vec<Edit>) -> Result<(), EditError> {
        if revision != self.revision {
            return Err(EditError::StaleRevision);
        }
        if edits.is_empty() {
            return Ok(());
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(EditError::RevisionOverflow)?;
        edits.sort_by_key(|edit| edit.bytes.start);
        for edit in &edits {
            if edit.bytes.start > edit.bytes.end {
                return Err(EditError::InvalidBoundary);
            }
            self.byte_to_scalar(edit.bytes.start)?;
            self.byte_to_scalar(edit.bytes.end)?;
        }
        for pair in edits.windows(2) {
            if pair[0].bytes.end > pair[1].bytes.start || pair[0].bytes.start == pair[1].bytes.start
            {
                return Err(EditError::Overlap);
            }
        }
        let before = self.rope.clone();
        for edit in edits.iter().rev() {
            let start = self.rope.byte_to_char(edit.bytes.start);
            let end = self.rope.byte_to_char(edit.bytes.end);
            self.rope.remove(start..end);
            self.rope.insert(start, &edit.text);
        }
        self.undo.push(before);
        self.redo.clear();
        self.revision = next_revision;
        Ok(())
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        if self.undo.is_empty() {
            return Ok(false);
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(EditError::RevisionOverflow)?;
        let previous = self.undo.pop().expect("nonempty undo history");
        self.redo.push(std::mem::replace(&mut self.rope, previous));
        self.revision = next_revision;
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        if self.redo.is_empty() {
            return Ok(false);
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(EditError::RevisionOverflow)?;
        let next = self.redo.pop().expect("nonempty redo history");
        self.undo.push(std::mem::replace(&mut self.rope, next));
        self.revision = next_revision;
        Ok(true)
    }
}

pub fn large_fixture() -> String {
    (0..LARGE_DOCUMENT_LINES)
        .map(|line| format!("let fixture_{line} = \"한글 日本語 中文\";\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Scroll, Shaping, Wrap, fontdb};
    use tree_sitter::{InputEdit, Parser, Point};
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn byte_scalar_utf16_grapheme와_줄_인덱스를_검사한다() {
        let text = "a한글\u{1f600}e\u{301}\r\n日本語\n";
        let document = DocumentProbe::new(text);
        for scalar in 0..=document.rope.len_chars() {
            let byte = document.rope.char_to_byte(scalar);
            let utf16 = document.rope.char_to_utf16_cu(scalar);
            assert_eq!(document.byte_to_scalar(byte), Ok(scalar));
            assert_eq!(document.utf16_to_scalar(utf16), Ok(scalar));
        }
        assert_eq!(document.byte_to_scalar(2), Err(EditError::InvalidBoundary));
        assert_eq!(document.utf16_to_scalar(4), Err(EditError::InvalidBoundary));
        assert!(document.byte_to_scalar(text.len() + 1).is_err());
        assert!(
            document
                .utf16_to_scalar(document.rope.len_utf16_cu() + 1)
                .is_err()
        );
        assert_eq!(document.rope.line(1).to_string(), "日本語\n");
        let graphemes = text.graphemes(true).collect::<Vec<_>>();
        assert!(graphemes.contains(&"e\u{301}"));
        assert!(graphemes.contains(&"\r\n"));
    }

    #[test]
    fn 다중_edit는_원자적이고_undo_redo_revision을_보존한다() {
        let mut document = DocumentProbe::new("abc def");
        let edits = vec![
            Edit {
                bytes: 0..3,
                text: "한글".to_string(),
            },
            Edit {
                bytes: 4..7,
                text: "日本語".to_string(),
            },
        ];
        document.apply(0, edits).unwrap();
        assert_eq!(document.rope.to_string(), "한글 日本語");
        assert_eq!(document.revision, 1);
        let before = document.rope.to_string();
        assert_eq!(
            document.apply(
                0,
                vec![Edit {
                    bytes: 0..0,
                    text: "stale".to_string()
                }]
            ),
            Err(EditError::StaleRevision)
        );
        assert_eq!(
            document.apply(
                1,
                vec![
                    Edit {
                        bytes: 0..0,
                        text: "valid".to_string()
                    },
                    Edit {
                        bytes: 1..2,
                        text: "invalid".to_string()
                    }
                ]
            ),
            Err(EditError::InvalidBoundary)
        );
        assert_eq!(document.rope.to_string(), before);
        assert_eq!(document.revision, 1);
        assert!(document.undo().unwrap());
        assert_eq!(document.rope.to_string(), "abc def");
        assert_eq!(document.revision, 2);
        assert!(document.redo().unwrap());
        assert_eq!(document.rope.to_string(), before);
        assert_eq!(document.revision, 3);
        assert_eq!(
            document.apply(
                3,
                vec![
                    Edit {
                        bytes: 0..0,
                        text: "first".to_string()
                    },
                    Edit {
                        bytes: 0..0,
                        text: "second".to_string()
                    }
                ]
            ),
            Err(EditError::Overlap)
        );
        document
            .apply(
                3,
                vec![Edit {
                    bytes: 0..0,
                    text: "fresh".to_string(),
                }],
            )
            .unwrap();
        assert!(!document.redo().unwrap());
    }

    #[test]
    fn tree_sitter_incremental_edit는_전체_파싱과_동일하다() {
        let old = "fn sample() { let value = 1; }";
        let new = "fn sample() { let value = 100; }";
        let start = old.find("1;").unwrap();
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let mut tree = parser.parse(old, None).unwrap();
        tree.edit(&InputEdit {
            start_byte: start,
            old_end_byte: start + 1,
            new_end_byte: start + 3,
            start_position: Point::new(0, start),
            old_end_position: Point::new(0, start + 1),
            new_end_position: Point::new(0, start + 3),
        });
        let incremental = parser.parse(new, Some(&tree)).unwrap();
        let full = parser.parse(new, None).unwrap();
        assert!(!incremental.root_node().has_error());
        assert_eq!(
            incremental.root_node().to_sexp(),
            full.root_node().to_sexp()
        );
    }

    #[test]
    fn cosmic_shaping은_cjk_bidi_cluster와_50000줄_viewport를_처리한다() {
        const FONT_PATHS: &[&str] = &[
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/Apple Color Emoji.ttc",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ];
        let mut database = fontdb::Database::new();
        for path in FONT_PATHS {
            database.load_font_file(path).unwrap();
        }
        let mut fonts = FontSystem::new_with_locale_and_db("en-US".to_string(), database);
        let mut buffer = Buffer::new(&mut fonts, Metrics::new(FONT_SIZE, LINE_HEIGHT));
        buffer.set_size(Some(VIEW_WIDTH), Some(VIEW_HEIGHT));
        buffer.set_wrap(Wrap::None);
        let text = "한글 日本語 中文 e\u{301} \u{1f600} אבג";
        buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut fonts, true);
        let run = buffer.layout_runs().next().unwrap();
        assert!(!run.glyphs.is_empty());
        assert!(run.glyphs.iter().all(|glyph| glyph.glyph_id != 0));
        assert!(run.glyphs.iter().all(|glyph| text.is_char_boundary(glyph.start) && text.is_char_boundary(glyph.end)));
        assert!(run.glyphs.iter().any(|glyph| glyph.level.is_rtl()));
        let first = &run.glyphs[0];
        let hit = buffer
            .hit(first.x, run.line_top + run.line_height / 2.0)
            .unwrap();
        assert!(text.is_char_boundary(hit.index));
        buffer.set_text(&large_fixture(), &Attrs::new(), Shaping::Advanced, None);
        for line in [0, LARGE_DOCUMENT_LINES / 2, LARGE_DOCUMENT_LINES - 1] {
            buffer.set_scroll(Scroll::new(line, 0.0, 0.0));
            buffer.shape_until_scroll(&mut fonts, true);
            let runs = buffer.layout_runs().collect::<Vec<_>>();
            assert!(!runs.is_empty());
            assert!(
                runs.len() <= MAX_VISIBLE_RUNS,
                "visible runs {}",
                runs.len()
            );
            assert!(runs[0].line_i.abs_diff(line) < MAX_VISIBLE_RUNS);
        }
    }
}
