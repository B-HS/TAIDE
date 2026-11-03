use eframe::egui::{Event, Key, Modifiers, PointerButton, pos2};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::search::{SearchFileMatches, SearchLineMatch};

use super::*;

const SCREEN: egui::Vec2 = egui::vec2(320.0, 480.0);
const FRAME_SECONDS: f64 = 1.0 / 60.0;
const DEBOUNCE_MS: u32 = 300;
const BEFORE_DEBOUNCE_SECONDS: f64 = 0.2;
const AFTER_DEBOUNCE_SECONDS: f64 = 0.4;

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

fn appearance() -> crate::explorer::Appearance {
    let state = taide_runtime::AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-search-panel-{}", ProjectId::new())),
    ));
    let theme =
        taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap();
    crate::explorer::Appearance::new(&theme).unwrap()
}

struct Harness {
    context: egui::Context,
    time: f64,
    panel: Panel,
    streams: Streams,
    appearance: crate::explorer::Appearance,
    locale: ResolvedLocale,
    settings: Settings,
    history: Vec<String>,
    request: Option<Request>,
    runs: Vec<Run>,
    last: Option<egui::FullOutput>,
    opens: Vec<Open>,
    replaces: Vec<Replace>,
    editors: Vec<SearchQuery>,
    cleared: usize,
}

impl Harness {
    fn new(is_live: bool) -> Self {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut harness = Self {
            context,
            time: 0.0,
            panel: Panel::new(Session {
                owner: "native-main".into(),
                id: "search-panel-test".into(),
            }),
            streams: Streams::default(),
            appearance: appearance(),
            locale: locale(),
            settings: Settings {
                is_live,
                debounce_ms: DEBOUNCE_MS,
            },
            history: Vec::new(),
            request: None,
            runs: Vec::new(),
            last: None,
            opens: Vec::new(),
            replaces: Vec::new(),
            editors: Vec::new(),
            cleared: 0,
        };
        harness.frame(Vec::new());
        harness
    }

