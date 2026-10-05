use taide_model::error::AppError;
use taide_native_app::terminal_file_links::{CACHE_ROWS, Cache, Link};
use taide_native_app::terminal_links::read_row;
use taide_native_terminal::{Column, Line, Point, Size, TerminalCore};

const COLUMNS: u16 = 120;
const ROWS: u16 = 6;
const HISTORY: usize = 8;
const FILE_START: usize = 5;

fn core(text: &str) -> TerminalCore {
    let mut core = TerminalCore::new(
        Size {
            columns: COLUMNS,
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
fn file_cache는_null_fifo_live_cwd와_실패_retry_단일pending을_유지한다() {
    let terminal = core("file.rs:2:3");
    let row = read_row(&terminal, Line(0)).unwrap();
    let point = Point::new(Line(0), Column(0));
    let mut cache = Cache::default();
    assert!(cache.request("", &row).unwrap().is_none());
    let first = cache.request("/synthetic/a", &row).unwrap().unwrap();
    assert_eq!(first.candidates(), ["file.rs"]);
    assert!(cache.request("/synthetic/a", &row).unwrap().is_none());
    assert!(cache.request("/synthetic/b", &row).unwrap().is_none());
    assert!(cache.complete(&first, Ok(vec![None])));
    assert!(
        cache
            .at(&terminal, "/synthetic/a", point)
            .unwrap()
            .is_none()
    );
    assert!(cache.request("/synthetic/a", &row).unwrap().is_none());
    let changed = cache.request("/synthetic/b", &row).unwrap().unwrap();
    assert!(cache.complete(
        &changed,
        Err(AppError::Internal("synthetic failure".into()))
    ));
    let retry = cache.request("/synthetic/b", &row).unwrap().unwrap();
    assert!(!cache.complete(&changed, Ok(vec![Some("/synthetic/old.rs".into())])));
    cache.cancel(&changed);
    assert!(cache.request("/synthetic/b", &row).unwrap().is_none());
    cache.cancel(&retry);
    let retry = cache.request("/synthetic/b", &row).unwrap().unwrap();
    assert!(cache.complete(&retry, Ok(vec![Some("/synthetic/b/file.rs".into())])));
    assert!(
        matches!(cache.at(&terminal, "/synthetic/b", point).unwrap(), Some(Link::File(link)) if link.path == "/synthetic/b/file.rs")
    );
    for index in 0..CACHE_ROWS - 2 {
        let row = read_row(&core(&format!("file-{index}.rs")), Line(0)).unwrap();
        let request = cache.request("/synthetic/a", &row).unwrap().unwrap();
        cache.complete(&request, Ok(vec![None]));
    }
    assert!(cache.request("/synthetic/a", &row).unwrap().is_none());
    let last = read_row(&core("last.rs"), Line(0)).unwrap();
    let request = cache.request("/synthetic/a", &last).unwrap().unwrap();
    cache.complete(&request, Ok(vec![None]));
    assert!(cache.request("/synthetic/a", &row).unwrap().is_some());
}

#[test]
fn file_cache는_현재_grid_range를_재계산하고_osc8_url_우선순위를_유지한다() {
    let terminal = core("汉𐐀e\u{301} file.rs:2:3 ghost.rs");
    let row = read_row(&terminal, Line(0)).unwrap();
    let mut cache = Cache::default();
    let request = cache.request("/synthetic", &row).unwrap().unwrap();
    assert!(cache.complete(&request, Ok(vec![Some("/synthetic/file.rs".into()), None])));
    let point = Point::new(Line(0), Column(FILE_START));
    let Some(Link::File(link)) = cache.at(&terminal, "/synthetic", point).unwrap() else {
        panic!("resolved file must be linked")
    };
    assert_eq!(link.matched.line, Some(2.0));
    assert_eq!(link.matched.column, Some(3.0));
    assert_eq!(link.matched.range.start, point);
    let moved = core("\r\n汉𐐀e\u{301} file.rs:2:3 ghost.rs");
    let Some(Link::File(link)) = cache
        .at(
            &moved,
            "/synthetic",
            Point::new(Line(1), Column(FILE_START)),
        )
        .unwrap()
    else {
        panic!("cache must reuse text without old row coordinates")
    };
    assert_eq!(link.matched.range.start.line, Line(1));
    let osc =
        core("\x1b]8;;https://example.com/\x1b\\汉𐐀e\u{301} file.rs:2:3 ghost.rs\x1b]8;;\x1b\\");
    assert!(matches!(
        cache.at(&osc, "/synthetic", point).unwrap(),
        Some(Link::External(_))
    ));
    assert!(
        cache
            .at(
                &terminal,
                "/synthetic",
                Point::new(Line(0), Column(FILE_START - 1))
            )
            .unwrap()
            .is_none()
    );
}
