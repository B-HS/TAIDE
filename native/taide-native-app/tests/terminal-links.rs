use eframe::egui::Modifiers;
use taide_native_app::terminal_links::{external_at, file_links, read_row, should_activate};
use taide_native_terminal::{Column, Line, Point, Size, TerminalCore};

const COLUMNS: u16 = 120;
const ROWS: u16 = 6;
const HISTORY: usize = 8;
const PREFIX_COLUMNS: usize = 5;
const WRAP_COLUMNS: u16 = 12;

fn create_core(columns: u16, text: &str) -> TerminalCore {
    let mut core = TerminalCore::new(
        Size {
            columns,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    core.advance(text.as_bytes()).unwrap();
    core
}

#[test]
fn 파일_좌표_6종은_원본_ascii문법과_wide_astral_nfd_cell위치를_보존한다() {
    for (suffix, line, column) in [
        (":12:34", Some(12.0), Some(34.0)),
        (":12-34", Some(12.0), None),
        ("(12,34)", Some(12.0), Some(34.0)),
        ("(12)", Some(12.0), None),
        ("#L12-L34", Some(12.0), None),
        ("\", line 12", Some(12.0), None),
    ] {
        let path = "../src/view.tsx";
        let core = create_core(COLUMNS, &format!("한𐐀e\u{301} {path}{suffix} trailing"));
        let row = read_row(&core, Line(0)).unwrap();
        assert_eq!(row.text, format!("한𐐀e\u{301} {path}{suffix} trailing"));
        let links = file_links(&row).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].path, path);
        assert_eq!(links[0].text, format!("{path}{suffix}"));
        assert_eq!(links[0].line, line);
        assert_eq!(links[0].column, column);
        assert_eq!(
            links[0].range.start,
            Point::new(Line(0), Column(PREFIX_COLUMNS))
        );
        assert_eq!(
            links[0].range.end,
            Point::new(
                Line(0),
                Column(PREFIX_COLUMNS + path.len() + suffix.len() - 1)
            )
        );
    }
    let core = create_core(
        COLUMNS,
        "汉.ts 文/src.ts ok.rs:1:2 next.md#L3-L9 ./last.txt(4)\u{feff}",
    );
    let links = file_links(&read_row(&core, Line(0)).unwrap()).unwrap();
    assert_eq!(
        links
            .iter()
            .map(|link| link.path.as_str())
            .collect::<Vec<_>>(),
        ["ok.rs", "next.md", "./last.txt"]
    );
    assert_eq!(links[0].column, Some(2.0));
    assert_eq!(links[1].line, Some(3.0));
    let core = create_core(WRAP_COLUMNS, "../very-long-file.ts:1:2");
    for line in [Line(0), Line(1)] {
        assert!(
            file_links(&read_row(&core, line).unwrap())
                .unwrap()
                .iter()
                .all(|link| link.path != "../very-long-file.ts")
        );
    }
}