    fn frame(&mut self, events: Vec<Event>) -> &egui::FullOutput {
        self.time += FRAME_SECONDS;
        let mut result = None;
        let mut output = self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                let mut icons = crate::navigation_icons::Deferred::default();
                result = Some(
                    self.panel
                        .show(
                            ui,
                            Id::new("search-panel-test"),
                            Inputs {
                                request: self.request.as_ref(),
                                nonce: 0,
                                settings: self.settings,
                                history: &self.history,
                                streams: &self.streams,
                                appearance: &self.appearance,
                                locale: &self.locale,
                            },
                            &mut icons,
                        )
                        .unwrap(),
                );
            },
        );
        output.textures_delta.clear();
        let result = result.unwrap();
        self.runs.extend(result.run);
        self.opens.extend(result.open);
        self.replaces.extend(result.replace);
        self.editors.extend(result.open_editor);
        self.cleared += usize::from(result.clear_scope);
        self.last = Some(output);
        self.last.as_ref().unwrap()
    }

    fn wait(&mut self, seconds: f64) {
        let until = self.time + seconds;
        while self.time < until {
            self.frame(Vec::new());
        }
    }

    fn typed(&mut self, text: &str) {
        for character in text.chars() {
            self.frame(vec![Event::Text(character.to_string())]);
        }
    }

    fn enter(&mut self) {
        self.frame(
            [true, false]
                .into_iter()
                .map(|pressed| Event::Key {
                    key: Key::Enter,
                    physical_key: Some(Key::Enter),
                    pressed,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                })
                .collect(),
        );
    }

    fn node(&self, label: &str) -> Option<(egui::Rect, Option<bool>, bool)> {
        self.last
            .as_ref()?
            .platform_output
            .accesskit_update
            .as_ref()?
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label))
            .and_then(|(_, node)| {
                let bounds = node.bounds()?;
                Some((
                    egui::Rect::from_min_max(
                        pos2(bounds.x0 as f32, bounds.y0 as f32),
                        pos2(bounds.x1 as f32, bounds.y1 as f32),
                    ),
                    node.toggled()
                        .map(|toggled| toggled == egui::accesskit::Toggled::True),
                    node.is_disabled(),
                ))
            })
    }

    fn click(&mut self, key: &str) {
        self.click_label(&message(&self.locale, key));
    }

    fn click_label(&mut self, label: &str) {
        let (rect, ..) = self.node(label).unwrap_or_else(|| panic!("{label}"));
        let button = |pressed| Event::PointerButton {
            pos: rect.center(),
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(rect.center()), button(true)]);
        self.frame(vec![button(false)]);
    }

    fn texts(&self) -> Vec<String> {
        self.last
            .as_ref()
            .map(|output| {
                output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn has_text(&self, key: &str) -> bool {
        self.texts().contains(&message(&self.locale, key))
    }
}

fn message(locale: &ResolvedLocale, key: &str) -> String {
    crate::presentation::message(locale, key, &[])
}

fn found(line: u32) -> SearchLineMatch {
    SearchLineMatch {
        line,
        column: 3,
        preview: "a needle".into(),
        match_start: 2,
        match_end: 8,
        before: Vec::new(),
        after: Vec::new(),
    }
}

#[test]
fn 아직_실행하지_않았으면_0건이_아니라_안내를_보여준다() {
    assert_eq!(resolve_view(Status::Idle, false), ResultsView::Hint);
}

#[test]
fn 실패한_검색을_0건과_구분한다() {
    assert_eq!(resolve_view(Status::Failed, false), ResultsView::Failed);
}

#[test]
fn 완료했는데_결과가_없으면_0건이다() {
    assert_eq!(resolve_view(Status::Completed, false), ResultsView::Empty);
}

#[test]
fn 실행_중이고_아직_결과가_없으면_검색_중이다() {
    assert_eq!(resolve_view(Status::Running, false), ResultsView::Searching);
}

#[test]
fn 결과가_도착했으면_실행_중이어도_결과를_보여준다() {
    assert_eq!(resolve_view(Status::Running, true), ResultsView::Results);
    assert_eq!(resolve_view(Status::Completed, true), ResultsView::Results);
}

#[test]
fn 상한_미만은_잘리지_않은_것이다() {
    assert!(!is_truncated(0));
    assert!(!is_truncated(SEARCH_MATCH_LIMIT - 1));
}

#[test]
fn 상한에_도달하면_잘린_것으로_본다() {
    assert!(is_truncated(SEARCH_MATCH_LIMIT));
}

#[test]
fn 머리_영역은_24px_버튼과_입력_상자_테두리와_아래_구분선을_원본_토큰으로_그린다() {
    let mut harness = Harness::new(false);
    let output = harness.frame(Vec::new()).clone();
    let colors = harness.appearance.search();
    for key in [
        "search.replaceToggle",
        "search.recentSearches",
        "search.caseSensitive",
        "search.wholeWord",
        "search.regex",
        "search.respectGitignore",
    ] {
        let (rect, ..) = harness
            .node(&message(&harness.locale, key))
            .unwrap_or_else(|| panic!("{key}"));
        assert_eq!(rect.size(), egui::vec2(BUTTON_SIZE, BUTTON_SIZE), "{key}");
    }
    let mut boxes = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.stroke.width == BORDER_WIDTH && shape.rect.height() == INPUT_HEIGHT =>
            {
                Some((shape.rect, shape.stroke.color))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    boxes.sort_by(|left, right| left.0.top().total_cmp(&right.0.top()));
    let [(query_box, query_border), (exclude_box, exclude_border)] = boxes[..] else {
        panic!("{boxes:?}");
    };
    assert_eq!(query_border, colors.focus_border);
    assert_eq!(exclude_border, colors.input_border);
    assert_eq!(exclude_box.top(), query_box.bottom() + COLUMN_GAP);
    let filled = |rect: egui::Rect, fill: Color32| {
        output.shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.rect == rect && shape.fill == fill)
        })
    };
    assert!(filled(query_box, colors.input_background));
    assert!(filled(exclude_box, colors.input_background));
    assert!(output.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::LineSegment { points, stroke }
            if stroke.color == colors.border
                && points[0].y == exclude_box.bottom() + PADDING_Y
    )));
    assert!(output.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::Rect(shape) if shape.fill == colors.background
            && shape.rect.top() == query_box.top() - PADDING_Y
    )));
}

