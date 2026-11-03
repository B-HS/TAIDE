use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui::{
    self, Color32, FontId, Id, Rect, Response, Sense, Ui, pos2, text::LayoutJob, vec2,
};
use taide_model::error::{AppError, AppResult};
use taide_model::locale::ResolvedLocale;
use taide_model::search::{
    ReplaceSkipReason, SEARCH_CONTEXT_LINES, SEARCH_MATCH_LIMIT, SearchQuery, SearchReplaceResult,
};
use taide_model::theme::ResolvedTheme;

use crate::explorer::PaintIcon;
use crate::navigation_icons::Icon;
use crate::problems_icons::Glyph;
use crate::search_results::{Accumulator, Action};
use crate::search_stream::{Session, Streams};

const RESULT_FLUSH_INTERVAL: f64 = 0.05;
const MILLISECONDS_PER_SECOND: f64 = 1000.0;
const PADDING_X: f32 = 8.0;
const PADDING_Y: f32 = 6.0;
const SECTION_GAP: f32 = 6.0;
const COLUMN_GAP: f32 = 4.0;
const BUTTON_SIZE: f32 = 24.0;
const BUTTON_ICON_SIZE: f32 = 14.0;
const BUTTON_OFFSET: f32 = 2.0;
const RADIUS: u8 = 4;
const BORDER_WIDTH: f32 = 1.0;
const TEXT_SIZE: f32 = 12.0;
const LINE_HEIGHT: f32 = 16.0;
const INPUT_PADDING_X: f32 = 8.0;
const INPUT_PADDING_Y: f32 = 6.0;
const INPUT_HEIGHT: f32 = LINE_HEIGHT + INPUT_PADDING_Y * 2.0 + BORDER_WIDTH * 2.0;
const TOGGLE_TRAILING_MARGIN: f32 = 4.0;
const DISABLED_OPACITY: f32 = 0.4;
const CHIP_PADDING_X: f32 = 6.0;
const CHIP_PADDING_Y: f32 = 2.0;
const CHIP_ICON_SIZE: f32 = 12.0;
const SPINNER_SIZE: f32 = 12.0;
const STATUS_GAP: f32 = 6.0;
const SPINNER_TURNS_PER_SECOND: f64 = 1.0;
const HINT_ICON_SIZE: f32 = 20.0;
const HINT_ICON_OPACITY: f32 = 0.6;
const HINT_GAP: f32 = 8.0;
const HINT_PADDING_X: f32 = 16.0;
const EDITOR_PADDING_X: f32 = 12.0;
const EDITOR_PADDING_Y: f32 = 8.0;
const EDITOR_ROW_GAP: f32 = 12.0;
const CONTEXT_LABEL_GAP: f32 = 6.0;
const CONTEXT_INPUT_WIDTH: f32 = 56.0;
const CONTEXT_INPUT_PADDING_X: f32 = 6.0;
const CONTEXT_INPUT_PADDING_Y: f32 = 4.0;
const CONTEXT_INPUT_HEIGHT: f32 = LINE_HEIGHT + CONTEXT_INPUT_PADDING_Y * 2.0 + BORDER_WIDTH * 2.0;
const CONFIRM_MAX_WIDTH: f32 = 384.0;
pub(crate) const MIN_CONTEXT_LINES: u32 = 0;
pub(crate) const MAX_CONTEXT_LINES: u32 = 10;

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Keeps a typed context-line count inside the search editor's range, truncating fractions and
/// reading anything unparsable as the minimum (`search-editor-context-lines.ts`).
pub(crate) fn clamp_context_lines(value: f64) -> u32 {
    if value.is_nan() {
        return MIN_CONTEXT_LINES;
    }
    value
        .trunc()
        .clamp(f64::from(MIN_CONTEXT_LINES), f64::from(MAX_CONTEXT_LINES)) as u32
}

/// Whether two queries select the same matches; the preview-only context line count is ignored
/// (`search-query.ts`).
pub(crate) fn is_same_query(left: &SearchQuery, right: &SearchQuery) -> bool {
    left.text == right.text
        && left.case_sensitive == right.case_sensitive
        && left.whole_word == right.whole_word
        && left.regex == right.regex
        && left.include_glob == right.include_glob
        && left.exclude_glob == right.exclude_glob
        && left.respect_gitignore == right.respect_gitignore
        && left.scope_dir == right.scope_dir
}

/// How many skipped files a replace warning names before "and N more"
/// (`replace-skip-report.ts` `REPLACE_SKIP_LIST_LIMIT`).
pub(crate) const SKIP_LIST_LIMIT: usize = 5;

pub(crate) fn skip_reason_key(reason: ReplaceSkipReason) -> &'static str {
    match reason {
        ReplaceSkipReason::TooLarge => "search.replaceSkipReason.tooLarge",
        ReplaceSkipReason::Binary => "search.replaceSkipReason.binary",
        ReplaceSkipReason::NotUtf8 => "search.replaceSkipReason.notUtf8",
        ReplaceSkipReason::Unreadable => "search.replaceSkipReason.unreadable",
        ReplaceSkipReason::WriteFailed => "search.replaceSkipReason.writeFailed",
    }
}

/// The warning a replace with skipped files raises: a count title and one `path — reason` line
/// per listed file, then the remainder the backend's capped list and this cap left out.
pub(crate) struct SkipReport {
    pub(crate) title: String,
    pub(crate) description: String,
}

