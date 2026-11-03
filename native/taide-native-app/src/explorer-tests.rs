use std::collections::HashMap;

use eframe::egui::{self, Color32, Event, Id, Modifiers, Rect, pos2, vec2};
use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::locale::ResolvedLocale;
use taide_model::paths::AppPaths;
use taide_model::tree::{TreeEntryKind, TreeRow, TreeRowPage};

use super::{Appearance, Explorer, Output, RowIcons};
use crate::explorer_git_decorations::Decoration;
use crate::problems_icons::Icons;

const SCREEN_WIDTH: f32 = 320.0;
const SCREEN_HEIGHT: f32 = 240.0;
const ROOT: &str = "/synthetic/root";
const DOT_RADIUS: f32 = 3.0;

fn locale() -> ResolvedLocale {
    ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    }
}

fn appearance() -> Appearance {
    let state = taide_runtime::AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-explorer-rows-{}", ProjectId::new())),
    ));
    let theme =
        taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
    Appearance::new(&theme).unwrap()
}

fn row(name: &str, kind: TreeEntryKind, depth: u32) -> TreeRow {
    let path = match depth {
        0 => format!("{ROOT}/{name}"),
        _ => format!("{ROOT}/src/{name}"),
    };
    TreeRow {
        path,
        name: name.into(),
        kind,
        depth,
        expanded: kind == TreeEntryKind::Directory,
        has_children: kind == TreeEntryKind::Directory,
    }
}

fn page(rows: Vec<TreeRow>) -> TreeRowPage {
    TreeRowPage {
        total: rows.len() as u32,
        rows,
    }
}

struct Frame {
    output: Output,
    full: egui::FullOutput,
}

fn draw(
    context: &egui::Context,
    explorer: &mut Explorer,
    (project, mount): (&ProjectId, &ShellSlotId),
    page: &TreeRowPage,
    (appearance, decorations): (&Appearance, Option<&HashMap<String, Decoration>>),
    events: Vec<Event>,
) -> Frame {
    let mut glyphs = Icons::new().unwrap();
    let mut output = None;
    let mut full = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            output = Some(explorer.show_with_icons(
                ui,
                project,
                mount,
                page,
                &locale(),
                RowIcons {
                    glyphs: &mut glyphs,
                    appearance,
                    decorations,
                },
            ));
        },
    );
    full.textures_delta.clear();
    Frame {
        output: output.unwrap(),
        full,
    }
}

fn row_fill(frame: &Frame, path: &str) -> Option<Color32> {
    let rect = frame.output.rows[path].rect;
    frame
        .full
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.rect == rect => Some(shape.fill),
            _ => None,
        })
}