#[test]
fn 설정이_켜져_있으면_입력이_멈춘_뒤에야_검색이_돈다() {
    let mut harness = Harness::new(true);
    harness.typed("foo");
    harness.wait(BEFORE_DEBOUNCE_SECONDS);
    assert!(harness.runs.is_empty());
    harness.wait(AFTER_DEBOUNCE_SECONDS - BEFORE_DEBOUNCE_SECONDS);
    assert_eq!(harness.runs.len(), 1);
    assert_eq!(harness.runs[0].query.text, "foo");
}

#[test]
fn 연속_입력은_마지막_한_번만_실행한다() {
    let mut harness = Harness::new(true);
    harness.typed("f");
    harness.wait(BEFORE_DEBOUNCE_SECONDS);
    harness.typed("oo");
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert_eq!(harness.runs.len(), 1);
    assert_eq!(harness.runs[0].query.text, "foo");
}

#[test]
fn 옵션을_바꿔도_같은_디바운스로_재실행한다() {
    let mut harness = Harness::new(true);
    harness.typed("foo");
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    harness.click("search.caseSensitive");
    harness.wait(BEFORE_DEBOUNCE_SECONDS);
    assert_eq!(harness.runs.len(), 1);
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert_eq!(harness.runs.len(), 2);
    assert!(harness.runs[1].query.case_sensitive);
}

#[test]
fn 실시간_실행은_검색_이력을_쌓지_않는다() {
    let mut harness = Harness::new(true);
    harness.typed("foo");
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert_eq!(harness.runs.len(), 1);
    assert_eq!(harness.runs[0].record, None);
}

#[test]
fn enter_는_즉시_실행하고_대기_중인_디바운스를_취소하며_이력을_남긴다() {
    let mut harness = Harness::new(true);
    harness.typed("foo");
    harness.enter();
    assert_eq!(harness.runs.len(), 1);
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert_eq!(harness.runs.len(), 1);
    assert_eq!(harness.runs[0].record.as_deref(), Some("foo"));
}

#[test]
fn 실시간_실행의_실패는_토스트_없이_패널_안에서만_알린다() {
    let mut harness = Harness::new(true);
    harness.typed("(foo");
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    let generation = harness.runs[0].generation;
    let failure = harness.panel.finish(
        &harness.streams,
        generation,
        Err(AppError::InvalidArgument("regex parse error".into())),
    );
    assert!(failure.is_none());
    harness.frame(Vec::new());
    assert!(harness.has_text("search.failed"));
}

#[test]
fn enter_실행의_실패는_토스트로_알린다() {
    let mut harness = Harness::new(false);
    harness.typed("(foo");
    harness.enter();
    let generation = harness.runs[0].generation;
    let failure = harness.panel.finish(
        &harness.streams,
        generation,
        Err(AppError::InvalidArgument("regex parse error".into())),
    );
    assert!(failure.is_some());
}

#[test]
fn 설정이_꺼져_있으면_입력만으로는_돌지_않고_enter_에서만_돈다() {
    let mut harness = Harness::new(false);
    assert!(harness.has_text("search.pressEnterHint"));
    harness.typed("foo");
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert!(harness.runs.is_empty());
    harness.enter();
    assert_eq!(harness.runs.len(), 1);
}

#[test]
fn 설정이_켜져_있으면_안내_문구가_실시간_검색_문구로_바뀐다() {
    let harness = Harness::new(true);
    assert!(harness.has_text("search.liveSearchHint"));
}

#[test]
fn 조합_중_글자만으로는_실시간_검색이_돌지_않고_조합이_끝나면_돈다() {
    let mut harness = Harness::new(true);
    harness.frame(vec![Event::Ime(egui::ImeEvent::Preedit {
        text: "한".into(),
        active_range_chars: None,
    })]);
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert!(harness.runs.is_empty());
    harness.frame(vec![Event::Ime(egui::ImeEvent::Commit("한".into()))]);
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert_eq!(harness.runs.len(), 1);
    assert_eq!(harness.runs[0].query.text, "한");
}