pub(crate) fn skip_report(
    result: &SearchReplaceResult,
    locale: &ResolvedLocale,
) -> Option<SkipReport> {
    if result.skipped_count == 0 {
        return None;
    }
    let listed = &result.skipped[..result.skipped.len().min(SKIP_LIST_LIMIT)];
    let remaining = (result.skipped_count as usize).saturating_sub(listed.len());
    let mut lines = listed
        .iter()
        .map(|file| {
            format!(
                "{} — {}",
                file.path,
                crate::presentation::message(locale, skip_reason_key(file.reason), &[])
            )
        })
        .collect::<Vec<_>>();
    if remaining > 0 {
        let count = remaining.to_string();
        lines.push(crate::presentation::message(
            locale,
            "search.replaceSkippedMore",
            &[("count", &count)],
        ));
    }
    let total = result.skipped_count.to_string();
    Some(SkipReport {
        title: crate::presentation::message(locale, "search.replaceSkipped", &[("count", &total)]),
        description: lines.join("\n"),
    })
}

/// Where a panel is mounted: the sidebar Search view or a search editor tab body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Surface {
    Sidebar,
    Editor,
}

/// Colors of the search panel and its result list.
pub(crate) struct Appearance {
    pub(crate) background: Color32,
    pub(crate) editor_background: Color32,
    pub(crate) input_background: Color32,
    pub(crate) input_border: Color32,
    pub(crate) focus_border: Color32,
    pub(crate) border: Color32,
    pub(crate) foreground: Color32,
    pub(crate) muted: Color32,
    pub(crate) hover: Color32,
    pub(crate) selected: Color32,
    pub(crate) match_highlight: Color32,
    pub(crate) checkbox_border: Color32,
    pub(crate) checkbox_checked: Color32,
    pub(crate) checkbox_check: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        use crate::presentation::color;
        Ok(Self {
            background: color(theme, "panel.background")?,
            editor_background: color(theme, "editor.background")?,
            input_background: color(theme, "panel.inputBackground")?,
            input_border: color(theme, "panel.inputBorder")?,
            focus_border: color(theme, "app.focusBorder")?,
            border: color(theme, "app.border")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            hover: color(theme, "explorer.itemHover")?,
            selected: color(theme, "explorer.itemSelected")?,
            match_highlight: color(theme, "panel.matchHighlight")?,
            checkbox_border: color(theme, "input.border")?,
            checkbox_checked: color(theme, "button.primaryBackground")?,
            checkbox_check: color(theme, "button.primaryForeground")?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResultsView {
    Hint,
    Searching,
    Failed,
    Empty,
    Results,
}

/// Results win over every status, since matches stream in while the run is still going.
pub(crate) fn resolve_view(status: Status, has_results: bool) -> ResultsView {
    if has_results {
        return ResultsView::Results;
    }
    match status {
        Status::Idle => ResultsView::Hint,
        Status::Running => ResultsView::Searching,
        Status::Failed => ResultsView::Failed,
        Status::Completed => ResultsView::Empty,
    }
}

/// Whether the backend stopped at its exact match budget rather than at the project's end.
pub(crate) fn is_truncated(total: u32) -> bool {
    total >= SEARCH_MATCH_LIMIT
}

/// What opened the panel: a folder or glob scope, a seed term, and whether replace starts open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) include_glob: Option<String>,
    pub(crate) scope_dir: Option<String>,
    pub(crate) seed_text: Option<String>,
    pub(crate) open_replace: bool,
}

impl Request {
    pub(crate) fn without_scope(&self) -> Self {
        Self {
            include_glob: None,
            scope_dir: None,
            ..self.clone()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) is_live: bool,
    pub(crate) debounce_ms: u32,
}

pub(crate) struct Inputs<'a> {
    pub(crate) request: Option<&'a Request>,
    pub(crate) nonce: u64,
    pub(crate) settings: Settings,
    pub(crate) history: &'a [String],
    pub(crate) streams: &'a Streams,
    pub(crate) appearance: &'a crate::explorer::Appearance,
    pub(crate) locale: &'a ResolvedLocale,
}

pub struct Run {
    pub(crate) session: Session,
    pub(crate) generation: u64,
    pub(crate) query: SearchQuery,
    pub(crate) record: Option<String>,
}

pub struct Open {
    pub(crate) path: String,
    pub(crate) line: u32,
    pub(crate) column: u32,
}

/// A confirmed Replace All: the query the shown results came from and the checked files.
pub struct Replace {
    pub(crate) query: SearchQuery,
    pub(crate) replacement: String,
    pub(crate) paths: Option<Vec<String>>,
}

#[derive(Default)]
pub(crate) struct Output {
    pub(crate) run: Option<Run>,
    pub(crate) open: Option<Open>,
    pub(crate) replace: Option<Replace>,
    pub(crate) open_editor: Option<SearchQuery>,
    pub(crate) clear_scope: bool,
    pub(crate) tooltips: Vec<(Response, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Signature {
    text: String,
    toggles: [bool; 4],
    exclude_glob: String,
    include_glob: Option<String>,
    scope_dir: Option<String>,
    settings: Settings,
}

/// One mounted search panel: its inputs, its latest run and that run's streamed results.
pub(crate) struct Panel {
    session: Session,
    surface: Surface,
    query: String,
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
    respect_gitignore: bool,
    exclude_glob: String,
    context_lines: u32,
    seeded_nonce: Option<u64>,
    replace_open: bool,
    replace_text: String,
    ran_query: Option<SearchQuery>,
    is_replacing: bool,
    confirm: Option<bool>,
    excluded: HashSet<String>,
    collapsed: HashSet<String>,
    generation: u64,
    status: Status,
    is_live_run: bool,
    results: Accumulator,
    total: u32,
    flush_deadline: Option<f64>,
    live_deadline: Option<f64>,
    live_signature: Option<Signature>,
    is_live_deferred: bool,
    composing: bool,
}

const TOGGLES: [(&str, Icon); 4] = [
    ("search.caseSensitive", Icon::CaseSensitive),
    ("search.wholeWord", Icon::WholeWord),
    ("search.regex", Icon::Regex),
    ("search.respectGitignore", Icon::EyeOff),
];

impl Panel {
    pub(crate) fn new(session: Session) -> Self {
        Self {
            session,
            surface: Surface::Sidebar,
            query: String::new(),
            case_sensitive: false,
            whole_word: false,
            regex: false,
            respect_gitignore: true,
            exclude_glob: String::new(),
            context_lines: SEARCH_CONTEXT_LINES,
            seeded_nonce: None,
            replace_open: false,
            replace_text: String::new(),
            ran_query: None,
            is_replacing: false,
            confirm: None,
            excluded: HashSet::new(),
            collapsed: HashSet::new(),
            generation: 0,
            status: Status::Idle,
            is_live_run: false,
            results: Accumulator::default(),
            total: 0,
            flush_deadline: None,
            live_deadline: None,
            live_signature: None,
            is_live_deferred: false,
            composing: false,
        }
    }

    /// A search editor tab body starting from the tab's persisted query.
    pub(crate) fn editor(session: Session, query: &SearchQuery) -> Self {
        Self {
            surface: Surface::Editor,
            query: query.text.clone(),
            case_sensitive: query.case_sensitive,
            whole_word: query.whole_word,
            regex: query.regex,
            respect_gitignore: query.respect_gitignore,
            exclude_glob: query.exclude_glob.clone().unwrap_or_default(),
            context_lines: query.context_lines,
            ..Self::new(session)
        }
    }

    /// The backend session to cancel when this panel goes away while its run is still going.
    pub(crate) fn running_session(&self) -> Option<Session> {
        (self.status == Status::Running).then(|| self.session.clone())
    }

    /// Leaves a running search the way an unmount does: cancelled backend work, results kept.
    pub(crate) fn stop(&mut self) -> Option<Session> {
        let session = self.running_session()?;
        self.status = Status::Completed;
        self.flush_deadline = None;
        Some(session)
    }

    /// Runs the tab's own query once when a search editor first mounts with a non-empty term.
    pub(crate) fn auto_run(&mut self, streams: &Streams, request: Option<&Request>) -> Option<Run> {
        if self.query.is_empty() {
            return None;
        }
        let query = self.search_query(request);
        Some(self.start_run(streams, query, None, false))
    }

    /// Whether this panel started the Replace All a reply settles; a remounted panel did not.
    pub(crate) fn is_replacing(&self) -> bool {
        self.is_replacing
    }

    /// Settles a Replace All and, on success, re-runs the replaced query without recording it.
    pub(crate) fn replace_finished(
        &mut self,
        streams: &Streams,
        query: Option<SearchQuery>,
    ) -> Option<Run> {
        self.is_replacing = false;
        Some(self.start_run(streams, query?, None, false))
    }

    /// Settles the run `generation`; a failure of an explicit run comes back to be reported.
    pub(crate) fn finish(
        &mut self,
        streams: &Streams,
        generation: u64,
        result: AppResult<u32>,
    ) -> Option<AppError> {
        if generation != self.generation || self.status != Status::Running {
            return None;
        }
        match result {
            Ok(total) => {
                self.flush_deadline = None;
                self.drain(streams);
                self.total = total;
                self.status = Status::Completed;
                None
            }
            Err(error) => {
                self.status = Status::Failed;
                (!self.is_live_run).then_some(error)
            }
        }
    }

    fn drain(&mut self, streams: &Streams) {
        let batches = streams.take(&self.session, self.generation);
        if !batches.is_empty() {
            self.results.append(batches);
            self.total = self.results.total();
        }
    }

    fn flush(&mut self, now: f64, streams: &Streams) -> Option<f64> {
        if self.status != Status::Running {
            return None;
        }
        match self.flush_deadline {
            Some(deadline) if now >= deadline => {
                self.flush_deadline = None;
                self.drain(streams);
                Some(RESULT_FLUSH_INTERVAL)
            }
            Some(deadline) => Some(deadline - now),
            None if streams.has(&self.session, self.generation) => {
                self.flush_deadline = Some(now + RESULT_FLUSH_INTERVAL);
                Some(RESULT_FLUSH_INTERVAL)
            }
            None => Some(RESULT_FLUSH_INTERVAL),
        }
    }

    fn search_query(&self, request: Option<&Request>) -> SearchQuery {
        SearchQuery {
            text: self.query.clone(),
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            regex: self.regex,
            include_glob: request.and_then(|request| request.include_glob.clone()),
            exclude_glob: Some(self.exclude_glob.trim())
                .filter(|glob| !glob.is_empty())
                .map(str::to_owned),
            context_lines: self.context_lines,
            respect_gitignore: self.respect_gitignore,
            scope_dir: request.and_then(|request| request.scope_dir.clone()),
        }
    }

    fn start_run(
        &mut self,
        streams: &Streams,
        query: SearchQuery,
        record: Option<String>,
        is_live: bool,
    ) -> Run {
        self.generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        self.results = Accumulator::default();
        self.total = 0;
        self.status = Status::Running;
        self.is_live_run = is_live;
        self.flush_deadline = None;
        self.collapsed.clear();
        self.ran_query = Some(query.clone());
        streams.begin(&self.session, self.generation);
        Run {
            session: self.session.clone(),
            generation: self.generation,
            query,
            record,
        }
    }

    fn submit(&mut self, streams: &Streams, request: Option<&Request>, is_live: bool) -> Run {
        let query = self.search_query(request);
        let record = (!is_live).then(|| self.query.clone());
        self.start_run(streams, query, record, is_live)
    }

    fn arm_live(&mut self, now: f64, settings: Settings) {
        if settings.is_live && !self.query.trim().is_empty() {
            self.live_deadline =
                Some(now + f64::from(settings.debounce_ms) / MILLISECONDS_PER_SECOND);
        }
    }

    fn observe_live(&mut self, now: f64, request: Option<&Request>, settings: Settings) {
        let signature = Signature {
            text: self.query.clone(),
            toggles: [
                self.case_sensitive,
                self.whole_word,
                self.regex,
                self.respect_gitignore,
            ],
            exclude_glob: self.exclude_glob.clone(),
            include_glob: request.and_then(|request| request.include_glob.clone()),
            scope_dir: request.and_then(|request| request.scope_dir.clone()),
            settings,
        };
        if self.live_signature.as_ref() != Some(&signature) {
            self.live_signature = Some(signature);
            self.live_deadline = None;
            self.is_live_deferred = self.composing;
            if !self.composing {
                self.arm_live(now, settings);
            }
        } else if self.is_live_deferred && !self.composing {
            self.is_live_deferred = false;
            self.arm_live(now, settings);
        }
    }

    fn take_enter(&mut self, ui: &Ui, id: Id) -> bool {
        if !ui.is_enabled() || !ui.memory(|memory| memory.has_focus(id)) {
            return false;
        }
        let mut is_ime_frame = false;
        let mut is_enter = false;
        ui.input_mut(|input| {
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    is_ime_frame = true;
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
            input.events.retain(|event| match event {
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                } => {
                    is_enter = true;
                    false
                }
                _ => true,
            });
        });
        is_enter && !is_ime_frame && !self.composing
    }

    fn paint_box(ui: &Ui, rect: Rect, id: Id, colors: &Appearance) {
        let border = if ui.memory(|memory| memory.has_focus(id)) {
            colors.focus_border
        } else {
            colors.input_border
        };
        ui.painter()
            .rect_filled(rect, RADIUS, colors.input_background);
        ui.painter().rect_stroke(
            rect,
            RADIUS,
            egui::Stroke::new(BORDER_WIDTH, border),
            egui::StrokeKind::Inside,
        );
    }

    fn input(
        ui: &mut Ui,
        id: Id,
        rect: Rect,
        text: &mut String,
        placeholder: &str,
        colors: &Appearance,
    ) -> egui::text_edit::TextEditOutput {
        let field = rect.shrink2(vec2(INPUT_PADDING_X, BORDER_WIDTH));
        let mut field_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(field)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let output = egui::TextEdit::singleline(text)
            .id(id)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(field.width())
            .font(FontId::proportional(TEXT_SIZE))
            .text_color(colors.foreground)
            .hint_text(placeholder)
            .return_key(None)
            .show(&mut field_ui);
        output.response.response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, ui.is_enabled(), placeholder)
        });
        output
    }

