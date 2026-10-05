#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        grid::{Dimensions, Grid, Row},
        index::{Column, Line},
        term::{Config, Term, cell::Cell},
        vte::ansi::{Processor, StdSyncHandler},
    };

    const COLUMNS: usize = 12;
    const ROWS: usize = 4;
    const HISTORY: usize = 8;
    const UPSTREAM_ROW_WORDS: usize = 4;

    #[test]
    fn fork_feature_조합의_parser_row_clone와_region_swap을_검사한다() {
        let dimensions = Grid::<Cell>::new(ROWS, COLUMNS, HISTORY);
        let mut term = Term::new(Config::default(), &dimensions, VoidListener);
        let mut parser = Processor::<StdSyncHandler>::default();
        parser.advance(&mut term, b"first\r\nsecond\r\nthird\r\nfourth");
        assert_eq!(term.grid()[Line(0)][Column(0)].c, 'f');
        assert_eq!(term.grid()[Line(1)][Column(0)].c, 's');
        let cloned = term.grid().clone();
        assert_eq!(cloned[Line(1)], term.grid()[Line(1)]);
        term.grid_mut()
            .scroll_up::<alacritty_terminal::vte::ansi::Color>(&(Line(1)..Line(ROWS as i32)), 1);
        assert_eq!(term.grid()[Line(0)][Column(0)].c, 'f');
        assert_eq!(term.grid()[Line(1)][Column(0)].c, 't');
        assert_eq!(term.grid()[Line(2)][Column(0)].c, 'f');
        assert_eq!(term.grid()[Line(3)][Column(0)].c, ' ');
        assert_eq!(term.grid().history_size(), 0);
        parser.advance(&mut term, b"\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07");
        #[cfg(feature = "native-retained")]
        {
            assert_eq!(term.native_command_blocks().len(), 1);
            let marker = term.grid_mut()[Line(0)].native_marker();
            let cloned = term.grid()[Line(0)].clone();
            assert!(cloned.native_marker_ref().is_none());
            assert_eq!(marker.load(std::sync::atomic::Ordering::Relaxed), 1);
        }
        #[cfg(not(feature = "native-retained"))]
        assert_eq!(
            size_of::<Row<Cell>>(),
            UPSTREAM_ROW_WORDS * size_of::<usize>()
        );
        assert!(size_of::<Row<Cell>>() >= UPSTREAM_ROW_WORDS * size_of::<usize>());
        assert_eq!(term.grid().screen_lines(), ROWS);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn fork_serde는_row_cell을_왕복하고_runtime_marker는_저장하지_않는다() {
        let mut row = Row::<Cell>::new(COLUMNS);
        row[Column(0)].c = '한';
        #[cfg(feature = "native-retained")]
        let marker = row.native_marker();
        let json = serde_json::to_string(&row).unwrap();
        assert!(!json.contains("native_marker"));
        let restored: Row<Cell> = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, row);
        #[cfg(feature = "native-retained")]
        {
            assert!(restored.native_marker_ref().is_none());
            assert_eq!(marker.load(std::sync::atomic::Ordering::Relaxed), 1);
            drop(row);
            assert_eq!(marker.load(std::sync::atomic::Ordering::Relaxed), 0);
        }
    }
}