#[test]
fn 조합_중_enter_와_공백뿐인_질의의_enter_는_무시한다() {
    let mut harness = Harness::new(false);
    harness.typed("   ");
    harness.enter();
    assert!(harness.runs.is_empty());
    harness.frame(vec![Event::Ime(egui::ImeEvent::Preedit {
        text: "한".into(),
        active_range_chars: None,
    })]);
    harness.enter();
    assert!(harness.runs.is_empty());
}

#[test]
fn 질의는_토글_제외_glob_범위를_담고_컨텍스트_줄은_0이다() {
    let mut harness = Harness::new(false);
    harness.request = Some(Request {
        scope_dir: Some("node_modules/pkg".into()),
        ..Default::default()
    });
    harness.frame(Vec::new());
    assert!(harness.texts().contains(&crate::presentation::message(
        &harness.locale,
        "explorer.searchScopeLabel",
        &[("path", "node_modules/pkg")],
    )));
    for key in ["search.regex", "search.respectGitignore"] {
        harness.click(key);
    }
    harness.frame(Vec::new());
    let regex = message(&harness.locale, "search.regex");
    assert_eq!(harness.node(&regex).unwrap().1, Some(true));
    let gitignore = message(&harness.locale, "search.respectGitignore");
    assert_eq!(harness.node(&gitignore).unwrap().1, Some(false));
    let exclude = message(&harness.locale, "search.excludeGlobPlaceholder");
    let (rect, ..) = harness.node(&exclude).unwrap();
    harness.frame(vec![
        Event::PointerMoved(rect.center()),
        Event::PointerButton {
            pos: rect.center(),
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: rect.center(),
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]);
    harness.typed(" *.test.ts ");
    harness.panel.query = "needle".into();
    let run = harness
        .panel
        .submit(&harness.streams, harness.request.as_ref(), false);
    assert_eq!(
        run.query,
        SearchQuery {
            text: "needle".into(),
            case_sensitive: false,
            whole_word: false,
            regex: true,
            include_glob: None,
            exclude_glob: Some("*.test.ts".into()),
            context_lines: 0,
            respect_gitignore: false,
            scope_dir: Some("node_modules/pkg".into()),
        }
    );
    harness.click("search.clearScope");
    assert_eq!(harness.cleared, 1);
}

#[test]
fn 이력_버튼은_이력이_없으면_비활성이고_고르면_질의만_바꾼다() {
    let mut harness = Harness::new(false);
    let label = message(&harness.locale, "search.recentSearches");
    assert!(harness.node(&label).unwrap().2);
    harness.history = vec!["older".into()];
    harness.frame(Vec::new());
    assert!(!harness.node(&label).unwrap().2);
    harness.click("search.recentSearches");
    harness.frame(Vec::new());
    let (rect, ..) = harness.node("older").expect("history menu item");
    let button = |pressed| Event::PointerButton {
        pos: rect.center(),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![Event::PointerMoved(rect.center()), button(true)]);
    harness.frame(vec![button(false)]);
    assert_eq!(harness.panel.query, "older");
    assert!(harness.runs.is_empty());
}

#[test]
fn 결과는_50ms_뒤에_한꺼번에_붙고_완료는_남은_배치와_total을_반영한다() {
    let mut harness = Harness::new(false);
    harness.typed("needle");
    harness.enter();
    let generation = harness.runs[0].generation;
    let session = harness.panel.session.clone();
    harness.streams.push(
        &session,
        generation,
        SearchFileMatches {
            path: "/repo/a.txt".into(),
            matches: vec![found(1)],
        },
    );
    harness.frame(Vec::new());
    assert!(harness.has_text("search.searching"));
    assert!(!harness.texts().contains(&"a.txt".to_owned()));
    harness.wait(RESULT_FLUSH_INTERVAL + FRAME_SECONDS);
    assert!(harness.texts().contains(&"a.txt".to_owned()));
    harness.streams.push(
        &session,
        generation,
        SearchFileMatches {
            path: "/repo/b.txt".into(),
            matches: vec![found(2), found(4)],
        },
    );
    assert!(
        harness
            .panel
            .finish(&harness.streams, generation + 1, Ok(0))
            .is_none()
    );
    assert_eq!(harness.panel.status, Status::Running);
    assert!(
        harness
            .panel
            .finish(&harness.streams, generation, Ok(3))
            .is_none()
    );
    harness.frame(Vec::new());
    assert!(harness.texts().contains(&crate::presentation::message(
        &harness.locale,
        "search.matchCount",
        &[("count", "3"), ("files", "2")],
    )));
    assert!(harness.texts().contains(&"b.txt".to_owned()));
    assert_eq!(harness.panel.running_session(), None);
}

#[test]
fn 새_요청은_시드_질의와_바꾸기_열림을_적용하고_일치_행_클릭은_열기를_낸다() {
    let mut harness = Harness::new(false);
    harness.request = Some(Request {
        seed_text: Some("seed".into()),
        open_replace: true,
        ..Default::default()
    });
    harness.panel.seeded_nonce = None;
    harness.frame(Vec::new());
    assert_eq!(harness.panel.query, "seed");
    let toggle = message(&harness.locale, "search.replaceToggle");
    harness.frame(Vec::new());
    assert_eq!(harness.node(&toggle).unwrap().1, Some(true));
    harness.enter();
    let generation = harness.runs[0].generation;
    let session = harness.panel.session.clone();
    harness.streams.push(
        &session,
        generation,
        SearchFileMatches {
            path: "/repo/a.txt".into(),
            matches: vec![found(7)],
        },
    );
    harness.panel.finish(&harness.streams, generation, Ok(1));
    harness.frame(Vec::new());
    let select = crate::presentation::message(
        &harness.locale,
        "search.selectFile",
        &[("path", "/repo/a.txt")],
    );
    assert_eq!(harness.node(&select).unwrap().1, Some(true));
    let (row, ..) = harness.node("7 a needle").expect("match row");
    let button = |pressed| Event::PointerButton {
        pos: row.center(),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![Event::PointerMoved(row.center()), button(true)]);
    harness.frame(vec![button(false)]);
    assert_eq!(harness.opens.len(), 1);
    assert_eq!(
        (
            harness.opens[0].path.as_str(),
            harness.opens[0].line,
            harness.opens[0].column
        ),
        ("/repo/a.txt", 7, 3)
    );
}

fn panel_query() -> SearchQuery {
    SearchQuery {
        text: "needle".into(),
        case_sensitive: false,
        whole_word: false,
        regex: false,
        include_glob: None,
        exclude_glob: None,
        context_lines: 0,
        respect_gitignore: true,
        scope_dir: None,
    }
}

fn show_results(harness: &mut Harness, files: &[(&str, u32)]) {
    harness.enter();
    let generation = harness.runs.last().unwrap().generation;
    let session = harness.panel.session.clone();
    for (path, count) in files {
        harness.streams.push(
            &session,
            generation,
            SearchFileMatches {
                path: (*path).into(),
                matches: (1..=*count).map(found).collect(),
            },
        );
    }
    let total = files.iter().map(|(_, count)| count).sum();
    harness
        .panel
        .finish(&harness.streams, generation, Ok(total));
    harness.frame(Vec::new());
}

fn skipped(count: usize) -> Vec<taide_model::search::ReplaceSkippedFile> {
    (0..count)
        .map(|index| taide_model::search::ReplaceSkippedFile {
            path: format!("src/file-{index}.bin"),
            reason: ReplaceSkipReason::Binary,
        })
        .collect()
}

fn replace_result(
    skipped: Vec<taide_model::search::ReplaceSkippedFile>,
    skipped_count: u32,
) -> SearchReplaceResult {
    SearchReplaceResult {
        changed_files: 0,
        replaced_matches: 0,
        skipped,
        skipped_count,
    }
}

#[test]
fn 생략된_옵션을_기본값으로_채운다() {
    let normalized: SearchQuery =
        serde_json::from_value(serde_json::json!({"text": "needle"})).unwrap();
    assert_eq!(normalized, panel_query());
}

#[test]
fn 명시된_옵션은_그대로_둔다() {
    let normalized: SearchQuery = serde_json::from_value(
        serde_json::json!({"text": "needle", "caseSensitive": true, "contextLines": 3}),
    )
    .unwrap();
    assert!(normalized.case_sensitive);
    assert_eq!(normalized.context_lines, 3);
}

#[test]
fn 옵션이_생략된_쪽과_기본값으로_채운_쪽을_같다고_본다() {
    let omitted: SearchQuery =
        serde_json::from_value(serde_json::json!({"text": "needle"})).unwrap();
    assert!(is_same_query(&omitted, &panel_query()));
}

#[test]
fn 검색어가_다르면_다르다() {
    let other = SearchQuery {
        text: "other".into(),
        ..panel_query()
    };
    assert!(!is_same_query(&panel_query(), &other));
}

#[test]
fn 토글이_하나라도_다르면_다르다() {
    for changed in [
        SearchQuery {
            case_sensitive: true,
            ..panel_query()
        },
        SearchQuery {
            whole_word: true,
            ..panel_query()
        },
        SearchQuery {
            regex: true,
            ..panel_query()
        },
        SearchQuery {
            respect_gitignore: false,
            ..panel_query()
        },
    ] {
        assert!(!is_same_query(&panel_query(), &changed), "{changed:?}");
    }
}

#[test]
fn 포함_제외_글롭이_다르면_다르다() {
    let include = SearchQuery {
        include_glob: Some("src/**".into()),
        ..panel_query()
    };
    let exclude = SearchQuery {
        exclude_glob: Some("*.test.ts".into()),
        ..panel_query()
    };
    assert!(!is_same_query(&panel_query(), &include));
    assert!(!is_same_query(&panel_query(), &exclude));
}

#[test]
fn 폴더_범위가_다르면_다르다() {
    let scoped = SearchQuery {
        scope_dir: Some("node_modules/pkg".into()),
        ..panel_query()
    };
    assert!(!is_same_query(&panel_query(), &scoped));
}

#[test]
fn 컨텍스트_줄_수는_매치_집합을_바꾸지_않으므로_비교하지_않는다() {
    let context = SearchQuery {
        context_lines: 3,
        ..panel_query()
    };
    assert!(is_same_query(&panel_query(), &context));
}

#[test]
fn 스킵이_없으면_리포트를_만들지_않는다() {
    assert!(skip_report(&replace_result(Vec::new(), 0), &locale()).is_none());
}

#[test]
fn 스킵된_파일을_사유와_함께_나열한다() {
    let locale = locale();
    let report = skip_report(
        &replace_result(
            vec![
                taide_model::search::ReplaceSkippedFile {
                    path: "a.bin".into(),
                    reason: ReplaceSkipReason::Binary,
                },
                taide_model::search::ReplaceSkippedFile {
                    path: "b.txt".into(),
                    reason: ReplaceSkipReason::NotUtf8,
                },
            ],
            2,
        ),
        &locale,
    )
    .unwrap();
    assert_eq!(
        report.title,
        crate::presentation::message(&locale, "search.replaceSkipped", &[("count", "2")])
    );
    assert_eq!(
        report.description,
        format!(
            "a.bin — {}\nb.txt — {}",
            message(&locale, "search.replaceSkipReason.binary"),
            message(&locale, "search.replaceSkipReason.notUtf8"),
        )
    );
}

#[test]
fn 나열_상한을_넘으면_남은_개수를_따로_알린다() {
    let locale = locale();
    let listed = SKIP_LIST_LIMIT + 3;
    let report = skip_report(&replace_result(skipped(listed), listed as u32), &locale).unwrap();
    let lines = report.description.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), SKIP_LIST_LIMIT + 1);
    assert_eq!(
        lines[SKIP_LIST_LIMIT],
        crate::presentation::message(&locale, "search.replaceSkippedMore", &[("count", "3")])
    );
}