    fn icon_button<I: PaintIcon<Icon>>(
        ui: &Ui,
        rect: Rect,
        id: Id,
        icon: Icon,
        is_active: bool,
        colors: &Appearance,
        icons: &mut I,
    ) -> AppResult<Response> {
        let response = ui.interact(rect, id, Sense::click());
        if is_active || response.hovered() {
            ui.painter().rect_filled(
                rect,
                RADIUS,
                if is_active {
                    colors.selected
                } else {
                    colors.hover
                },
            );
        }
        icons.paint(
            ui,
            Rect::from_center_size(rect.center(), vec2(BUTTON_ICON_SIZE, BUTTON_ICON_SIZE)),
            icon,
            if is_active {
                colors.foreground
            } else {
                colors.muted
            },
            0.0,
        )?;
        Ok(response)
    }

    fn paint_centered(ui: &Ui, center_x: f32, top: f32, width: f32, text: String, color: Color32) {
        let mut job = LayoutJob::simple(text, FontId::proportional(TEXT_SIZE), color, width);
        job.halign = egui::Align::Center;
        let galley = ui.painter().layout_job(job);
        ui.painter().galley(pos2(center_x, top), galley, color);
    }

    pub(crate) fn show<I>(
        &mut self,
        ui: &mut Ui,
        id: Id,
        inputs: Inputs<'_>,
        icons: &mut I,
    ) -> AppResult<Output>
    where
        I: PaintIcon<Glyph> + PaintIcon<Icon>,
    {
        let colors = inputs.appearance.search();
        let locale = inputs.locale;
        let message = |key: &str| crate::presentation::message(locale, key, &[]);
        let mut output = Output::default();
        let now = ui.input(|input| input.time);
        let is_new_nonce = self.seeded_nonce != Some(inputs.nonce);
        if is_new_nonce {
            self.seeded_nonce = Some(inputs.nonce);
            if let Some(request) = inputs.request {
                if let Some(seed) = &request.seed_text {
                    self.query = seed.clone();
                }
                if request.open_replace {
                    self.replace_open = true;
                }
            }
        }
        let mut repaint_after = self.flush(now, inputs.streams);
        let is_sidebar = self.surface == Surface::Sidebar;
        let (padding_x, padding_y, background) = if is_sidebar {
            (PADDING_X, PADDING_Y, colors.background)
        } else {
            (EDITOR_PADDING_X, EDITOR_PADDING_Y, colors.editor_background)
        };
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, background);
        let left = rect.left() + padding_x;
        let right = (rect.right() - padding_x).max(left);
        let top = rect.top() + padding_y;

