use super::*;
use egui::vec2;
use taide_model::layout::TabKind;

const TAB_SIZE: [f32; 2] = [100.0, 36.0];
const TAB_GAP: f32 = 2.0;
const BODY: Rect = Rect {
    min: Pos2 { x: 0.0, y: 40.0 },
    max: Pos2 { x: 400.0, y: 440.0 },
};
const PROBE: f32 = 10.0;
const BAR_RIGHT: f32 = 600.0;

fn tab(title: &str, pinned: bool) -> Tab {
    Tab {
        id: TabId::new(),
        kind: TabKind::Settings,
        title: title.into(),
        pinned,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

fn area() -> TabArea {
    TabArea {
        project: ProjectId::new(),
        slot: Some(ShellSlotId::new()),
    }
}

fn tab_rects(count: usize) -> Vec<Rect> {
    (0..count)
        .map(|index| {
            Rect::from_min_size(
                pos2(index as f32 * (TAB_SIZE[0] + TAB_GAP), 0.0),
                vec2(TAB_SIZE[0], TAB_SIZE[1]),
            )
        })
        .collect()
}

struct Fixture {
    area: TabArea,
    pane: PaneId,
    tabs: Vec<Tab>,
    frame: Frame,
}

fn fixture() -> Fixture {
    let area = area();
    let pane = PaneId::new();
    let tabs = vec![
        tab("pinned", true),
        tab("a", false),
        tab("b", false),
        tab("c", false),
    ];
    let rects = tab_rects(tabs.len());
    let frame = Frame {
        tabs: tabs
            .iter()
            .zip(&rects)
            .map(|(tab, rect)| PlacedTab {
                area: area.clone(),
                pane: pane.clone(),
                tab: tab.id.clone(),
                pinned: tab.pinned,
                rect: *rect,
                visible: *rect,
            })
            .collect(),
        containers: vec![(
            area.clone(),
            pane.clone(),
            Rect::from_min_max(
                pos2(rects[rects.len() - 1].right(), 0.0),
                pos2(BAR_RIGHT, TAB_SIZE[1]),
            ),
        )],
        panes: vec![PlacedPane {
            area: area.clone(),
            pane: pane.clone(),
            body: BODY,
            tabs: tabs
                .iter()
                .map(|tab| (tab.id.clone(), tab.pinned))
                .collect(),
        }],
    };
    Fixture {
        area,
        pane,
        tabs,
        frame,
    }
}

fn press(fixture: &Fixture, dragged: &Tab, over: Option<TabDropTarget>) -> Press {
    Press {
        area: fixture.area.clone(),
        pane: fixture.pane.clone(),
        tab: dragged.clone(),
        origin: Pos2::ZERO,
        rect: Rect::from_min_size(Pos2::ZERO, vec2(TAB_SIZE[0], TAB_SIZE[1])),
        content: BODY,
        phase: Phase::Active,
        over,
        layout: fixture.frame.tabs.clone(),
        pointer: None,
        intent: ScrollIntent::default(),
    }
}

fn moved_to(request: Option<ShellMutation>) -> Option<(TabId, PaneId, u32)> {
    match request? {
        ShellMutation::MoveTab { tab, pane, index } => Some((tab, pane, index)),
        _ => None,
    }
}

#[test]
fn 가로_정렬_이동량은_dnd_kit_가로목록_전략처럼_원본자리와_간격을_밀어낸다() {
    let rects = tab_rects(3);
    let width = TAB_SIZE[0];
    assert_eq!(
        horizontal_sorting_offset(&rects, 0, 2, 0),
        rects[2].right() - rects[0].right()
    );
    assert_eq!(horizontal_sorting_offset(&rects, 0, 2, 1), -width - TAB_GAP);
    assert_eq!(horizontal_sorting_offset(&rects, 0, 2, 2), -width - TAB_GAP);
    assert_eq!(
        horizontal_sorting_offset(&rects, 2, 0, 2),
        rects[0].left() - rects[2].left()
    );
    assert_eq!(horizontal_sorting_offset(&rects, 2, 0, 0), width + TAB_GAP);
    assert_eq!(horizontal_sorting_offset(&rects, 2, 1, 0), 0.0);
}

#[test]
fn 드롭_결과는_고정탭_경계와_가운데_빈칸_끝이동과_가장자리_분할을_원본처럼_고른다() {
    let fixture = fixture();
    let [pinned, a, b, c] = [0, 1, 2, 3].map(|index| fixture.tabs[index].clone());
    let pane = fixture.pane.clone();
    let on_tab = |target: &Tab| TabDropTarget::Tab {
        pane: pane.clone(),
        tab: target.id.clone(),
    };
    let drop = |dragged: &Tab, over: TabDropTarget| {
        resolve_drop(
            &press(&fixture, dragged, None),
            Some(&over),
            &fixture.frame.panes,
        )
    };

    assert_eq!(
        moved_to(drop(&b, on_tab(&pinned))),
        Some((b.id.clone(), pane.clone(), 1)),
        "an unpinned tab stops right after the pinned run"
    );
    assert_eq!(
        moved_to(drop(&a, on_tab(&c))),
        Some((a.id.clone(), pane.clone(), 3))
    );
    assert_eq!(
        moved_to(drop(&pinned, on_tab(&c))),
        Some((pinned.id.clone(), pane.clone(), 1)),
        "a pinned tab never leaves the pinned run"
    );
    for over in [
        TabDropTarget::Container(pane.clone()),
        TabDropTarget::Split {
            pane: pane.clone(),
            edge: DropEdge::Center,
        },
    ] {
        assert_eq!(
            moved_to(drop(&b, over)),
            Some((b.id.clone(), pane.clone(), 4)),
            "the filler and the centre append"
        );
    }
    assert!(matches!(
        drop(&b, TabDropTarget::Split { pane: pane.clone(), edge: DropEdge::Right }),
        Some(ShellMutation::SplitTab { pane: target, edge: DropEdge::Right, tab })
            if target == pane && tab == b.id
    ));
    assert!(
        resolve_drop(&press(&fixture, &b, None), None, &fixture.frame.panes).is_none(),
        "a release over nothing asks for nothing"
    );
    assert!(
        drop(&b, TabDropTarget::Container(PaneId::new())).is_none(),
        "a pane outside the frame is not a destination"
    );
}

#[test]
fn 충돌_판정은_같은_영역의_탭_빈칸_드롭존만_고르고_가려진_탭은_건너뛴다() {
    let mut fixture = fixture();
    let pane = fixture.pane.clone();
    let other = TabArea {
        slot: Some(ShellSlotId::new()),
        ..fixture.area.clone()
    };
    fixture.frame.panes.push(PlacedPane {
        area: other.clone(),
        pane: PaneId::new(),
        body: BODY,
        tabs: Vec::new(),
    });
    let area = fixture.area.clone();
    let target = |frame: &Frame, pointer: Pos2| resolve_target(frame, &area, pointer);
    let tab_center = fixture.frame.tabs[2].rect.center();
    let frame = &fixture.frame;

    assert_eq!(
        target(frame, tab_center),
        Some(TabDropTarget::Tab {
            pane: pane.clone(),
            tab: fixture.tabs[2].id.clone(),
        })
    );
    assert_eq!(
        target(frame, frame.containers[0].2.center()),
        Some(TabDropTarget::Container(pane.clone()))
    );
    assert_eq!(
        target(frame, pos2(BODY.left() + PROBE, BODY.center().y)),
        Some(TabDropTarget::Split {
            pane: pane.clone(),
            edge: DropEdge::Left,
        })
    );
    assert_eq!(
        target(frame, BODY.center()),
        Some(TabDropTarget::Split {
            pane: pane.clone(),
            edge: DropEdge::Center,
        })
    );
    assert_eq!(
        target(frame, pos2(BODY.left() + PROBE, BODY.top() + PROBE * 2.0)),
        Some(TabDropTarget::Split {
            pane: pane.clone(),
            edge: DropEdge::Left,
        }),
        "a corner belongs to the strip whose corners are nearer on average"
    );
    assert_eq!(
        resolve_target(frame, &other, tab_center),
        None,
        "another area's tabs are outside this drag"
    );
    fixture.frame.tabs[2].visible = Rect::NOTHING;
    assert_eq!(
        target(&fixture.frame, tab_center),
        None,
        "a tab scrolled out of the bar takes no drop"
    );
}

#[test]
fn 눌린_탭의_보기는_활성화_뒤에만_흐려지고_같은_고정단_안에서만_밀린다() {
    let fixture = fixture();
    let [pinned, a, b, c] = [0, 1, 2, 3].map(|index| fixture.tabs[index].clone());
    let over = |target: &Tab| {
        Some(TabDropTarget::Tab {
            pane: fixture.pane.clone(),
            tab: target.id.clone(),
        })
    };
    let rects: Vec<Rect> = fixture
        .frame
        .tabs
        .iter()
        .map(|placed| placed.rect)
        .collect();

    let mut held = press(&fixture, &a, over(&c));
    held.phase = Phase::Pending;
    assert_eq!(held.view(&a.id), TabItemView::default());
    assert_eq!(held.view(&b.id), TabItemView::default());

    held.phase = Phase::Active;
    let source = held.view(&a.id);
    assert!(source.is_dragging && source.suppresses_click);
    assert_eq!(source.offset, rects[3].right() - rects[1].right());
    assert_eq!(held.view(&b.id).offset, -TAB_SIZE[0] - TAB_GAP);
    assert_eq!(held.view(&c.id).offset, -TAB_SIZE[0] - TAB_GAP);
    assert_eq!(held.view(&pinned.id).offset, 0.0);
    let passed = held.view(&b.id);
    assert!(!passed.is_dragging && !passed.suppresses_click);

    let across = press(&fixture, &a, over(&pinned));
    assert!(
        [&pinned, &a, &b, &c]
            .iter()
            .all(|tab| across.view(&tab.id).offset == 0.0),
        "the pinned and unpinned runs are separate sortable lists"
    );

    held.phase = Phase::Cancelled;
    let cancelled = held.view(&a.id);
    assert!(!cancelled.is_dragging && cancelled.suppresses_click);
    assert_eq!(cancelled.offset, 0.0);
}

#[test]
fn 자동_스크롤은_가장자리_오분의일_띠에서_움직인_방향으로만_가까울수록_빨라진다() {
    let viewport = Rect::from_min_size(Pos2::ZERO, vec2(BAR_RIGHT, TAB_SIZE[1]));
    let band = BAR_RIGHT * AUTO_SCROLL_THRESHOLD_FRACTION;
    let both = ScrollIntent {
        backward: true,
        forward: true,
    };
    let speed = |x: f32, intent: ScrollIntent| auto_scroll_speed(viewport, x, intent);

    assert_eq!(speed(BAR_RIGHT / 2.0, both), 0.0);
    assert_eq!(speed(BAR_RIGHT - band, both), 0.0);
    assert_eq!(
        speed(BAR_RIGHT - band / 2.0, both),
        AUTO_SCROLL_ACCELERATION / 2.0
    );
    assert_eq!(
        speed(BAR_RIGHT + band, both),
        AUTO_SCROLL_ACCELERATION * 2.0,
        "past the edge it keeps speeding up"
    );
    assert_eq!(speed(band / 2.0, both), -AUTO_SCROLL_ACCELERATION / 2.0);
    assert_eq!(
        speed(
            BAR_RIGHT - band / 2.0,
            ScrollIntent {
                backward: true,
                forward: false,
            }
        ),
        0.0,
        "a pointer that never moved forward does not scroll forward"
    );
}

const SCREEN: Rect = Rect {
    min: Pos2 { x: 0.0, y: 0.0 },
    max: Pos2 { x: 800.0, y: 600.0 },
};
const OUTSIDE: Pos2 = Pos2 { x: 500.0, y: 300.0 };

fn release(fixture: &Fixture, press: &Press, at: Pos2, escape: bool) -> Option<ShellMutation> {
    let context = Context::default();
    let mut events = vec![
        egui::Event::PointerMoved(at),
        egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ];
    if escape {
        events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    }
    let mut request = None;
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(SCREEN),
            events,
            ..Default::default()
        },
        |ui| {
            with_memory(ui.ctx(), |memory| {
                memory.press = Some(press.clone());
                memory.frame = fixture.frame.clone();
            });
            request = end_frame(ui.ctx()).request;
        },
    );
    output.textures_delta.clear();
    request
}

#[test]
fn 해제_프레임은_escape를_먼저_판정하고_대상_밖에서_놓으면_직전_대상으로_옮기지_않는다() {
    let fixture = fixture();
    let [_, a, _, c] = [0, 1, 2, 3].map(|index| fixture.tabs[index].clone());
    let over_c = TabDropTarget::Tab {
        pane: fixture.pane.clone(),
        tab: c.id.clone(),
    };
    let held = Press {
        content: SCREEN,
        ..press(&fixture, &a, Some(over_c))
    };
    let on_c = fixture.frame.tabs[3].rect.center();
    assert_eq!(
        moved_to(release(&fixture, &held, on_c, false)),
        Some((a.id.clone(), fixture.pane.clone(), 3))
    );
    assert!(
        release(&fixture, &held, OUTSIDE, false).is_none(),
        "a release over nothing drops nowhere even though c was the last target"
    );
    assert!(
        release(&fixture, &held, on_c, true).is_none(),
        "Escape in the release frame cancels before the drop"
    );
}