fn texts(frame: &Frame) -> Vec<(String, Color32, Rect)> {
    frame
        .full
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) => Some((
                shape.galley.text().to_owned(),
                shape
                    .override_text_color
                    .or_else(|| shape.galley.job.sections.first().map(|s| s.format.color))
                    .unwrap_or(shape.fallback_color),
                shape.visual_bounding_rect(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn 행_바탕은_hover_primary_focus_선택_순서로_탐색기_토큰을_칠한다() {
    let appearance = appearance();
    let project = ProjectId::new();
    let mount = ShellSlotId::new();
    let page = page(vec![
        row("a.rs", TreeEntryKind::File, 0),
        row("b.rs", TreeEntryKind::File, 0),
        row("c.rs", TreeEntryKind::File, 0),
    ]);
    let (a, b, c) = (&page.rows[0].path, &page.rows[1].path, &page.rows[2].path);
    let mut explorer = Explorer::default();
    explorer.select_only(Some(a.clone()));
    explorer.select_row(&page.rows[1], &page.rows, Modifiers::COMMAND);
    assert_eq!(explorer.selected.as_ref(), Some(b));
    let context = egui::Context::default();
    let tree = Id::new(("native-tree-focus", &project));
    let inputs = (&project, &mount);
    let colors = (&appearance, None);
    draw(&context, &mut explorer, inputs, &page, colors, Vec::new());
    context.memory_mut(|memory| memory.request_focus(tree));
    let focused = draw(&context, &mut explorer, inputs, &page, colors, Vec::new());
    assert_eq!(row_fill(&focused, b), Some(appearance.item_selected));
    assert_eq!(row_fill(&focused, a), Some(appearance.item_focused));
    assert_eq!(row_fill(&focused, c), None);

    context.memory_mut(|memory| memory.surrender_focus(tree));
    let blurred = draw(&context, &mut explorer, inputs, &page, colors, Vec::new());
    assert_eq!(row_fill(&blurred, b), Some(appearance.item_focused));
    assert_eq!(row_fill(&blurred, a), Some(appearance.item_focused));

    context.memory_mut(|memory| memory.request_focus(tree));
    let hover = blurred.output.rows[b].rect.center();
    draw(
        &context,
        &mut explorer,
        inputs,
        &page,
        colors,
        vec![Event::PointerMoved(hover)],
    );
    let hovered = draw(
        &context,
        &mut explorer,
        inputs,
        &page,
        colors,
        vec![Event::PointerMoved(hover)],
    );
    assert_eq!(row_fill(&hovered, b), Some(appearance.item_hover));
    assert_eq!(row_fill(&hovered, a), Some(appearance.item_focused));
    assert_ne!(appearance.item_hover, appearance.item_selected);
    assert_ne!(appearance.item_selected, appearance.item_focused);
}

#[test]
fn git_데코레이션은_이름_chevron_색과_배지와_디렉터리_점과_접근성_설명을_그린다() {
    let appearance = appearance();
    let project = ProjectId::new();
    let mount = ShellSlotId::new();
    let page = page(vec![
        row("src", TreeEntryKind::Directory, 0),
        row("new.rs", TreeEntryKind::File, 1),
        row("other.rs", TreeEntryKind::File, 0),
    ]);
    let (src, new, other) = (&page.rows[0].path, &page.rows[1].path, &page.rows[2].path);
    let decorations = HashMap::from([
        (src.clone(), Decoration::Untracked),
        (new.clone(), Decoration::Untracked),
        (other.clone(), Decoration::Modified),
    ]);
    let untracked = appearance.git_color(Decoration::Untracked);
    let modified = appearance.git_color(Decoration::Modified);
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut explorer = Explorer::default();
    let inputs = (&project, &mount);
    draw(
        &context,
        &mut explorer,
        inputs,
        &page,
        (&appearance, Some(&decorations)),
        Vec::new(),
    );
    let frame = draw(
        &context,
        &mut explorer,
        inputs,
        &page,
        (&appearance, Some(&decorations)),
        Vec::new(),
    );
    let texts = texts(&frame);
    let text = |name: &str, within: Rect| {
        texts
            .iter()
            .find(|(text, _, rect)| text == name && within.intersects(*rect))
            .cloned()
            .unwrap_or_else(|| panic!("{name}: {texts:?}"))
    };
    let new_row = frame.output.rows[new].rect;
    let (_, new_color, new_name) = text("new.rs", new_row);
    let (_, badge_color, badge) = text("U", new_row);
    assert_eq!(new_color, untracked);
    assert_eq!(badge_color, untracked);
    assert!(new_name.right() <= badge.left(), "{new_name:?} {badge:?}");
    assert!(badge.right() <= new_row.right() - super::ROW_RIGHT_PADDING + 1.0);
    let other_row = frame.output.rows[other].rect;
    assert_eq!(text("other.rs", other_row).1, modified);
    assert_eq!(text("M", other_row).1, modified);
    let src_row = frame.output.rows[src].rect;
    assert_eq!(text("src", src_row).1, untracked);
    assert!(frame.full.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::Circle(circle)
            if circle.radius == DOT_RADIUS && circle.fill == untracked && src_row.contains(circle.center)
    )));
    assert!(
        !texts
            .iter()
            .any(|(text, _, rect)| text == "U" && src_row.intersects(*rect))
    );
    let update = frame.full.platform_output.accesskit_update.unwrap();
    let described = |name: &str| {
        update.nodes.iter().find_map(|(_, node)| {
            (node.label() == Some(name)).then(|| node.description().map(str::to_owned))
        })
    };
    assert_eq!(described("new.rs"), Some(Some("untracked".into())));
    assert_eq!(described("other.rs"), Some(Some("modified".into())));

    let plain = draw(
        &context,
        &mut explorer,
        inputs,
        &page,
        (&appearance, None),
        Vec::new(),
    );
    let texts_without = self::texts(&plain);
    assert!(
        !texts_without
            .iter()
            .any(|(text, _, _)| text == "U" || text == "M")
    );
    assert!(
        texts_without
            .iter()
            .any(|(text, color, _)| text == "new.rs" && *color != untracked)
    );
}