        let toggle_rect = Rect::from_min_size(
            pos2(left, top + BUTTON_OFFSET),
            vec2(BUTTON_SIZE, BUTTON_SIZE),
        );
        let editor_button_rect = Rect::from_min_size(
            pos2(right - BUTTON_SIZE, top + BUTTON_OFFSET),
            vec2(BUTTON_SIZE, BUTTON_SIZE),
        );
        let (column_left, column_right) = if is_sidebar {
            (
                (toggle_rect.right() + COLUMN_GAP).min(right),
                (editor_button_rect.left() - COLUMN_GAP).max(left),
            )
        } else {
            (left, right)
        };
        if is_sidebar {
            let replace_toggle =
                ui.interact(toggle_rect, id.with("replace-toggle"), Sense::click());
            if replace_toggle.hovered() {
                ui.painter().rect_filled(toggle_rect, RADIUS, colors.hover);
            }
            PaintIcon::<Glyph>::paint(
                icons,
                ui,
                Rect::from_center_size(
                    toggle_rect.center(),
                    vec2(BUTTON_ICON_SIZE, BUTTON_ICON_SIZE),
                ),
                Glyph::ChevronRight,
                colors.muted,
                if self.replace_open {
                    std::f32::consts::FRAC_PI_2
                } else {
                    0.0
                },
            )?;
            let replace_label = message("search.replaceToggle");
            replace_toggle.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Button,
                    ui.is_enabled(),
                    self.replace_open,
                    &replace_label,
                )
            });
            if replace_toggle.clicked() {
                self.replace_open = !self.replace_open;
            }
            output.tooltips.push((replace_toggle, replace_label));

            let editor_label = message("searchEditor.title");
            let open_editor = Self::icon_button(
                ui,
                editor_button_rect,
                id.with("open-editor"),
                Icon::FileSearch,
                false,
                colors,
                icons,
            )?;
            open_editor.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &editor_label)
            });
            if open_editor.clicked() {
                output.open_editor = Some(self.search_query(inputs.request));
            }
            output.tooltips.push((open_editor, editor_label));
        }

        let query_box = Rect::from_min_max(
            pos2(column_left, top),
            pos2(column_right, top + INPUT_HEIGHT),
        );
        let mut button_right = query_box.right() - BORDER_WIDTH - TOGGLE_TRAILING_MARGIN;
        let mut toggle_rects = [Rect::NOTHING; 4];
        for rect in toggle_rects.iter_mut().rev() {
            *rect = Rect::from_center_size(
                pos2(button_right - BUTTON_SIZE / 2.0, query_box.center().y),
                vec2(BUTTON_SIZE, BUTTON_SIZE),
            );
            button_right -= BUTTON_SIZE;
        }
        let history_rect = Rect::from_center_size(
            pos2(button_right - BUTTON_SIZE / 2.0, query_box.center().y),
            vec2(BUTTON_SIZE, BUTTON_SIZE),
        );
        let query_id = id.with("query");
        let is_enter = self.take_enter(ui, query_id);
        let placeholder = message("search.placeholder");
        let input_rect = Rect::from_min_max(
            query_box.min,
            pos2(
                history_rect.left().max(query_box.left()),
                query_box.bottom(),
            ),
        );
        Self::paint_box(ui, query_box, query_id, colors);
        let mut query_output = Self::input(
            ui,
            query_id,
            input_rect,
            &mut self.query,
            &placeholder,
            colors,
        );
        if is_new_nonce && ui.is_enabled() {
            query_output.response.response.request_focus();
        }
        if is_new_nonce && ui.is_enabled() && is_sidebar {
            query_output
                .state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(self.query.chars().count()),
                )));
            query_output.state.store(ui.ctx(), query_id);
        }

        let history_label = message("search.recentSearches");
        let has_history = !inputs.history.is_empty();
        let history = ui.interact(
            history_rect,
            id.with("history"),
            if has_history {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        if has_history && history.hovered() {
            ui.painter().rect_filled(history_rect, RADIUS, colors.hover);
        }
        PaintIcon::<Icon>::paint(
            icons,
            ui,
            Rect::from_center_size(
                history_rect.center(),
                vec2(BUTTON_ICON_SIZE, BUTTON_ICON_SIZE),
            ),
            Icon::History,
            if has_history {
                colors.muted
            } else {
                colors.muted.gamma_multiply(DISABLED_OPACITY)
            },
            0.0,
        )?;
        history.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, has_history, &history_label)
        });
        let mut chosen = None;
        egui::Popup::menu(&history).show(|ui| {
            for term in inputs.history {
                if ui.button(term).clicked() {
                    chosen = Some(term.clone());
                }
            }
        });
        if let Some(term) = chosen {
            self.query = term;
        }
        if has_history {
            output.tooltips.push((history, history_label));
        }

        let states = [
            self.case_sensitive,
            self.whole_word,
            self.regex,
            self.respect_gitignore,
        ];
        let mut flipped = [false; 4];
        for (index, ((key, icon), rect)) in TOGGLES.into_iter().zip(toggle_rects).enumerate() {
            let response =
                Self::icon_button(ui, rect, id.with(key), icon, states[index], colors, icons)?;
            let label = message(key);
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Button,
                    ui.is_enabled(),
                    states[index],
                    &label,
                )
            });
            flipped[index] = response.clicked();
            output.tooltips.push((response, label));
        }
        for (value, flip) in [
            &mut self.case_sensitive,
            &mut self.whole_word,
            &mut self.regex,
            &mut self.respect_gitignore,
        ]
        .into_iter()
        .zip(flipped)
        {
            if flip {
                *value = !*value;
            }
        }

        let font = FontId::proportional(TEXT_SIZE);
        let mut column_bottom = query_box.bottom();
        if is_sidebar && self.replace_open {
            let current = self.search_query(inputs.request);
            let is_query_shown = self
                .ran_query
                .as_ref()
                .is_some_and(|ran| is_same_query(ran, &current));
            let has_selection = self
                .results
                .groups()
                .iter()
                .any(|group| !self.excluded.contains(&group.path));
            let can_replace = has_selection && !self.is_replacing && is_query_shown;
            let replace_box = Rect::from_min_max(
                pos2(column_left, column_bottom + COLUMN_GAP),
                pos2(column_right, column_bottom + COLUMN_GAP + INPUT_HEIGHT),
            );
            let replace_id = id.with("replace");
            let button_rect = Rect::from_center_size(
                pos2(
                    replace_box.right() - BORDER_WIDTH - TOGGLE_TRAILING_MARGIN - BUTTON_SIZE / 2.0,
                    replace_box.center().y,
                ),
                vec2(BUTTON_SIZE, BUTTON_SIZE),
            );
            Self::paint_box(ui, replace_box, replace_id, colors);
            Self::input(
                ui,
                replace_id,
                Rect::from_min_max(
                    replace_box.min,
                    pos2(
                        button_rect.left().max(replace_box.left()),
                        replace_box.bottom(),
                    ),
                ),
                &mut self.replace_text,
                &message("search.replacePlaceholder"),
                colors,
            );
            let replace_label = message("search.replaceAll");
            let replace_all = ui.interact(
                button_rect,
                id.with("replace-all"),
                if can_replace {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            if can_replace && replace_all.hovered() {
                ui.painter().rect_filled(button_rect, RADIUS, colors.hover);
            }
            PaintIcon::<Icon>::paint(
                icons,
                ui,
                Rect::from_center_size(
                    button_rect.center(),
                    vec2(BUTTON_ICON_SIZE, BUTTON_ICON_SIZE),
                ),
                Icon::ReplaceAll,
                if can_replace {
                    colors.muted
                } else {
                    colors.muted.gamma_multiply(DISABLED_OPACITY)
                },
                0.0,
            )?;
            replace_all.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, can_replace, &replace_label)
            });
            if can_replace && replace_all.clicked() {
                self.confirm = Some(true);
            }
            if can_replace {
                output.tooltips.push((replace_all, replace_label));
            }
            column_bottom = replace_box.bottom();
            if !self.results.groups().is_empty() && !is_query_shown {
                let galley = ui.painter().layout(
                    message("search.replaceStaleHint"),
                    font.clone(),
                    colors.muted,
                    (column_right - column_left).max(0.0),
                );
                let hint_top = column_bottom + COLUMN_GAP;
                column_bottom = hint_top + galley.size().y;
                ui.painter()
                    .galley(pos2(column_left, hint_top), galley, colors.muted);
            }
        }
        let exclude_top = column_bottom + if is_sidebar { COLUMN_GAP } else { SECTION_GAP };
        let mut exclude_right = column_right;
        if !is_sidebar {
            let input_rect = Rect::from_min_size(
                pos2(
                    right - CONTEXT_INPUT_WIDTH,
                    exclude_top + (INPUT_HEIGHT - CONTEXT_INPUT_HEIGHT) / 2.0,
                ),
                vec2(CONTEXT_INPUT_WIDTH, CONTEXT_INPUT_HEIGHT),
            );
            let label = message("searchEditor.contextLinesLabel");
            let galley = ui
                .painter()
                .layout_no_wrap(label.clone(), font.clone(), colors.muted);
            let label_left = input_rect.left() - CONTEXT_LABEL_GAP - galley.size().x;
            ui.painter().galley(
                pos2(label_left, input_rect.center().y - galley.size().y / 2.0),
                galley,
                colors.muted,
            );
            let context_id = id.with("context-lines");
            Self::paint_box(ui, input_rect, context_id, colors);
            let mut text = self.context_lines.to_string();
            let field = input_rect.shrink2(vec2(CONTEXT_INPUT_PADDING_X, BORDER_WIDTH));
            let mut field_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(field)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            let response = egui::TextEdit::singleline(&mut text)
                .id(context_id)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .desired_width(field.width())
                .font(FontId::proportional(TEXT_SIZE))
                .text_color(colors.foreground)
                .return_key(None)
                .show(&mut field_ui)
                .response
                .response;
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, ui.is_enabled(), &label)
            });
            if response.changed() {
                self.context_lines = clamp_context_lines(text.trim().parse().unwrap_or(f64::NAN));
            }
            exclude_right = (label_left - EDITOR_ROW_GAP).max(column_left);
        }
        let exclude_rect = Rect::from_min_max(
            pos2(column_left, exclude_top),
            pos2(exclude_right, exclude_top + INPUT_HEIGHT),
        );
        let exclude_id = id.with("exclude");
        Self::paint_box(ui, exclude_rect, exclude_id, colors);
        Self::input(
            ui,
            exclude_id,
            exclude_rect,
            &mut self.exclude_glob,
            &message("search.excludeGlobPlaceholder"),
            colors,
        );
        let mut bottom = exclude_rect.bottom().max(toggle_rect.bottom());

        let scope_path = inputs.request.filter(|_| is_sidebar).and_then(|request| {
            request
                .scope_dir
                .as_deref()
                .or(request.include_glob.as_deref())
        });
        if let Some(path) = scope_path {
            let chip_top = bottom + SECTION_GAP;
            let text = crate::presentation::message(
                locale,
                "explorer.searchScopeLabel",
                &[("path", path)],
            );
            let available =
                (right - left - CHIP_PADDING_X * 2.0 - COLUMN_GAP - CHIP_ICON_SIZE).max(0.0);
            let text_width = ui
                .painter()
                .layout_no_wrap(text.clone(), font.clone(), colors.foreground)
                .size()
                .x
                .min(available);
            let chip = Rect::from_min_size(
                pos2(left, chip_top),
                vec2(
                    text_width + COLUMN_GAP + CHIP_ICON_SIZE + CHIP_PADDING_X * 2.0,
                    LINE_HEIGHT + CHIP_PADDING_Y * 2.0,
                ),
            );
            ui.painter().rect_filled(chip, RADIUS, colors.selected);
            let text_rect = Rect::from_min_size(
                pos2(chip.left() + CHIP_PADDING_X, chip.top()),
                vec2(text_width, chip.height()),
            );
            let mut job = LayoutJob::simple(text, font.clone(), colors.foreground, text_width);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            let galley = ui.painter().layout_job(job);
            ui.painter()
                .with_clip_rect(text_rect.intersect(ui.clip_rect()))
                .galley(
                    pos2(
                        text_rect.left(),
                        text_rect.center().y - galley.size().y / 2.0,
                    ),
                    galley,
                    colors.foreground,
                );
            let clear_rect = Rect::from_center_size(
                pos2(
                    text_rect.right() + COLUMN_GAP + CHIP_ICON_SIZE / 2.0,
                    chip.center().y,
                ),
                vec2(CHIP_ICON_SIZE, CHIP_ICON_SIZE),
            );
            let clear = ui.interact(clear_rect, id.with("clear-scope"), Sense::click());
            PaintIcon::<Glyph>::paint(
                icons,
                ui,
                clear_rect,
                Glyph::X,
                if clear.hovered() {
                    colors.foreground
                } else {
                    colors.muted
                },
                0.0,
            )?;
            let clear_label = message("search.clearScope");
            clear.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &clear_label)
            });
            output.clear_scope = clear.clicked();
            output.tooltips.push((clear, clear_label));
            bottom = chip.bottom();
        }

        let status_line = |text: String, color: Color32, icon_width: f32, bottom: &mut f32| {
            let line = Rect::from_min_size(
                pos2(left, *bottom + SECTION_GAP),
                vec2(right - left, LINE_HEIGHT),
            );
            let content = Rect::from_min_max(pos2(line.left() + icon_width, line.top()), line.max);
            let mut job = LayoutJob::simple(text, font.clone(), color, content.width());
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            let galley = ui.painter().layout_job(job);
            ui.painter().galley(
                pos2(content.left(), content.center().y - galley.size().y / 2.0),
                galley,
                color,
            );
            *bottom = line.bottom();
            line
        };
        match self.status {
            Status::Running => {
                let line = status_line(
                    message("search.searching"),
                    colors.muted,
                    SPINNER_SIZE + STATUS_GAP,
                    &mut bottom,
                );
                let angle = (now * SPINNER_TURNS_PER_SECOND).fract() * std::f64::consts::TAU;
                PaintIcon::<Icon>::paint(
                    icons,
                    ui,
                    Rect::from_center_size(
                        pos2(line.left() + SPINNER_SIZE / 2.0, line.center().y),
                        vec2(SPINNER_SIZE, SPINNER_SIZE),
                    ),
                    Icon::Loader,
                    colors.muted,
                    angle as f32,
                )?;
            }
            Status::Completed => {
                let summary = if self.total > 0 {
                    let count = self.total.to_string();
                    let files = self.results.groups().len().to_string();
                    crate::presentation::message(
                        locale,
                        "search.matchCount",
                        &[("count", &count), ("files", &files)],
                    )
                } else if is_sidebar {
                    message("search.noMatches")
                } else {
                    message("searchEditor.noResults")
                };
                status_line(summary, colors.muted, 0.0, &mut bottom);
                if is_truncated(self.total) {
                    let limit = SEARCH_MATCH_LIMIT.to_string();
                    status_line(
                        crate::presentation::message(
                            locale,
                            "search.truncated",
                            &[("limit", &limit)],
                        ),
                        colors.match_highlight,
                        0.0,
                        &mut bottom,
                    );
                }
            }
            Status::Idle | Status::Failed => {}
        }
        let header_bottom = bottom + padding_y;
        ui.painter().hline(
            rect.x_range(),
            header_bottom,
            egui::Stroke::new(BORDER_WIDTH, colors.border),
        );

        let body = Rect::from_min_max(
            pos2(
                rect.left(),
                (header_bottom + BORDER_WIDTH).min(rect.bottom()),
            ),
            rect.max,
        );
        let view = resolve_view(self.status, !self.results.groups().is_empty());
        match view {
            ResultsView::Results => {
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
                let actions = crate::search_results::show(
                    &mut body_ui,
                    id.with("results"),
                    crate::search_results::List {
                        groups: self.results.groups(),
                        collapsed: &self.collapsed,
                        excluded: self.replace_open.then_some(&self.excluded),
                    },
                    inputs.appearance,
                    locale,
                    icons,
                )?;
                for action in actions {
                    match action {
                        Action::Toggle(path) => {
                            if !self.collapsed.remove(&path) {
                                self.collapsed.insert(path);
                            }
                        }
                        Action::Select(path) => {
                            if !self.excluded.remove(&path) {
                                self.excluded.insert(path);
                            }
                        }
                        Action::Open { path, line, column } => {
                            output.open = Some(Open { path, line, column })
                        }
                    }
                }
            }
            ResultsView::Hint => {
                let text = message(if is_sidebar && inputs.settings.is_live {
                    "search.liveSearchHint"
                } else {
                    "search.pressEnterHint"
                });
                let width = (body.width() - HINT_PADDING_X * 2.0).max(0.0);
                let galley = ui.painter().layout_job({
                    let mut job =
                        LayoutJob::simple(text.clone(), font.clone(), colors.muted, width);
                    job.halign = egui::Align::Center;
                    job
                });
                let height = HINT_ICON_SIZE + HINT_GAP + galley.size().y;
                let icon_top = body.center().y - height / 2.0;
                PaintIcon::<Icon>::paint(
                    icons,
                    ui,
                    Rect::from_min_size(
                        pos2(body.center().x - HINT_ICON_SIZE / 2.0, icon_top),
                        vec2(HINT_ICON_SIZE, HINT_ICON_SIZE),
                    ),
                    Icon::Search,
                    colors.muted.gamma_multiply(HINT_ICON_OPACITY),
                    0.0,
                )?;
                Self::paint_centered(
                    ui,
                    body.center().x,
                    icon_top + HINT_ICON_SIZE + HINT_GAP,
                    width,
                    text,
                    colors.muted,
                );
            }
            ResultsView::Empty | ResultsView::Failed => {
                let text = message(match (view, is_sidebar) {
                    (ResultsView::Failed, _) => "search.failed",
                    (_, true) => "search.noResults",
                    (_, false) => "searchEditor.noResults",
                });
                let width = (body.width() - HINT_PADDING_X * 2.0).max(0.0);
                Self::paint_centered(
                    ui,
                    body.center().x,
                    body.center().y - LINE_HEIGHT / 2.0,
                    width,
                    text,
                    colors.muted,
                );
            }
            ResultsView::Searching => {}
        }
        ui.allocate_rect(rect, Sense::hover());
        if let Some(focus_cancel) = self.confirm {
            output.replace = self.show_confirm(ui, id, focus_cancel, locale);
        }

        if is_sidebar {
            self.observe_live(now, inputs.request, inputs.settings);
        }
        if is_enter && !self.query.trim().is_empty() {
            self.live_deadline = None;
            output.run = Some(self.submit(inputs.streams, inputs.request, false));
            repaint_after = Some(RESULT_FLUSH_INTERVAL);
        }
        if let Some(deadline) = self.live_deadline {
            if now >= deadline {
                self.live_deadline = None;
                output.run = Some(self.submit(inputs.streams, inputs.request, true));
                repaint_after = Some(RESULT_FLUSH_INTERVAL);
            } else {
                repaint_after =
                    Some(repaint_after.map_or(deadline - now, |after| after.min(deadline - now)));
            }
        }
        if let Some(after) = repaint_after {
            ui.ctx().request_repaint_after_secs(after.max(0.0) as f32);
        }
        Ok(output)
    }

    /// The Replace All confirmation (`search-panel.tsx` 268-281): Cancel takes the first focus,
    /// Escape cancels, and confirming hands back the replace of the checked files.
    fn show_confirm(
        &mut self,
        ui: &Ui,
        id: Id,
        focus_cancel: bool,
        locale: &ResolvedLocale,
    ) -> Option<Replace> {
        let message = |key: &str| crate::presentation::message(locale, key, &[]);
        let selected = self
            .results
            .groups()
            .iter()
            .filter(|group| !self.excluded.contains(&group.path))
            .collect::<Vec<_>>();
        let files = selected.len().to_string();
        let matches = selected
            .iter()
            .map(|group| group.matches.len())
            .sum::<usize>()
            .to_string();
        let paths = selected
            .iter()
            .map(|group| group.path.clone())
            .collect::<Vec<_>>();
        let response = egui::Modal::new(id.with("replace-confirm")).show(ui.ctx(), |ui| {
            ui.set_max_width(CONFIRM_MAX_WIDTH);
            ui.heading(message("search.replaceConfirmTitle"));
            ui.label(crate::presentation::message(
                locale,
                "search.replaceConfirmDescription",
                &[("files", &files), ("matches", &matches)],
            ));
            ui.horizontal(|ui| {
                let cancel = ui.button(message("common.cancel"));
                if focus_cancel {
                    cancel.request_focus();
                }
                let confirm = ui.button(message("common.confirm"));
                (cancel.clicked(), confirm.clicked())
            })
            .inner
        });
        self.confirm = Some(false);
        let (is_cancelled, is_confirmed) = response.inner;
        let is_escaped = response.is_top_modal
            && !response.any_popup_open
            && ui
                .ctx()
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if is_confirmed {
            self.confirm = None;
            let query = self.ran_query.clone()?;
            self.is_replacing = true;
            return Some(Replace {
                query,
                replacement: self.replace_text.clone(),
                paths: (!paths.is_empty()).then_some(paths),
            });
        }
        if is_cancelled || is_escaped {
            self.confirm = None;
        }
        None
    }
}

#[cfg(test)]
impl Panel {
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn status(&self) -> Status {
        self.status
    }

    pub(crate) fn context_lines(&self) -> u32 {
        self.context_lines
    }

    /// Starts a run whose host work a test never submits, so it stays running.
    pub(crate) fn begin_unsubmitted_run(&mut self, streams: &Streams) -> Run {
        self.submit(streams, None, false)
    }
}

#[cfg(test)]
#[path = "search-panel-tests.rs"]
mod tests;