#[test]
fn url은_wrap과_wide_prefix를_매핑하고_원본_종결구두점과_유효성규칙을_유지한다() {
    let core = create_core(WRAP_COLUMNS, "한𐐀e\u{301} https://example.com/abc?x=1, ");
    for (point, expected) in [
        (
            Point::new(Line(0), Column(PREFIX_COLUMNS)),
            "https://example.com/abc?x=1",
        ),
        (
            Point::new(Line(1), Column(0)),
            "https://example.com/abc?x=1",
        ),
        (
            Point::new(Line(2), Column(4)),
            "https://example.com/abc?x=1",
        ),
    ] {
        let link = external_at(&core, point).unwrap().unwrap();
        assert_eq!(link.uri, expected);
        assert_eq!(
            link.range.start,
            Point::new(Line(0), Column(PREFIX_COLUMNS))
        );
        assert_eq!(link.range.end, Point::new(Line(2), Column(7)));
    }
    assert!(
        external_at(&core, Point::new(Line(0), Column(0)))
            .unwrap()
            .is_none()
    );
    let core = create_core(
        COLUMNS,
        "https://[::1]:8080/a?q=1). HTTPS://example.com/ok HtTp://example.com/mixed http://例.example/",
    );
    let first = external_at(&core, Point::new(Line(0), Column(0)))
        .unwrap()
        .unwrap();
    assert_eq!(first.uri, "https://[::1]:8080/a?q=1");
    let row = read_row(&core, Line(0)).unwrap();
    let upper = row.text.find("HTTPS:").unwrap();
    assert_eq!(
        external_at(&core, Point::new(Line(0), Column(upper)))
            .unwrap()
            .unwrap()
            .uri,
        "HTTPS://example.com/ok"
    );
    for refused in ["HtTp:", "http://例"] {
        let offset = row.text[..row.text.find(refused).unwrap()].chars().count();
        assert!(
            external_at(&core, Point::new(Line(0), Column(offset)))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn osc8은_같은_grid의_id와_uri를_우선하고_잘못된_scheme_출력수정_reset을_따른다() {
    let mut core = create_core(
        COLUMNS,
        "\u{1b}]8;id=one;https://example.com/target\u{1b}\\한e\u{301} link\u{1b}]8;;\u{1b}\\ https://example.com/plain",
    );
    for column in [0, 1, 2, 3, 4, 5, 6, 7] {
        let link = external_at(&core, Point::new(Line(0), Column(column)))
            .unwrap()
            .unwrap();
        assert_eq!(link.uri, "https://example.com/target");
        assert_eq!(link.range.start.column, Column(0));
        assert_eq!(link.range.end.column, Column(7));
    }
    core.advance(b"\r\x1b[2Kno link").unwrap();
    assert!(
        external_at(&core, Point::new(Line(0), Column(0)))
            .unwrap()
            .is_none()
    );
    core.advance(b"\x1bc\x1b]8;;file:///synthetic.txt\x1b\\bad\x1b]8;;\x1b\\")
        .unwrap();
    assert!(
        external_at(&core, Point::new(Line(0), Column(0)))
            .unwrap()
            .is_none()
    );
    assert!(
        external_at(&core, Point::new(Line(-99), Column(0)))
            .unwrap()
            .is_none()
    );
}

#[test]
fn 링크_modifier는_alt와_플랫폼_mod만_사용하고_한행_승인후보는_16개로_제한한다() {
    for is_mac in [false, true] {
        assert!(should_activate(Modifiers::ALT, is_mac));
        assert!(should_activate(
            if is_mac {
                Modifiers::MAC_CMD
            } else {
                Modifiers::CTRL
            },
            is_mac
        ));
        assert!(!should_activate(
            if is_mac {
                Modifiers::CTRL
            } else {
                Modifiers::MAC_CMD
            },
            is_mac
        ));
        assert!(!should_activate(Modifiers::SHIFT, is_mac));
        assert!(!should_activate(Modifiers::NONE, is_mac));
    }
    let text = (0..17)
        .map(|index| format!(" {index}.rs"))
        .collect::<String>();
    let core = create_core(COLUMNS, &text);
    assert_eq!(
        file_links(&read_row(&core, Line(0)).unwrap())
            .unwrap()
            .len(),
        taide_terminal::service::MAX_LINK_CANDIDATES_PER_ROW
    );
}

#[test]
fn 같은_물리행의_유효_osc8과_겹친_낮은_우선순위_url은_전체_범위를_제외한다() {
    for (uri, expected) in [
        ("https://example.org/osc", None),
        ("file:///synthetic.txt", Some("https://example.com/plain")),
    ] {
        let core = create_core(
            COLUMNS,
            &format!(
                "https://exa\u{1b}]8;id=overlap;{uri}\u{1b}\\mple\u{1b}]8;;\u{1b}\\.com/plain"
            ),
        );
        assert_eq!(
            external_at(&core, Point::new(Line(0), Column(0)))
                .unwrap()
                .map(|link| link.uri),
            expected.map(String::from)
        );
    }
}