#[test]
fn 백엔드가_목록을_잘라_보낸_경우에도_전체_개수를_기준으로_남은_수를_센다() {
    let locale = locale();
    let total = 120;
    let report = skip_report(&replace_result(skipped(SKIP_LIST_LIMIT), total), &locale).unwrap();
    let total_text = total.to_string();
    let remaining = (total as usize - SKIP_LIST_LIMIT).to_string();
    assert_eq!(
        report.title,
        crate::presentation::message(&locale, "search.replaceSkipped", &[("count", &total_text)])
    );
    assert_eq!(
        report.description.lines().last(),
        Some(
            crate::presentation::message(
                &locale,
                "search.replaceSkippedMore",
                &[("count", &remaining)]
            )
            .as_str()
        )
    );
}

#[test]
fn 모든_스킵_사유가_로케일_키를_가진다() {
    let locale = locale();
    for reason in [
        ReplaceSkipReason::TooLarge,
        ReplaceSkipReason::Binary,
        ReplaceSkipReason::NotUtf8,
        ReplaceSkipReason::Unreadable,
        ReplaceSkipReason::WriteFailed,
    ] {
        let key = skip_reason_key(reason);
        assert!(key.starts_with("search.replaceSkipReason."), "{key}");
        assert_ne!(message(&locale, key), key, "{key}");
    }
}

