pub const TREE_ROW_COUNT: usize = 10_000;
pub const TAB_COUNT: usize = 50;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Pane {
    Main,
    Auxiliary,
}

impl Pane {
    pub fn window_title(self) -> &'static str {
        match self {
            Self::Main => "TAIDE M8 egui spike",
            Self::Auxiliary => "TAIDE M8 auxiliary",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    Korean,
    English,
    Japanese,
}

impl Locale {
    pub fn label(self, key: &str) -> &str {
        match (self, key) {
            (Self::Korean, "tree") => "파일 탐색기",
            (Self::Korean, "auxiliary") => "보조 창",
            (Self::Korean, "dialog") => "파일 선택",
            (Self::Korean, "theme") => "어두운 테마",
            (Self::Korean, "input") => "IME 입력 실험",
            (Self::Korean, "move-tab") => "다른 창으로 탭 이동",
            (Self::Japanese, "tree") => "エクスプローラー",
            (Self::Japanese, "auxiliary") => "補助ウィンドウ",
            (Self::Japanese, "dialog") => "ファイルを選択",
            (Self::Japanese, "theme") => "ダークテーマ",
            (Self::Japanese, "input") => "IME 入力実験",
            (Self::Japanese, "move-tab") => "他のウィンドウへタブを移動",
            (_, "tree") => "Explorer",
            (_, "auxiliary") => "Auxiliary window",
            (_, "dialog") => "Choose file",
            (_, "theme") => "Dark theme",
            (_, "input") => "IME input probe",
            (_, "move-tab") => "Move to other window",
            _ => key,
        }
    }
}

pub struct ShellFixture {
    pub tabs: Vec<Pane>,
    pub active_main: Option<usize>,
    pub active_auxiliary: Option<usize>,
    pub selected_row: Option<usize>,
    pub input: String,
    pub is_dark: bool,
    pub locale: Locale,
}

impl Default for ShellFixture {
    fn default() -> Self {
        Self {
            tabs: vec![Pane::Main; TAB_COUNT],
            active_main: Some(0),
            active_auxiliary: None,
            selected_row: None,
            input: String::new(),
            is_dark: true,
            locale: Locale::English,
        }
    }
}

impl ShellFixture {
    pub fn finish_context_menu(
        &mut self,
        tab: usize,
        source: Pane,
        is_selected: bool,
    ) -> Option<(usize, Pane)> {
        if self.tabs.get(tab) != Some(&source) {
            return None;
        }
        if is_selected {
            self.move_tab_from(tab, source);
        }
        Some((tab, self.tabs[tab]))
    }

    pub fn move_tab_from(&mut self, tab: usize, source: Pane) -> bool {
        if self.tabs.get(tab) != Some(&source) {
            return false;
        }
        let destination = match source {
            Pane::Main => Pane::Auxiliary,
            Pane::Auxiliary => Pane::Main,
        };
        self.move_tab(tab, destination)
    }

    pub fn move_tab(&mut self, tab: usize, destination: Pane) -> bool {
        let Some(owner) = self.tabs.get_mut(tab) else {
            return false;
        };
        if *owner == destination {
            return false;
        }
        *owner = destination;
        if self.active_main == Some(tab) {
            self.active_main = self.tabs.iter().position(|pane| *pane == Pane::Main);
        }
        if self.active_auxiliary == Some(tab) {
            self.active_auxiliary = self.tabs.iter().position(|pane| *pane == Pane::Auxiliary);
        }
        match destination {
            Pane::Main => self.active_main = Some(tab),
            Pane::Auxiliary => self.active_auxiliary = Some(tab),
        }
        true
    }

    pub fn return_auxiliary_tabs(&mut self) {
        self.tabs.fill(Pane::Main);
        self.active_main = self.active_main.or(self.active_auxiliary);
        self.active_auxiliary = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_completion은_취소와_이동의_focus_대상을_구분하고_stale_source를_거절한다() {
        let mut fixture = ShellFixture::default();
        assert_eq!(
            fixture.finish_context_menu(0, Pane::Main, false),
            Some((0, Pane::Main))
        );
        assert_eq!(fixture.active_main, Some(0));
        assert_eq!(fixture.active_auxiliary, None);
        assert_eq!(fixture.finish_context_menu(0, Pane::Auxiliary, true), None);
        assert_eq!(
            fixture.finish_context_menu(TAB_COUNT, Pane::Main, true),
            None
        );
        assert_eq!(
            fixture.finish_context_menu(0, Pane::Main, true),
            Some((0, Pane::Auxiliary))
        );
        assert_eq!(fixture.finish_context_menu(0, Pane::Main, false), None);
        fixture.return_auxiliary_tabs();
        assert_eq!(fixture.finish_context_menu(0, Pane::Auxiliary, false), None);
        assert_eq!(
            fixture.finish_context_menu(0, Pane::Main, false),
            Some((0, Pane::Main))
        );
        assert_eq!(fixture.tabs.len(), TAB_COUNT);
    }

    #[test]
    fn context_action은_원래_창의_탭만_이동하고_잘못된_source를_거절한다() {
        let mut fixture = ShellFixture::default();
        assert_ne!(Pane::Main.window_title(), Pane::Auxiliary.window_title());
        assert!(!fixture.move_tab_from(0, Pane::Auxiliary));
        assert!(!fixture.move_tab_from(TAB_COUNT, Pane::Main));
        assert_eq!(fixture.active_main, Some(0));
        assert_eq!(fixture.active_auxiliary, None);
        assert!(fixture.move_tab_from(0, Pane::Main));
        assert_eq!(fixture.tabs[0], Pane::Auxiliary);
        assert_eq!(fixture.active_main, Some(1));
        assert_eq!(fixture.active_auxiliary, Some(0));
        assert!(!fixture.move_tab_from(0, Pane::Main));
        assert!(fixture.move_tab_from(0, Pane::Auxiliary));
        assert_eq!(fixture.tabs[0], Pane::Main);
        assert_eq!(fixture.active_main, Some(0));
        assert_eq!(fixture.active_auxiliary, None);
        assert_eq!(fixture.tabs.len(), TAB_COUNT);
        for locale in [Locale::English, Locale::Korean, Locale::Japanese] {
            assert_ne!(locale.label("move-tab"), "move-tab");
        }
    }

    #[test]
    fn 탭_이동은_식별자와_총수를_보존하고_선택을_복구한다() {
        let mut fixture = ShellFixture::default();
        assert!(fixture.move_tab(0, Pane::Auxiliary));
        assert_eq!(fixture.tabs.len(), TAB_COUNT);
        assert_eq!(fixture.active_main, Some(1));
        assert_eq!(fixture.active_auxiliary, Some(0));
        assert!(!fixture.move_tab(0, Pane::Auxiliary));
        assert!(!fixture.move_tab(TAB_COUNT, Pane::Main));
        assert!(fixture.move_tab(0, Pane::Main));
        assert_eq!(fixture.active_main, Some(0));
        assert_eq!(fixture.active_auxiliary, None);
    }

    #[test]
    fn 보조_창을_닫으면_모든_탭이_주_창으로_돌아온다() {
        let mut fixture = ShellFixture::default();
        for tab in 0..TAB_COUNT {
            assert!(fixture.move_tab(tab, Pane::Auxiliary));
        }
        assert_eq!(fixture.active_main, None);
        fixture.return_auxiliary_tabs();
        assert!(fixture.tabs.iter().all(|pane| *pane == Pane::Main));
        assert!(fixture.active_main.is_some());
        assert_eq!(fixture.active_auxiliary, None);
    }
}