#[test]
fn 범위_안의_컨텍스트_줄_수는_그대로_반환한다() {
    assert_eq!(clamp_context_lines(3.0), 3);
}

#[test]
fn 컨텍스트_줄_수가_최댓값을_넘으면_최댓값으로_자른다() {
    assert_eq!(
        clamp_context_lines(f64::from(MAX_CONTEXT_LINES + 5)),
        MAX_CONTEXT_LINES
    );
}

#[test]
fn 음수_컨텍스트_줄_수는_최솟값으로_자른다() {
    assert_eq!(clamp_context_lines(-1.0), MIN_CONTEXT_LINES);
}

#[test]
fn 소수_컨텍스트_줄_수는_정수로_잘라낸다() {
    assert_eq!(clamp_context_lines(2.9), 2);
}

#[test]
fn nan_컨텍스트_줄_수는_최솟값으로_취급한다() {
    assert_eq!(clamp_context_lines(f64::NAN), MIN_CONTEXT_LINES);
}

#[test]
fn 부분_요청은_나머지_필드가_기본값으로_채워진다() {
    assert_eq!(
        Request {
            include_glob: Some("src/**".into()),
            ..Default::default()
        },
        Request {
            include_glob: Some("src/**".into()),
            scope_dir: None,
            seed_text: None,
            open_replace: false,
        }
    );
}

#[test]
fn 폴더_범위_요청은_scope_dir만_싣고_include_glob은_비운다() {
    let request = Request {
        scope_dir: Some("node_modules/pkg".into()),
        ..Default::default()
    };
    assert_eq!(request.include_glob, None);
    assert_eq!(request.scope_dir.as_deref(), Some("node_modules/pkg"));
    assert_eq!(request.seed_text, None);
    assert!(!request.open_replace);
}

#[test]
fn 모두_바꾸기는_표시_결과와_같은_입력에서만_켜지고_확인은_체크한_파일과_실행한_질의만_넘긴다() {
    let mut harness = Harness::new(false);
    harness.typed("needle");
    show_results(&mut harness, &[("/repo/a.txt", 1), ("/repo/b.txt", 2)]);
    let replace_all = message(&harness.locale, "search.replaceAll");
    assert!(harness.node(&replace_all).is_none());
    harness.click("search.replaceToggle");
    harness.frame(Vec::new());
    assert_eq!(harness.node(&replace_all).map(|node| node.2), Some(false));

    harness.panel.query.push('x');
    harness.frame(Vec::new());
    assert_eq!(harness.node(&replace_all).map(|node| node.2), Some(true));
    assert!(harness.has_text("search.replaceStaleHint"));
    harness.panel.query.pop();
    harness.panel.replace_text = "pin".into();
    harness.frame(Vec::new());
    assert!(!harness.has_text("search.replaceStaleHint"));
    assert_eq!(harness.node(&replace_all).map(|node| node.2), Some(false));

    let select_b = crate::presentation::message(
        &harness.locale,
        "search.selectFile",
        &[("path", "/repo/b.txt")],
    );
    harness.click_label(&select_b);
    harness.frame(Vec::new());
    assert_eq!(harness.node(&select_b).unwrap().1, Some(false));

    let description = crate::presentation::message(
        &harness.locale,
        "search.replaceConfirmDescription",
        &[("files", "1"), ("matches", "1")],
    );
    harness.click("search.replaceAll");
    harness.frame(Vec::new());
    assert!(harness.texts().contains(&description));
    assert!(harness.has_text("search.replaceConfirmTitle"));
    harness.frame(vec![Event::Key {
        key: Key::Escape,
        physical_key: Some(Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.frame(Vec::new());
    assert!(!harness.texts().contains(&description));
    harness.click("search.replaceAll");
    harness.frame(Vec::new());
    harness.click("common.cancel");
    harness.frame(Vec::new());
    assert!(!harness.texts().contains(&description));
    assert!(harness.replaces.is_empty());

    harness.click("search.replaceAll");
    harness.frame(Vec::new());
    harness.click("common.confirm");
    assert_eq!(harness.replaces.len(), 1);
    let replace = harness.replaces.remove(0);
    assert_eq!(replace.paths, Some(vec!["/repo/a.txt".to_owned()]));
    assert_eq!(replace.replacement, "pin");
    assert_eq!(replace.query, harness.runs[0].query);
    harness.frame(Vec::new());
    assert!(!harness.texts().contains(&description));
    assert_eq!(harness.node(&replace_all).map(|node| node.2), Some(true));

    let rerun = harness
        .panel
        .replace_finished(&harness.streams, Some(replace.query.clone()))
        .unwrap();
    assert_eq!(rerun.record, None);
    assert_eq!(rerun.query, replace.query);
    assert_eq!(harness.panel.status(), Status::Running);
    assert!(
        harness
            .panel
            .replace_finished(&harness.streams, None)
            .is_none()
    );
}

#[test]
fn 검색_편집기_열기_버튼은_빈_검색어도_현재_입력과_범위로_만든_질의를_낸다() {
    let mut harness = Harness::new(false);
    harness.request = Some(Request {
        scope_dir: Some("pkg".into()),
        ..Default::default()
    });
    harness.frame(Vec::new());
    harness.click("search.regex");
    harness.click("searchEditor.title");
    assert_eq!(
        harness.editors,
        vec![SearchQuery {
            text: String::new(),
            regex: true,
            scope_dir: Some("pkg".into()),
            ..panel_query()
        }]
    );
}

#[test]
fn 검색_편집기_본문은_컨텍스트_줄을_받고_바꾸기_범위_칩_입력_검색_없이_enter로_범위를_유지해_실행한다()
 {
    let mut harness = Harness::new(true);
    let tab_query = SearchQuery {
        include_glob: Some("src/**".into()),
        scope_dir: Some("pkg".into()),
        ..panel_query()
    };
    harness.panel = Panel::editor(harness.panel.session.clone(), &tab_query);
    harness.request = Some(Request {
        include_glob: tab_query.include_glob.clone(),
        scope_dir: tab_query.scope_dir.clone(),
        ..Default::default()
    });
    harness.frame(Vec::new());
    for key in [
        "search.replaceToggle",
        "searchEditor.title",
        "search.clearScope",
    ] {
        assert!(
            harness.node(&message(&harness.locale, key)).is_none(),
            "{key}"
        );
    }
    assert!(harness.has_text("search.pressEnterHint"));
    harness.wait(AFTER_DEBOUNCE_SECONDS);
    assert!(harness.runs.is_empty());

    harness.click("searchEditor.contextLinesLabel");
    harness.typed("2");
    assert_eq!(harness.panel.context_lines(), 2);
    harness.click("search.placeholder");
    harness.enter();
    assert_eq!(harness.runs.len(), 1);
    assert_eq!(
        harness.runs[0].query,
        SearchQuery {
            context_lines: 2,
            ..tab_query
        }
    );
    assert_eq!(harness.runs[0].record.as_deref(), Some("needle"));
    let generation = harness.runs[0].generation;
    harness.panel.finish(&harness.streams, generation, Ok(0));
    harness.frame(Vec::new());
    let empty = message(&harness.locale, "searchEditor.noResults");
    assert_eq!(
        harness
            .texts()
            .iter()
            .filter(|text| **text == empty)
            .count(),
        2
    );
}
