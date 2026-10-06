use std::sync::Arc;
use std::time::Duration;

pub use web_time::Instant;

use egui::{self, Color32, ColorImage, FontId, Id, Rect, Stroke, TextureHandle};
use resvg::{tiny_skia, usvg};
use taide_model::{
    error::{AppError, AppResult},
    locale::ResolvedLocale,
    theme::ThemeType,
};

#[path = "toast-motion.rs"]
mod motion;

#[path = "toast-swipe.rs"]
mod swipe;

const LIFETIME: Duration = Duration::from_millis(4000);
const EXIT_DURATION: Duration = Duration::from_millis(200);
const WIDTH: f32 = 356.0;
const GAP: f32 = 14.0;
const VISIBLE: usize = 3;
const VIEWPORT_OFFSET: f32 = 24.0;
const MOBILE_OFFSET: f32 = 16.0;
const MOBILE_WIDTH: f32 = 600.0;
const PADDING: f32 = 16.0;
const BORDER: f32 = 1.0;
const RADIUS: u8 = 8;
const FONT: f32 = 13.0;
const TITLE_LINE_HEIGHT: f32 = 19.5;
const DESCRIPTION_LINE_HEIGHT: f32 = 18.2;
const DESCRIPTION_GAP: f32 = 2.0;
const ICON_SIZE: f32 = 20.0;
const ICON_WRAPPER: f32 = 16.0;
const ICON_LEFT_MARGIN: f32 = -3.0;
const ICON_SVG_MARGIN: f32 = -1.0;
const ICON_RIGHT_MARGIN: f32 = 4.0;
const CONTENT_GAP: f32 = 6.0;
const MAX_RASTER_SIDE: f32 = 1024.0;
const CLOSE_SIZE: f32 = 20.0;
const CLOSE_SHIFT: f32 = 7.0;
const CLOSE_GLYPH_INSET: f32 = 7.0;
const CLOSE_GLYPH_STROKE: f32 = 0.75;
const CLOSE_LABEL: &str = "Close toast";
const LIST_ID: &str = "native-notification-list";
const LIST_LABEL: &str = "Notifications altKey+T";
const FOCUS_RING: f32 = 2.0;
const FOCUS_ALPHA: u8 = 51;
const SHADOW_ALPHA: u8 = 26;
const SHADOW_OFFSET: [i8; 2] = [0, 4];
const SHADOW_BLUR: u8 = 12;
const LIGHT_WARNING: [[u8; 3]; 3] = [[255, 252, 240], [251, 238, 177], [220, 118, 9]];
const LIGHT_ERROR: [[u8; 3]; 3] = [[255, 240, 240], [255, 224, 225], [230, 0, 0]];
const DARK_WARNING: [[u8; 3]; 3] = [[29, 31, 0], [46, 46, 0], [243, 207, 88]];
const DARK_ERROR: [[u8; 3]; 3] = [[45, 6, 7], [77, 4, 8], [255, 158, 161]];
const LIGHT_SUCCESS: [[u8; 3]; 3] = [[236, 253, 243], [211, 251, 223], [0, 138, 46]];
const DARK_SUCCESS: [[u8; 3]; 3] = [[0, 31, 15], [0, 61, 28], [89, 243, 166]];
const LIGHT_CLOSE_HOVER: [[u8; 3]; 2] = [[248, 248, 248], [232, 232, 232]];
const DARK_CLOSE_HOVER: [[u8; 3]; 2] = [[31, 31, 31], [64, 64, 64]];
const LIGHT_INFO: [[u8; 3]; 3] = [[240, 248, 255], [221, 231, 253], [9, 115, 220]];
const DARK_INFO: [[u8; 3]; 3] = [[0, 13, 31], [25, 35, 62], [88, 150, 243]];
const LIGHT_ACTION: [[u8; 3]; 2] = [[23, 23, 23], [255, 255, 255]];
const DARK_ACTION: [[u8; 3]; 2] = [[252, 252, 252], [0, 0, 0]];
const ACTION_HEIGHT: f32 = 24.0;
const ACTION_PADDING: f32 = 8.0;
const ACTION_FONT: f32 = 12.0;
const ACTION_RADIUS: u8 = 4;
const ACTION_FOCUS_RING: f32 = 2.0;
const ACTION_FOCUS_ALPHA: u8 = 102;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Warning,
    Error,
    Success,
    Info,
}

impl Kind {
    fn index(self) -> usize {
        match self {
            Self::Warning => 0,
            Self::Error => 1,
            Self::Success => 2,
            Self::Info => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoAction {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action<A> {
    pub label: String,
    pub id: A,
}

pub struct Options<A> {
    pub description: Option<String>,
    pub action: Option<Action<A>>,
}

impl<A> Default for Options<A> {
    fn default() -> Self {
        Self {
            description: None,
            action: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Vertical {
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Horizontal {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Position {
    vertical: Vertical,
    horizontal: Horizontal,
}

impl Position {
    fn parent_key(self) -> (bool, Horizontal) {
        (self.vertical != Vertical::Bottom, self.horizontal)
    }

    fn parse(value: &str) -> Self {
        let mut parts = value.split('-');
        let vertical = match parts.next() {
            Some("top") => Some(Vertical::Top),
            Some("middle") => Some(Vertical::Middle),
            Some("bottom") => Some(Vertical::Bottom),
            _ => None,
        };
        let horizontal = match parts.next() {
            Some("left") => Some(Horizontal::Left),
            Some("center") => Some(Horizontal::Center),
            Some("right") => Some(Horizontal::Right),
            _ => None,
        };
        match (vertical, horizontal) {
            (Some(vertical), Some(horizontal)) => Self {
                vertical,
                horizontal,
            },
            _ => Self {
                vertical: Vertical::Bottom,
                horizontal: Horizontal::Right,
            },
        }
    }

    fn geometry(&self, screen: Rect, height: f32, offset: f32) -> Rect {
        let mobile = screen.width() <= MOBILE_WIDTH;
        let width = if mobile {
            (screen.width() - MOBILE_OFFSET * 2.0).max(0.0)
        } else {
            WIDTH
        };
        let margin = if mobile {
            MOBILE_OFFSET
        } else {
            VIEWPORT_OFFSET
        };
        let x = match (mobile, self.horizontal, self.vertical) {
            (true, _, _) => screen.left() + margin,
            (false, Horizontal::Left, _) => screen.left() + margin,
            (false, Horizontal::Right, _) => screen.right() - margin - width,
            (false, Horizontal::Center, Vertical::Middle) => screen.center().x,
            (false, Horizontal::Center, _) => screen.center().x - width * 0.5,
        };
        let y = match self.vertical {
            Vertical::Top => screen.top() + margin + offset,
            Vertical::Middle => screen.center().y + offset,
            Vertical::Bottom => screen.bottom() - margin - height - offset,
        };
        Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height))
    }
}

struct Toast<A> {
    id: u64,
    kind: Kind,
    title: String,
    description: Option<String>,
    action: Option<Action<A>>,
    remaining: Duration,
    running_since: Option<Instant>,
    dismissed_at: Option<Instant>,
    scheduled_removal_at: Option<Instant>,
    motion: Option<motion::Motion>,
    swipe_exit: Option<swipe::Exit>,
}

impl<A> Toast<A> {
    fn dismiss(&mut self, now: Instant) {
        self.dismissed_at = Some(now);
        self.scheduled_removal_at.get_or_insert(now);
        self.running_since = None;
        if let Some(motion) = &mut self.motion {
            motion.dismiss(now);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Mount(u64);

impl Mount {
    fn list(self) -> Id {
        Id::new((LIST_ID, self.0))
    }

    fn card(self, toast: u64) -> Id {
        Id::new(("native-toast", self.0, toast))
    }

    fn close(self, toast: u64) -> Id {
        Id::new(("native-toast-close", self.0, toast))
    }

    fn action(self, toast: u64) -> Id {
        Id::new(("native-toast-action", self.0, toast))
    }
}

struct SwipeOwner {
    toast: u64,
    button: egui::PointerButton,
    capture: swipe::Capture,
}

struct HitTarget {
    toast: u64,
    card: Rect,
    close: Rect,
    action: Option<Rect>,
}

struct Assets {
    trees: Vec<usvg::Tree>,
    textures: Vec<TextureHandle>,
    scale: Option<f32>,
}

impl Assets {
    fn new() -> AppResult<Self> {
        let options = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_string: Box::new(|_, _| None),
                resolve_data: Box::new(|_, _, _| None),
            },
            ..Default::default()
        };
        let trees = [
            include_bytes!("../../taide-native-app/resources/toasts/warning.svg").as_slice(),
            include_bytes!("../../taide-native-app/resources/toasts/error.svg").as_slice(),
            include_bytes!("../../taide-native-app/resources/toasts/success.svg").as_slice(),
            include_bytes!("../../taide-native-app/resources/toasts/info.svg").as_slice(),
        ]
        .into_iter()
        .map(|source| {
            usvg::Tree::from_data(source, &options)
                .map_err(|error| AppError::Internal(format!("native toast icon: {error}")))
        })
        .collect::<AppResult<Vec<_>>>()?;
        Ok(Self {
            trees,
            textures: Vec::new(),
            scale: None,
        })
    }

    fn prepare(&mut self, context: &egui::Context) -> AppResult<()> {
        let scale = context.pixels_per_point();
        if self.scale == Some(scale) {
            return Ok(());
        }
        let pixels = (ICON_SIZE * scale).ceil();
        if !pixels.is_finite() || !(1.0..=MAX_RASTER_SIDE).contains(&pixels) {
            return Err(AppError::Internal(
                "native toast icon scale is out of range".into(),
            ));
        }
        let side = pixels as u32;
        let mut textures = Vec::new();
        for (index, tree) in self.trees.iter().enumerate() {
            let mut pixmap = tiny_skia::Pixmap::new(side, side)
                .ok_or_else(|| AppError::Internal("native toast icon allocation failed".into()))?;
            resvg::render(
                tree,
                tiny_skia::Transform::from_scale(
                    ICON_SIZE * scale / tree.size().width(),
                    ICON_SIZE * scale / tree.size().height(),
                ),
                &mut pixmap.as_mut(),
            );
            textures.push(context.load_texture(
                format!("native-toast-icon-{index}"),
                ColorImage::from_rgba_unmultiplied(
                    [side as usize, side as usize],
                    &pixmap.take_demultiplied(),
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        self.textures = textures;
        self.scale = Some(scale);
        Ok(())
    }
}

pub struct Toasts<A = NoAction> {
    entries: Vec<Toast<A>>,
    actions: Vec<A>,
    next_id: u64,
    paused: bool,
    hovered: bool,
    interacting: bool,
    expanded: bool,
    focus_within: bool,
    previous_focus: Option<Id>,
    last_owned_focus: Option<Id>,
    event_frame: Option<u64>,
    assets: Assets,
    bounds: Vec<Rect>,
    hit_targets: Vec<HitTarget>,
    swipe_owner: Option<SwipeOwner>,
    pointer_frame: Option<u64>,
    is_reduced_motion: bool,
    position: Option<Position>,
    mount: Mount,
}

#[cfg(any(test, feature = "inspection"))]
pub struct Inspection {
    pub kind: Kind,
    pub title: String,
    pub description: Option<String>,
    pub action_label: Option<String>,
    pub is_dismissed: bool,
    pub close: Option<Rect>,
    pub action: Option<Rect>,
}

impl Toasts {
    pub fn new() -> AppResult<Self> {
        Self::with_actions()
    }
}

impl<A: Clone> Toasts<A> {
    #[cfg(any(test, feature = "inspection"))]
    pub fn inspection(&self) -> Vec<Inspection> {
        self.entries
            .iter()
            .map(|toast| Inspection {
                kind: toast.kind,
                title: toast.title.clone(),
                description: toast.description.clone(),
                action_label: toast.action.as_ref().map(|action| action.label.clone()),
                is_dismissed: toast.dismissed_at.is_some(),
                close: self
                    .hit_targets
                    .iter()
                    .find(|target| target.toast == toast.id)
                    .map(|target| target.close),
                action: self
                    .hit_targets
                    .iter()
                    .find(|target| target.toast == toast.id)
                    .and_then(|target| target.action),
            })
            .collect()
    }

    #[cfg(any(test, feature = "inspection"))]
    pub fn inspection_reduced_motion(&self) -> bool {
        self.is_reduced_motion
    }

    pub fn with_actions() -> AppResult<Self> {
        Ok(Self {
            entries: Vec::new(),
            actions: Vec::new(),
            next_id: 0,
            paused: false,
            hovered: false,
            interacting: false,
            expanded: false,
            focus_within: false,
            previous_focus: None,
            last_owned_focus: None,
            event_frame: None,
            assets: Assets::new()?,
            bounds: Vec::new(),
            hit_targets: Vec::new(),
            swipe_owner: None,
            pointer_frame: None,
            is_reduced_motion: false,
            position: None,
            mount: Mount::default(),
        })
    }

    pub fn set_reduced_motion(&mut self, is_reduced_motion: bool) {
        self.is_reduced_motion = is_reduced_motion;
    }

    pub fn set_position(
        &mut self,
        context: &egui::Context,
        position: &str,
        now: Instant,
        enabled: bool,
    ) {
        let position = Position::parse(position);
        let previous = self.position.replace(position);
        if previous.is_none_or(|previous| previous == position) {
            return;
        }
        self.bounds.clear();
        self.hit_targets.clear();
        if previous.is_some_and(|previous| previous.parent_key() == position.parent_key()) {
            return;
        }
        let focused = context.memory(|memory| memory.focused());
        if self.owns_focus(focused) {
            context.memory_mut(|memory| {
                if let Some(focused) = focused {
                    memory.surrender_focus(focused);
                }
                if enabled
                    && memory.top_modal_layer().is_none()
                    && let Some(previous) = self.previous_focus
                {
                    memory.request_focus(previous);
                }
            });
        }
        self.focus_within = false;
        self.last_owned_focus = None;
        self.mount.0 = self
            .mount
            .0
            .checked_add(1)
            .expect("native toast mount space exhausted");
        self.swipe_owner = None;
        self.hovered = false;
        self.paused = self.expanded
            || self.interacting
            || context.input(|input| input.viewport().minimized.unwrap_or(false));
        for toast in &mut self.entries {
            toast.remaining = LIFETIME;
            toast.running_since = (!self.paused).then_some(now);
            toast.dismissed_at = None;
            toast.motion = None;
            toast.swipe_exit = None;
        }
        context.request_repaint();
    }

    fn push(&mut self, kind: Kind, title: String, description: Option<String>, now: Instant) {
        self.notify(
            kind,
            title,
            Options {
                description,
                action: None,
            },
            now,
        );
    }

    pub fn notify(&mut self, kind: Kind, title: String, options: Options<A>, now: Instant) {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("native toast identity space exhausted");
        self.entries.insert(
            0,
            Toast {
                id,
                kind,
                title,
                description: options.description,
                action: options.action,
                remaining: LIFETIME,
                running_since: (!self.paused).then_some(now),
                dismissed_at: None,
                scheduled_removal_at: None,
                motion: None,
                swipe_exit: None,
            },
        );
        if self.entries.len() == 1 {
            self.expanded = false;
        }
    }

    pub fn warning(&mut self, title: String, now: Instant) {
        self.push(Kind::Warning, title, None, now);
    }

    pub fn success(&mut self, title: String, now: Instant) {
        self.push(Kind::Success, title, None, now);
    }

    pub fn error(&mut self, title: String, now: Instant) {
        self.push(Kind::Error, title, None, now);
    }

    pub fn info(&mut self, title: String, now: Instant) {
        self.push(Kind::Info, title, None, now);
    }

    pub fn is_showing(&self, kind: Kind, title: &str) -> bool {
        self.entries
            .iter()
            .any(|toast| toast.dismissed_at.is_none() && toast.kind == kind && toast.title == title)
    }

    pub fn take_actions(&mut self) -> Vec<A> {
        std::mem::take(&mut self.actions)
    }

    pub fn snippet(
        &mut self,
        locale: &ResolvedLocale,
        notice: crate::snippet_editor::Notice,
        now: Instant,
    ) {
        use crate::presentation::message;
        use crate::snippet_editor::Notice;
        let title = match notice {
            Notice::Saved => {
                self.push(
                    Kind::Success,
                    message(locale, "snippetEditor.saveSuccess", &[]),
                    None,
                    now,
                );
                return;
            }
            Notice::Incomplete(count) => message(
                locale,
                "snippetEditor.incompleteEntryError",
                &[("count", &count.to_string())],
            ),
            Notice::DuplicateNames => message(locale, "snippetEditor.duplicateNameError", &[]),
            Notice::SaveFailed { create, error } => {
                let key = match (create, error) {
                    (true, AppError::InvalidArgument(_)) => "snippetEditor.invalidFileName",
                    (false, AppError::InvalidArgument(_)) => "snippetEditor.parseError",
                    _ => "snippetEditor.saveFailed",
                };
                message(locale, key, &[])
            }
            Notice::DeleteFailed(error) => describe_error(locale, &error),
        };
        self.push(Kind::Error, title, None, now);
    }

    pub fn settings_failed(&mut self, locale: &ResolvedLocale, error: &AppError, now: Instant) {
        self.push(
            Kind::Error,
            crate::presentation::message(locale, "settings.saveFailed", &[]),
            Some(describe_error(locale, error)),
            now,
        );
    }

    pub fn ipc_error(&mut self, locale: &ResolvedLocale, error: &AppError, now: Instant) {
        self.push(Kind::Error, describe_error(locale, error), None, now);
    }

    pub fn app_file_failed(
        &mut self,
        locale: &ResolvedLocale,
        target: taide_model::app::AppFileTarget,
        error: &AppError,
        now: Instant,
    ) {
        if target == taide_model::app::AppFileTarget::Settings {
            self.push(
                Kind::Error,
                crate::presentation::message(locale, "settings.settingsJsonInvalid", &[]),
                None,
                now,
            );
            return;
        }
        self.ipc_error(locale, error, now);
    }

    fn update(&mut self, now: Instant, paused: bool) {
        let count = self.entries.len();
        for toast in &mut self.entries {
            if toast.dismissed_at.is_some() {
                continue;
            }
            if let Some(started) = toast.running_since.take() {
                toast.remaining = toast
                    .remaining
                    .saturating_sub(now.saturating_duration_since(started));
            }
            if toast.remaining.is_zero() {
                toast.dismiss(now);
            } else if !paused {
                toast.running_since = Some(now);
            }
        }
        self.entries.retain(|toast| {
            toast
                .scheduled_removal_at
                .is_none_or(|at| now.saturating_duration_since(at) < EXIT_DURATION)
        });
        self.paused = paused;
        if count != self.entries.len() && self.entries.len() <= 1 {
            self.expanded = false;
        }
        if self.entries.is_empty() {
            self.bounds.clear();
            self.hit_targets.clear();
            self.swipe_owner = None;
            self.hovered = false;
            self.interacting = false;
        }
    }

    fn update_hover(&mut self, pointer: Option<egui::Pos2>, moved: bool, enabled: bool) {
        let hovered = enabled
            && pointer.is_some_and(|point| self.bounds.iter().any(|rect| rect.contains(point)));
        if hovered && (!self.hovered || moved) {
            self.expanded = true;
        } else if !hovered && self.hovered && !self.interacting {
            self.expanded = false;
        }
        self.hovered = hovered;
    }

    fn owns_focus(&self, focused: Option<Id>) -> bool {
        focused.is_some_and(|focused| {
            focused == self.mount.list()
                || self.entries.iter().any(|toast| {
                    focused == self.mount.card(toast.id)
                        || focused == self.mount.close(toast.id)
                        || focused == self.mount.action(toast.id)
                })
        })
    }

    fn sync_focus(&mut self, context: &egui::Context, enabled: bool) {
        let focused = context.memory(|memory| memory.focused());
        if enabled && !self.entries.is_empty() && self.owns_focus(focused) {
            self.focus_within = true;
            self.last_owned_focus = focused;
            return;
        }
        if self.focus_within {
            if enabled && let Some(previous) = self.previous_focus.take() {
                context.memory_mut(|memory| memory.request_focus(previous));
            }
            self.focus_within = false;
            self.last_owned_focus = None;
        }
        self.previous_focus = if enabled {
            context.memory(|memory| memory.focused())
        } else {
            None
        };
    }

    pub fn tick(&mut self, context: &egui::Context, now: Instant, enabled: bool) {
        let enabled = enabled && context.memory(|memory| memory.top_modal_layer().is_none());
        let (hidden, pointer, focused) = context.input(|input| {
            (
                input.viewport().minimized.unwrap_or(false),
                input.pointer.hover_pos(),
                input.raw.focused,
            )
        });
        if !enabled || !focused {
            self.interacting = false;
        }
        let event_frame = context.cumulative_frame_nr();
        if !self.entries.is_empty() && self.event_frame != Some(event_frame) {
            self.event_frame = Some(event_frame);
            let events = context.input(|input| input.raw.events.clone());
            if !events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::PointerMoved(_) | egui::Event::PointerGone
                )
            }) {
                self.update_hover(pointer, false, enabled);
            }
            for event in events {
                match &event {
                    egui::Event::PointerMoved(pos) => {
                        self.update_hover(Some(*pos), true, enabled);
                    }
                    egui::Event::PointerGone => self.update_hover(None, false, enabled),
                    egui::Event::PointerButton {
                        pos, pressed: true, ..
                    } if enabled && self.bounds.iter().any(|rect| rect.contains(*pos)) => {
                        self.interacting = true;
                    }
                    egui::Event::PointerButton { pressed: false, .. } => {
                        self.interacting = false;
                    }
                    _ => {}
                }
                if enabled
                    && let egui::Event::AccessKitActionRequest(request) = &event
                    && request.action == egui::accesskit::Action::Focus
                    && request.target_tree == egui::accesskit::TreeId::ROOT
                    && request.target_node == self.mount.list().accesskit_id()
                {
                    self.sync_focus(context, enabled);
                    context.memory_mut(|memory| memory.request_focus(self.mount.list()));
                    self.focus_within = true;
                    self.last_owned_focus = Some(self.mount.list());
                }
                let egui::Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                else {
                    continue;
                };
                let physical_key = physical_key.unwrap_or(key);
                if modifiers.alt && physical_key == egui::Key::T {
                    self.expanded = true;
                    if enabled {
                        self.sync_focus(context, enabled);
                        context.memory_mut(|memory| memory.request_focus(self.mount.list()));
                        self.focus_within = true;
                        self.last_owned_focus = Some(self.mount.list());
                    }
                }
                if physical_key == egui::Key::Escape && enabled && self.focus_within {
                    self.expanded = false;
                    if context.memory(|memory| memory.focused().is_none())
                        && let Some(focused) = self.last_owned_focus
                    {
                        context.memory_mut(|memory| memory.request_focus(focused));
                    }
                }
                if physical_key == egui::Key::Tab
                    && enabled
                    && !modifiers.alt
                    && !modifiers.ctrl
                    && !modifiers.command
                    && !modifiers.mac_cmd
                    && context.memory(|memory| memory.focused() == Some(self.mount.list()))
                {
                    let target = if modifiers.shift {
                        self.previous_focus
                    } else {
                        self.entries.first().map(|toast| self.mount.card(toast.id))
                    };
                    context.memory_mut(|memory| {
                        memory.move_focus(egui::FocusDirection::None);
                        if let Some(target) = target {
                            memory.request_focus(target);
                        }
                    });
                    context.input_mut(|input| {
                        input.events.retain(|event| {
                            !matches!(
                                event,
                                egui::Event::Key {
                                    key: egui::Key::Tab,
                                    pressed: true,
                                    ..
                                }
                            )
                        })
                    });
                }
            }
        }
        self.update(now, hidden || self.expanded || self.interacting);
        self.sync_focus(context, enabled);
        if let Some(delay) = self
            .entries
            .iter()
            .filter_map(|toast| {
                toast
                    .scheduled_removal_at
                    .map(|at| EXIT_DURATION.saturating_sub(now.saturating_duration_since(at)))
                    .or_else(|| toast.running_since.map(|_| toast.remaining))
            })
            .min()
        {
            context.request_repaint_after(delay);
        }
    }

    pub fn show(
        &mut self,
        context: &egui::Context,
        theme: ThemeType,
        position: &str,
        now: Instant,
        enabled: bool,
    ) -> AppResult<()> {
        self.set_position(context, position, now, enabled);
        if self.entries.is_empty() {
            return Ok(());
        }
        self.assets.prepare(context)?;
        let position = Position::parse(position);
        let screen = context.content_rect();
        let width = position.geometry(screen, 0.0, 0.0).width();
        let icon_space = ICON_WRAPPER + ICON_LEFT_MARGIN + ICON_RIGHT_MARGIN + CONTENT_GAP;
        let text_width = (width - (PADDING + BORDER) * 2.0 - icon_space).max(0.0);
        let enabled = enabled && context.memory(|memory| memory.top_modal_layer().is_none());
        self.process_swipes(context, &position, now, enabled);
        let mount = self.mount;
        let expanded = self.expanded && self.entries.len() > 1;
        let swipe_owner = self.swipe_owner.as_ref();
        let cards = self
            .entries
            .iter_mut()
            .map(|toast| {
                let (background, border, foreground) = colors(theme, toast.kind);
                let action = toast.action.as_ref().map(|action| {
                    context.fonts_mut(|fonts| {
                        fonts.layout_no_wrap(
                            action.label.clone(),
                            FontId::proportional(ACTION_FONT),
                            action_colors(theme).1,
                        )
                    })
                });
                let text_width = action.as_ref().map_or(text_width, |label| {
                    (text_width - CONTENT_GAP - label.size().x - ACTION_PADDING * 2.0).max(0.0)
                });
                let title = layout(
                    context,
                    toast.title.clone(),
                    foreground,
                    text_width,
                    TITLE_LINE_HEIGHT,
                );
                let description = toast.description.as_ref().map(|text| {
                    layout(
                        context,
                        text.clone(),
                        foreground,
                        text_width,
                        DESCRIPTION_LINE_HEIGHT,
                    )
                });
                let content_height = title.size().y
                    + description
                        .as_ref()
                        .map_or(0.0, |galley| DESCRIPTION_GAP + galley.size().y);
                let control_height = if action.is_some() {
                    ACTION_HEIGHT
                } else {
                    ICON_WRAPPER
                };
                let height = content_height.max(control_height) + (PADDING + BORDER) * 2.0;
                (
                    toast,
                    background,
                    border,
                    foreground,
                    title,
                    description,
                    height,
                    action,
                )
            })
            .collect::<Vec<_>>();
        let front_height = cards
            .iter()
            .find(|card| card.0.dismissed_at.is_none())
            .map_or(cards[0].6, |card| card.6);
        let mut offset = 0.0;
        let mut plans = Vec::new();
        for (index, card) in cards.into_iter().enumerate() {
            let style = motion::Style {
                index,
                height: card.6,
                front_height,
                offset,
                gap: GAP,
                top: position.vertical != Vertical::Bottom,
                expanded,
                visible: index < VISIBLE,
                removed: card.0.dismissed_at.is_some(),
                swiping: card.0.swipe_exit.is_some()
                    || swipe_owner.is_some_and(|owner| owner.toast == card.0.id),
                swipe_out: card.0.swipe_exit.is_some(),
                is_reduced_motion: self.is_reduced_motion,
            };
            let mut sample = card
                .0
                .motion
                .get_or_insert_with(|| motion::Motion::new(&style, now))
                .sample(&style, now);
            let amount = if let Some(exit) = &card.0.swipe_exit {
                let (amount, opacity) =
                    exit.sample(now, width, sample.height, self.is_reduced_motion);
                sample.opacity = opacity;
                sample.active |= !self.is_reduced_motion
                    && now.saturating_duration_since(
                        card.0.dismissed_at.expect("swipe exit has a removal time"),
                    ) < EXIT_DURATION;
                amount
            } else {
                swipe_owner
                    .filter(|owner| owner.toast == card.0.id)
                    .map_or(egui::Vec2::ZERO, |owner| owner.capture.amount)
            };
            if sample.active {
                context.request_repaint();
            }
            let rect = position.geometry(screen, sample.height, 0.0);
            let transform = egui::emath::TSTransform::new(
                rect.center().to_vec2() * (1.0 - sample.scale)
                    + egui::vec2(sample.translate_x, sample.translate)
                    + amount * sample.scale,
                sample.scale,
            );
            let visual_rect = transformed_rect(transform, rect);
            if card.0.dismissed_at.is_none() {
                offset += card.6 + GAP;
            }
            plans.push((index, rect, visual_rect, transform, sample, card));
        }
        let mut dismissed = Vec::new();
        let mut triggered = Vec::new();
        self.hit_targets = plans
            .iter()
            .filter(|(index, _, _, _, _, card)| *index < VISIBLE && card.0.dismissed_at.is_none())
            .map(|(index, rect, visual_rect, transform, _, card)| HitTarget {
                toast: card.0.id,
                card: *visual_rect,
                close: transformed_rect(
                    *transform,
                    Rect::from_min_size(
                        rect.min - egui::Vec2::splat(CLOSE_SHIFT),
                        egui::Vec2::splat(CLOSE_SIZE),
                    ),
                ),
                action: card
                    .7
                    .as_ref()
                    .filter(|_| *index == 0 || expanded)
                    .map(|label| transformed_rect(*transform, action_rect(*rect, label))),
            })
            .collect();
        self.bounds = plans
            .iter()
            .rev()
            .filter(|(index, _, _, _, _, _)| *index < VISIBLE)
            .map(|(_, rect, visual_rect, transform, _, _)| {
                let close_rect = Rect::from_min_size(
                    rect.min - egui::Vec2::splat(CLOSE_SHIFT),
                    egui::Vec2::splat(CLOSE_SIZE),
                );
                let mut bounds = visual_rect.union(transformed_rect(*transform, close_rect));
                if expanded {
                    bounds.min.y = bounds.min.y.min(visual_rect.top() - GAP - BORDER);
                }
                bounds
            })
            .collect();
        let bounds = self
            .bounds
            .iter()
            .copied()
            .reduce(Rect::union)
            .expect("native toast list has visible entries");
        egui::Area::new(Id::new("native-notification-area"))
            .fixed_pos(bounds.min)
            .order(egui::Order::Tooltip)
            .constrain(false)
            .fade_in(false)
            .show(context, |ui| {
                if self.is_reduced_motion && ui.is_sizing_pass() {
                    context.request_discard("native reduced-motion toast sizing");
                }
                ui.allocate_exact_size(bounds.size(), egui::Sense::hover());
                if !enabled {
                    ui.disable();
                    ui.set_opacity(1.0);
                }
                let list_id = mount.list();
                let list_focused =
                    enabled && context.memory(|memory| memory.focused() == Some(list_id));
                context.check_for_id_clash(list_id, bounds, LIST_LABEL);
                ui.interact(bounds, list_id, egui::Sense::hover());
                if list_focused {
                    context.memory_mut(|memory| memory.request_focus(list_id));
                }
                context.accesskit_node_builder(list_id, |node| {
                    node.set_role(egui::accesskit::Role::List);
                    node.set_label(LIST_LABEL);
                    node.set_live(egui::accesskit::Live::Polite);
                    node.clear_live_atomic();
                    node.add_action(egui::accesskit::Action::Focus);
                    if !enabled {
                        node.set_hidden();
                    }
                });
                context.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        list_id,
                        egui::EventFilter {
                            escape: true,
                            ..Default::default()
                        },
                    )
                });
                for (index, rect, _, transform, sample, (_, background, border, _, _, _, _, _)) in
                    plans.iter().rev()
                {
                    if *index >= VISIBLE {
                        continue;
                    }
                    let opacity = sample.opacity;
                    let start = shape_index(ui);
                    let painter = ui.painter().with_clip_rect(Rect::EVERYTHING);
                    painter.add(
                        egui::epaint::Shadow {
                            offset: SHADOW_OFFSET,
                            blur: SHADOW_BLUR,
                            spread: 0,
                            color: Color32::from_black_alpha(SHADOW_ALPHA).gamma_multiply(opacity),
                        }
                        .as_shape(*rect, RADIUS),
                    );
                    painter.rect(
                        *rect,
                        RADIUS,
                        background.gamma_multiply(opacity),
                        Stroke::new(BORDER, border.gamma_multiply(opacity)),
                        egui::StrokeKind::Inside,
                    );
                    transform_shapes(ui, *transform, start);
                }
                for (
                    index,
                    rect,
                    visual_rect,
                    transform,
                    sample,
                    (toast, background, border, foreground, title, description, _, action_label),
                ) in plans
                {
                    let visible = index < VISIBLE;
                    let start = shape_index(ui);
                    let card_id = mount.card(toast.id);
                    let mut card_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(("native-toast-content", mount.0, toast.id))
                            .accessibility_parent(list_id)
                            .max_rect(rect),
                    );
                    if toast.dismissed_at.is_some() {
                        card_ui.disable();
                    }
                    if !visible {
                        card_ui.set_clip_rect(Rect::NOTHING);
                    } else {
                        card_ui.set_clip_rect(Rect::EVERYTHING);
                    }
                    card_ui.set_opacity(sample.opacity);
                    let card =
                        card_ui.interact(visual_rect, card_id, egui::Sense::click_and_drag());
                    card.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Label,
                            card_ui.is_enabled(),
                            &toast.title,
                        )
                    });
                    context.accesskit_node_builder(card_id, |node| {
                        node.set_role(egui::accesskit::Role::ListItem);
                        node.set_label(toast.title.as_str());
                        node.clear_value();
                        if let Some(description) = &toast.description {
                            node.set_description(description.as_str());
                        }
                    });
                    context.memory_mut(|memory| {
                        memory.set_focus_lock_filter(
                            card_id,
                            egui::EventFilter {
                                escape: true,
                                ..Default::default()
                            },
                        )
                    });
                    let (focus_shadow, is_shadow_active) = toast
                        .motion
                        .as_mut()
                        .expect("rendered toast has motion")
                        .focus_shadow(card.has_focus(), now);
                    if is_shadow_active {
                        context.request_repaint();
                    }
                    if visible && focus_shadow > 0.0 {
                        let spread = FOCUS_RING * focus_shadow;
                        card_ui.painter().rect_stroke(
                            rect.expand(spread),
                            (f32::from(RADIUS) + spread).round() as u8,
                            Stroke::new(
                                spread,
                                Color32::from_black_alpha(
                                    (f32::from(FOCUS_ALPHA) * focus_shadow).round() as u8,
                                ),
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if visible && sample.content_opacity > 0.0 {
                        let mut painter = card_ui.painter().clone();
                        painter.multiply_opacity(sample.content_opacity);
                        let body = rect.shrink(PADDING + BORDER);
                        let text_x = body.left() + icon_space;
                        let text_height = title.size().y
                            + description
                                .as_ref()
                                .map_or(0.0, |galley| DESCRIPTION_GAP + galley.size().y);
                        let text_y = body.center().y - text_height * 0.5;
                        painter.galley(egui::pos2(text_x, text_y), title.clone(), foreground);
                        if let Some(description) = &description {
                            painter.galley(
                                egui::pos2(text_x, text_y + title.size().y + DESCRIPTION_GAP),
                                description.clone(),
                                foreground,
                            );
                        }
                        let icon = Rect::from_min_size(
                            egui::pos2(
                                body.left() + ICON_LEFT_MARGIN + ICON_SVG_MARGIN,
                                body.center().y - ICON_SIZE * 0.5,
                            ),
                            egui::Vec2::splat(ICON_SIZE),
                        );
                        if let Some(texture) = self.assets.textures.get(toast.kind.index()) {
                            let mut icon_ui =
                                card_ui.new_child(egui::UiBuilder::new().max_rect(icon));
                            icon_ui.set_opacity(sample.opacity * sample.content_opacity);
                            egui::Image::new((texture.id(), egui::Vec2::splat(ICON_SIZE)))
                                .tint(foreground)
                                .paint_at(&icon_ui, icon);
                        }
                    }
                    let close_rect = Rect::from_min_size(
                        rect.min - egui::Vec2::splat(CLOSE_SHIFT),
                        egui::Vec2::splat(CLOSE_SIZE),
                    );
                    let close_id = mount.close(toast.id);
                    let mut controls_ui = card_ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("native-toast-controls")
                            .accessibility_parent(card_id)
                            .max_rect(close_rect),
                    );
                    controls_ui.set_opacity(sample.opacity * sample.close_opacity);
                    let close = controls_ui.interact(
                        transformed_rect(transform, close_rect),
                        close_id,
                        egui::Sense::click(),
                    );
                    let is_close_hovered = close.hovered()
                        && context.input(|input| {
                            input.pointer.hover_pos().is_some_and(|pointer| {
                                transformed_rect(transform, close_rect).contains(pointer)
                            })
                        });
                    let targets = if is_close_hovered {
                        match theme {
                            ThemeType::Light => LIGHT_CLOSE_HOVER,
                            ThemeType::Dark => DARK_CLOSE_HOVER,
                        }
                    } else {
                        [
                            [background.r(), background.g(), background.b()],
                            [border.r(), border.g(), border.b()],
                        ]
                    };
                    let (close_colors, active) = toast
                        .motion
                        .as_mut()
                        .expect("rendered toast has motion")
                        .close_colors(targets.map(|color| color.map(f32::from)), now);
                    if active {
                        context.request_repaint();
                    }
                    let [close_background, close_border] = close_colors.map(|color| {
                        let [red, green, blue] = color
                            .map(|channel| channel.round().clamp(0.0, f32::from(u8::MAX)) as u8);
                        Color32::from_rgb(red, green, blue)
                    });
                    if close.has_focus() && visible && sample.close_opacity > 0.0 {
                        controls_ui.painter().circle_stroke(
                            close_rect.center(),
                            CLOSE_SIZE * 0.5 + FOCUS_RING,
                            Stroke::new(FOCUS_RING, Color32::from_black_alpha(FOCUS_ALPHA)),
                        );
                    }
                    controls_ui.painter().circle(
                        close_rect.center(),
                        CLOSE_SIZE * 0.5,
                        close_background,
                        Stroke::new(BORDER, close_border),
                    );
                    let glyph = close_rect.shrink(CLOSE_GLYPH_INSET);
                    controls_ui.painter().line_segment(
                        [glyph.left_top(), glyph.right_bottom()],
                        Stroke::new(CLOSE_GLYPH_STROKE, foreground),
                    );
                    controls_ui.painter().line_segment(
                        [glyph.right_top(), glyph.left_bottom()],
                        Stroke::new(CLOSE_GLYPH_STROKE, foreground),
                    );
                    close.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            controls_ui.is_enabled(),
                            CLOSE_LABEL,
                        )
                    });
                    context.memory_mut(|memory| {
                        memory.set_focus_lock_filter(
                            close_id,
                            egui::EventFilter {
                                escape: true,
                                ..Default::default()
                            },
                        )
                    });
                    if close.clicked() {
                        dismissed.push(toast.id);
                    }
                    if let (Some(label), Some(action)) = (&action_label, &toast.action)
                        && visible
                        && sample.content_opacity > 0.0
                        && (index == 0 || expanded)
                    {
                        let button_rect = action_rect(rect, label);
                        let action_id = mount.action(toast.id);
                        let mut action_ui = card_ui.new_child(
                            egui::UiBuilder::new()
                                .id_salt("native-toast-action")
                                .accessibility_parent(card_id)
                                .max_rect(button_rect),
                        );
                        action_ui.set_opacity(sample.opacity * sample.content_opacity);
                        let button = action_ui.interact(
                            transformed_rect(transform, button_rect),
                            action_id,
                            egui::Sense::click(),
                        );
                        let (action_background, action_foreground) = action_colors(theme);
                        if button.has_focus() {
                            action_ui.painter().rect_stroke(
                                button_rect.expand(ACTION_FOCUS_RING),
                                (f32::from(ACTION_RADIUS) + ACTION_FOCUS_RING).round() as u8,
                                Stroke::new(
                                    ACTION_FOCUS_RING,
                                    Color32::from_black_alpha(ACTION_FOCUS_ALPHA),
                                ),
                                egui::StrokeKind::Inside,
                            );
                        }
                        action_ui.painter().rect_filled(
                            button_rect,
                            ACTION_RADIUS,
                            action_background,
                        );
                        action_ui.painter().galley(
                            button_rect.center() - label.size() * 0.5,
                            label.clone(),
                            action_foreground,
                        );
                        button.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                action_ui.is_enabled(),
                                &action.label,
                            )
                        });
                        context.memory_mut(|memory| {
                            memory.set_focus_lock_filter(
                                action_id,
                                egui::EventFilter {
                                    escape: true,
                                    ..Default::default()
                                },
                            )
                        });
                        if button.clicked() {
                            triggered.push(action.id.clone());
                            dismissed.push(toast.id);
                        }
                    }
                    transform_shapes(ui, transform, start);
                }
            });
        self.actions.extend(triggered);
        for toast in &mut self.entries {
            if dismissed.contains(&toast.id) && toast.dismissed_at.is_none() {
                toast.dismiss(now);
            }
        }
        self.sync_focus(context, enabled);
        if let Some(delay) = self
            .entries
            .iter()
            .filter_map(|toast| {
                toast
                    .scheduled_removal_at
                    .map(|at| EXIT_DURATION.saturating_sub(now.saturating_duration_since(at)))
            })
            .min()
        {
            if self.is_reduced_motion {
                context.request_repaint_after(delay);
            } else {
                context.request_repaint();
            }
        }
        Ok(())
    }
}

impl<A: Clone> Toasts<A> {
    fn process_swipes(
        &mut self,
        context: &egui::Context,
        position: &Position,
        now: Instant,
        enabled: bool,
    ) {
        if !enabled || !context.input(|input| input.raw.focused) {
            if let Some(owner) = self.swipe_owner.take()
                && let Some(toast) = self
                    .entries
                    .iter_mut()
                    .find(|toast| toast.id == owner.toast)
                && let Some(motion) = &mut toast.motion
            {
                motion.return_from_swipe([owner.capture.amount.x, owner.capture.amount.y], now);
            }
            return;
        }
        let frame = context.cumulative_frame_nr();
        if self.pointer_frame == Some(frame) {
            return;
        }
        self.pointer_frame = Some(frame);
        if self.swipe_owner.as_ref().is_some_and(|owner| {
            !self
                .entries
                .iter()
                .any(|toast| toast.id == owner.toast && toast.dismissed_at.is_none())
        }) {
            self.swipe_owner = None;
        }
        for event in context.input(|input| input.raw.events.clone()) {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    ..
                } if button != egui::PointerButton::Secondary
                    && pos.is_finite()
                    && self.swipe_owner.is_none() =>
                {
                    if let Some(target) = self
                        .hit_targets
                        .iter()
                        .find(|target| target.card.contains(pos) || target.close.contains(pos))
                        && !target.close.contains(pos)
                        && !target.action.is_some_and(|action| action.contains(pos))
                        && self
                            .entries
                            .iter()
                            .any(|toast| toast.id == target.toast && toast.dismissed_at.is_none())
                    {
                        self.swipe_owner = Some(SwipeOwner {
                            toast: target.toast,
                            button,
                            capture: swipe::Capture::new(pos, now),
                        });
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(owner) = &mut self.swipe_owner {
                        owner.capture.update(
                            pos,
                            false,
                            swipe::Directions {
                                top: position.vertical != Vertical::Bottom,
                                bottom: position.vertical == Vertical::Bottom,
                                left: position.horizontal == Horizontal::Left,
                                right: position.horizontal == Horizontal::Right,
                            },
                        );
                    }
                }
                egui::Event::PointerButton {
                    button,
                    pressed: false,
                    ..
                } if self
                    .swipe_owner
                    .as_ref()
                    .is_some_and(|owner| owner.button == button) =>
                {
                    let owner = self
                        .swipe_owner
                        .take()
                        .expect("matching swipe owner exists");
                    if let Some(toast) = self
                        .entries
                        .iter_mut()
                        .find(|toast| toast.id == owner.toast)
                    {
                        if let Some(exit) = owner.capture.release(now) {
                            toast.swipe_exit = Some(exit);
                            toast.dismiss(now);
                        } else if let Some(motion) = &mut toast.motion {
                            motion.return_from_swipe(
                                [owner.capture.amount.x, owner.capture.amount.y],
                                now,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn transformed_rect(transform: egui::emath::TSTransform, rect: Rect) -> Rect {
    Rect::from_two_pos(transform * rect.min, transform * rect.max)
}

fn shape_index(ui: &egui::Ui) -> egui::layers::ShapeIdx {
    ui.ctx().graphics(|graphics| {
        graphics
            .get(ui.layer_id())
            .map_or(egui::layers::ShapeIdx(0), |list| list.next_idx())
    })
}

fn transform_shapes(
    ui: &egui::Ui,
    transform: egui::emath::TSTransform,
    start: egui::layers::ShapeIdx,
) {
    if transform == egui::emath::TSTransform::IDENTITY {
        return;
    }
    if transform.scaling > 0.0 {
        let screen = ui.ctx().content_rect();
        ui.ctx().graphics_mut(|graphics| {
            let list = graphics.entry(ui.layer_id());
            let end = list.next_idx();
            list.transform_range(start, end, transform);
            let shapes = list
                .all_entries()
                .skip(start.0)
                .cloned()
                .collect::<Vec<_>>();
            for (offset, shape) in shapes.into_iter().enumerate() {
                list.set(
                    egui::layers::ShapeIdx(start.0 + offset),
                    shape.clip_rect.intersect(screen),
                    shape.shape,
                );
            }
        });
        return;
    }
    let shapes = ui.ctx().graphics_mut(|graphics| {
        let list = graphics.entry(ui.layer_id());
        let shapes = list
            .all_entries()
            .skip(start.0)
            .cloned()
            .collect::<Vec<_>>();
        let end = list.next_idx();
        for index in start.0..end.0 {
            list.set(
                egui::layers::ShapeIdx(index),
                Rect::NOTHING,
                egui::Shape::Noop,
            );
        }
        shapes
    });
    if transform.scaling == 0.0 {
        return;
    }
    for primitive in ui.ctx().tessellate(shapes, ui.ctx().pixels_per_point()) {
        if let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive {
            for vertex in &mut mesh.vertices {
                vertex.pos = transform * vertex.pos;
            }
            ui.painter()
                .with_clip_rect(
                    transformed_rect(transform, primitive.clip_rect)
                        .intersect(ui.ctx().content_rect()),
                )
                .add(egui::Shape::mesh(mesh));
        }
    }
}

fn layout(
    context: &egui::Context,
    text: String,
    color: Color32,
    width: f32,
    line_height: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, FontId::proportional(FONT), color, width);
    for section in &mut job.sections {
        section.format.line_height = Some(line_height);
    }
    context.fonts_mut(|fonts| fonts.layout_job(job))
}

fn colors(theme: ThemeType, kind: Kind) -> (Color32, Color32, Color32) {
    let [background, border, foreground] = match (theme, kind) {
        (ThemeType::Light, Kind::Warning) => LIGHT_WARNING,
        (ThemeType::Light, Kind::Error) => LIGHT_ERROR,
        (ThemeType::Dark, Kind::Warning) => DARK_WARNING,
        (ThemeType::Dark, Kind::Error) => DARK_ERROR,
        (ThemeType::Light, Kind::Success) => LIGHT_SUCCESS,
        (ThemeType::Dark, Kind::Success) => DARK_SUCCESS,
        (ThemeType::Light, Kind::Info) => LIGHT_INFO,
        (ThemeType::Dark, Kind::Info) => DARK_INFO,
    };
    let [background, border, foreground] =
        [background, border, foreground].map(|[r, g, b]| Color32::from_rgb(r, g, b));
    (background, border, foreground)
}

fn action_colors(theme: ThemeType) -> (Color32, Color32) {
    let [background, foreground] = match theme {
        ThemeType::Light => LIGHT_ACTION,
        ThemeType::Dark => DARK_ACTION,
    }
    .map(|[red, green, blue]| Color32::from_rgb(red, green, blue));
    (background, foreground)
}

fn action_rect(card: Rect, label: &egui::Galley) -> Rect {
    let body = card.shrink(PADDING + BORDER);
    let top = body.center().y - ACTION_HEIGHT * 0.5;
    Rect::from_min_max(
        egui::pos2(body.right() - label.size().x - ACTION_PADDING * 2.0, top),
        egui::pos2(body.right(), top + ACTION_HEIGHT),
    )
}

pub fn describe_error(locale: &ResolvedLocale, error: &AppError) -> String {
    match error {
        AppError::Localized(error) if locale.messages.contains_key(&error.key) => {
            crate::presentation::message(
                locale,
                &error.key,
                &error
                    .args
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.as_str()))
                    .collect::<Vec<_>>(),
            )
        }
        AppError::Localized(error) => error.fallback.clone(),
        AppError::Io(value)
        | AppError::NotFound(value)
        | AppError::InvalidArgument(value)
        | AppError::Forbidden(value)
        | AppError::Internal(value) => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_toast는_성공아이콘과_원본_검증_및_오류_메시지를_보존한다() {
        use crate::snippet_editor::Notice;
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        let mut locale = locale();
        locale.messages.insert(
            "snippetEditor.incompleteEntryError".into(),
            "Incomplete {{count}}".into(),
        );
        for (notice, kind, title) in [
            (Notice::Saved, Kind::Success, "snippetEditor.saveSuccess"),
            (Notice::Incomplete(2), Kind::Error, "Incomplete 2"),
            (
                Notice::DuplicateNames,
                Kind::Error,
                "snippetEditor.duplicateNameError",
            ),
            (
                Notice::SaveFailed {
                    create: true,
                    error: AppError::InvalidArgument("Synthetic".into()),
                },
                Kind::Error,
                "snippetEditor.invalidFileName",
            ),
            (
                Notice::SaveFailed {
                    create: false,
                    error: AppError::InvalidArgument("Synthetic".into()),
                },
                Kind::Error,
                "snippetEditor.parseError",
            ),
            (
                Notice::SaveFailed {
                    create: false,
                    error: AppError::Io("Synthetic".into()),
                },
                Kind::Error,
                "snippetEditor.saveFailed",
            ),
            (
                Notice::DeleteFailed(
                    AppError::localized(AppErrorKind::Io, "test.failure", "fallback")
                        .with_arg("target", "synthetic"),
                ),
                Kind::Error,
                "Cannot write synthetic",
            ),
        ] {
            toasts.snippet(&locale, notice, now);
            assert!(toasts.entries[0].kind == kind);
            assert_eq!(toasts.entries[0].title, title);
            assert!(toasts.entries[0].description.is_none());
        }
        let context = egui::Context::default();
        toasts.assets.prepare(&context).unwrap();
        assert_eq!(toasts.assets.textures.len(), 4);
        assert_ne!(
            toasts.assets.textures[Kind::Success.index()].id(),
            toasts.assets.textures[Kind::Error.index()].id()
        );
        assert_eq!(
            colors(ThemeType::Light, Kind::Success).0,
            Color32::from_rgb(236, 253, 243)
        );
        assert_eq!(
            colors(ThemeType::Dark, Kind::Success).0,
            Color32::from_rgb(0, 31, 15)
        );
    }

    #[test]
    fn native_toast_position_key는_수명과_같은_top_middle을_구분한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("Synthetic position".into(), now);
        let render = |toasts: &mut Toasts, position: &str, at| {
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                toasts
                    .show(ui.ctx(), ThemeType::Dark, position, at, true)
                    .unwrap();
            });
            output.textures_delta.clear();
        };
        render(&mut toasts, "bottom-right", now);
        toasts.update(now + Duration::from_secs(2), false);
        render(&mut toasts, "top-right", now + Duration::from_secs(2));
        assert_eq!(toasts.entries[0].remaining, LIFETIME);
        toasts.update(now + Duration::from_secs(3), false);
        render(&mut toasts, "middle-right", now + Duration::from_secs(3));
        assert_eq!(toasts.entries[0].remaining, Duration::from_secs(3));
        render(
            &mut toasts,
            "middle-right-extra",
            now + Duration::from_secs(3),
        );
        assert_eq!(toasts.entries[0].remaining, Duration::from_secs(3));
        render(&mut toasts, "middle-left", now + Duration::from_secs(3));
        assert_eq!(toasts.entries[0].remaining, LIFETIME);
    }

    #[test]
    fn native_toast_position_remount는_부모pause와_예약제거를_보존한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("Synthetic older".into(), now);
        toasts.warning("Synthetic newer".into(), now);
        toasts.set_position(&context, "bottom-right", now, true);
        let front = toasts.entries[0].id;
        toasts.expanded = true;
        toasts.interacting = true;
        toasts.swipe_owner = Some(SwipeOwner {
            toast: front,
            button: egui::PointerButton::Primary,
            capture: swipe::Capture::new(egui::Pos2::ZERO, now),
        });
        toasts.set_position(&context, "top-right", now, true);
        assert!(toasts.expanded && toasts.interacting && toasts.paused);
        assert!(toasts.swipe_owner.is_none());
        toasts.update(now + Duration::from_secs(10), true);
        assert!(
            toasts
                .entries
                .iter()
                .all(|toast| toast.remaining == LIFETIME)
        );
        let dismissed = now + Duration::from_secs(10);
        toasts.entries[0].dismiss(dismissed);
        let mount = toasts.mount.0;
        toasts.set_position(&context, "middle-right", dismissed, true);
        assert_eq!(toasts.mount.0, mount);
        assert_eq!(toasts.entries[0].dismissed_at, Some(dismissed));
        toasts.set_position(&context, "bottom-left", dismissed + EXIT_DURATION / 2, true);
        assert_eq!(toasts.entries[0].dismissed_at, None);
        assert_eq!(toasts.entries[0].scheduled_removal_at, Some(dismissed));
        assert_eq!(toasts.entries[0].id, front);
        assert_eq!(toasts.entries[0].title, "Synthetic newer");
        assert!(toasts.expanded && toasts.interacting && toasts.paused);
        toasts.entries[0].dismiss(dismissed + EXIT_DURATION / 2);
        assert_eq!(toasts.entries[0].scheduled_removal_at, Some(dismissed));
        toasts.update(dismissed + EXIT_DURATION - Duration::from_nanos(1), true);
        assert_eq!(toasts.entries.len(), 2);
        toasts.update(dismissed + EXIT_DURATION, true);
        assert_eq!(toasts.entries.len(), 1);
        assert_ne!(toasts.entries[0].id, front);
        assert!(!toasts.expanded);
        let previous = toasts.mount.0;
        toasts.set_position(&context, "invalid", dismissed + EXIT_DURATION, false);
        assert_eq!(toasts.mount.0, previous + 1);
        toasts.set_position(
            &context,
            "bottom-right-extra",
            dismissed + EXIT_DURATION,
            false,
        );
        assert_eq!(toasts.mount.0, previous + 1);
    }

    #[test]
    fn native_toast_position_renderer는_이전_ax요청과_focus_swipe를_분리한다() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("Synthetic position".into(), now);
        let render = |toasts: &mut Toasts, position: &str, events, at, enabled, focus_origin| {
            let mut origin = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 800.0),
                    )),
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ui| {
                    let button = ui.button("Synthetic position origin");
                    origin = Some(button.id);
                    if focus_origin {
                        button.request_focus();
                    }
                    toasts.set_position(ui.ctx(), position, at, enabled);
                    toasts.tick(ui.ctx(), at, enabled);
                    toasts
                        .show(ui.ctx(), ThemeType::Dark, position, at, enabled)
                        .unwrap();
                },
            );
            output.textures_delta.clear();
            (origin.unwrap(), output)
        };
        let (origin, _) = render(&mut toasts, "bottom-right", Vec::new(), now, true, true);
        render(&mut toasts, "bottom-right", Vec::new(), now, true, false);
        render(
            &mut toasts,
            "bottom-right",
            vec![input_key(
                egui::Key::T,
                egui::Key::T,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )],
            now,
            true,
            false,
        );
        let old_list = toasts.mount.list();
        let old_card = toasts.mount.card(toasts.entries[0].id);
        let old_close = toasts.mount.close(toasts.entries[0].id);
        assert_eq!(context.memory(|memory| memory.focused()), Some(old_list));
        toasts.swipe_owner = Some(SwipeOwner {
            toast: toasts.entries[0].id,
            button: egui::PointerButton::Primary,
            capture: swipe::Capture::new(egui::Pos2::ZERO, now),
        });
        let old_click = egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: old_close.accesskit_id(),
            data: None,
        });
        let (_, output) = render(&mut toasts, "top-left", vec![old_click], now, true, false);
        assert_eq!(context.memory(|memory| memory.focused()), Some(origin));
        assert!(toasts.swipe_owner.is_none() && !toasts.focus_within);
        assert!(toasts.entries[0].dismissed_at.is_none());
        let new_list = toasts.mount.list();
        let new_card = toasts.mount.card(toasts.entries[0].id);
        let new_close = toasts.mount.close(toasts.entries[0].id);
        assert_ne!(new_list, old_list);
        assert_ne!(new_card, old_card);
        assert_ne!(new_close, old_close);
        let update = output.platform_output.accesskit_update.unwrap();
        assert!(
            update
                .nodes
                .iter()
                .any(|(id, node)| *id == new_list.accesskit_id()
                    && node.role() == egui::accesskit::Role::List)
        );
        assert!(!update.nodes.iter().any(|(id, _)| {
            [old_list, old_card, old_close]
                .iter()
                .any(|old| *id == old.accesskit_id())
        }));
        let bounds = toasts.bounds.clone();
        render(&mut toasts, "middle-left", Vec::new(), now, true, false);
        assert_eq!(toasts.mount.list(), new_list);
        assert_ne!(toasts.bounds, bounds);
        context.memory_mut(|memory| memory.request_focus(new_list));
        toasts.set_position(&context, "bottom-left", now, false);
        assert_eq!(context.memory(|memory| memory.focused()), None);
        let external = Id::new("synthetic-other-modal-focus");
        context.memory_mut(|memory| memory.request_focus(external));
        toasts.set_position(&context, "bottom-right", now, true);
        assert_eq!(context.memory(|memory| memory.focused()), Some(external));
    }
    use std::collections::BTreeMap;
    use taide_model::error::{AppErrorKind, LocalizedError};

    fn locale() -> ResolvedLocale {
        ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: BTreeMap::from([
                (
                    "settings.saveFailed".into(),
                    "Failed to save settings".into(),
                ),
                ("test.failure".into(), "Cannot write {{target}}".into()),
            ]),
        }
    }

    #[test]
    fn native_toast는_설정오류의_원본과_번역을_보존한다() {
        let locale = locale();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        let raw = [
            AppError::Io("disk".into()),
            AppError::NotFound("disk".into()),
            AppError::InvalidArgument("disk".into()),
            AppError::Forbidden("disk".into()),
            AppError::Internal("disk".into()),
        ];
        for error in raw {
            assert_eq!(describe_error(&locale, &error), "disk");
        }
        let translated = AppError::Localized(LocalizedError {
            kind: AppErrorKind::Io,
            key: "test.failure".into(),
            args: BTreeMap::from([("target".into(), "synthetic.json".into())]),
            fallback: "fallback".into(),
        });
        toasts.settings_failed(&locale, &translated, now);
        assert_eq!(toasts.entries[0].title, "Failed to save settings");
        assert_eq!(
            toasts.entries[0].description.as_deref(),
            Some("Cannot write synthetic.json")
        );
        assert!(toasts.entries[0].kind == Kind::Error);
        assert_eq!(
            describe_error(
                &locale,
                &AppError::localized(AppErrorKind::Io, "missing.key", "fallback")
            ),
            "fallback"
        );
        toasts.warning("capture warning".into(), now);
        assert_eq!(toasts.entries[0].title, "capture warning");
        assert!(toasts.entries[0].kind == Kind::Warning);
        assert!(toasts.entries[0].description.is_none());
        assert_ne!(toasts.entries[0].id, toasts.entries[1].id);
    }

    #[test]
    fn native_toast는_theme_오류를_settings_제목없이_원본으로_표시한다() {
        let locale = locale();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        let error = AppError::localized(AppErrorKind::Io, "test.failure", "fallback")
            .with_arg("target", "theme.json");
        toasts.ipc_error(&locale, &error, now);
        assert_eq!(toasts.entries[0].title, "Cannot write theme.json");
        assert!(toasts.entries[0].description.is_none());
        assert!(toasts.entries[0].kind == Kind::Error);
        toasts.ipc_error(
            &locale,
            &AppError::Internal("synthetic theme failure".into()),
            now,
        );
        assert_eq!(toasts.entries[0].title, "synthetic theme failure");
        assert!(toasts.entries[0].description.is_none());
    }

    #[test]
    fn app_file_toast는_settings_고정문구와_prompt_번역오류를_보존한다() {
        let mut locale = locale();
        locale.messages.insert(
            "settings.settingsJsonInvalid".into(),
            "Invalid settings JSON".into(),
        );
        let mut toasts = Toasts::new().unwrap();
        let now = Instant::now();
        let error = AppError::localized(AppErrorKind::Io, "test.failure", "fallback")
            .with_arg("target", "prompt.json");
        toasts.app_file_failed(
            &locale,
            taide_model::app::AppFileTarget::Settings,
            &error,
            now,
        );
        assert_eq!(toasts.entries[0].title, "Invalid settings JSON");
        assert!(toasts.entries[0].description.is_none());
        assert!(toasts.entries[0].kind == Kind::Error);
        toasts.app_file_failed(
            &locale,
            taide_model::app::AppFileTarget::Prompt {
                id: taide_model::app::PromptTemplateId::AutoTabDefault,
            },
            &error,
            now,
        );
        assert_eq!(toasts.entries[0].title, "Cannot write prompt.json");
        assert!(toasts.entries[0].description.is_none());
        assert!(toasts.entries[0].kind == Kind::Error);
    }

    #[test]
    fn native_toast는_숨김과_상호작용_동안_남은시간을_보존한다() {
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        for value in 0..5 {
            toasts.warning(value.to_string(), now);
        }
        toasts.update(now + Duration::from_secs(1), true);
        assert!(toasts.entries.iter().all(
            |toast| toast.remaining == Duration::from_secs(3) && toast.running_since.is_none()
        ));
        toasts.update(now + Duration::from_secs(30), true);
        toasts.warning("while paused".into(), now + Duration::from_secs(30));
        toasts.update(now + Duration::from_secs(40), false);
        toasts.update(now + Duration::from_secs(43), false);
        assert!(
            toasts
                .entries
                .iter()
                .skip(1)
                .all(|toast| toast.dismissed_at == Some(now + Duration::from_secs(43)))
        );
        assert_eq!(toasts.entries[0].remaining, Duration::from_secs(1));
        toasts.update(now + Duration::from_secs(43) + EXIT_DURATION, false);
        assert_eq!(toasts.entries.len(), 1);
        toasts.update(now + Duration::from_secs(44), false);
        assert!(toasts.entries[0].dismissed_at.is_some());
        toasts.update(now + Duration::from_secs(44) + EXIT_DURATION, false);
        assert!(toasts.entries.is_empty());
    }

    #[test]
    fn native_toast는_원본_아홉위치와_모바일_기준을_사용한다() {
        let screen = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(1000.0, 800.0));
        let height = 80.0;
        for (vertical, expected_y) in [("top", 44.0), ("middle", 420.0), ("bottom", 716.0)] {
            for (horizontal, expected_x) in [("left", 34.0), ("center", 332.0), ("right", 630.0)] {
                let expected_x = if vertical == "middle" && horizontal == "center" {
                    510.0
                } else {
                    expected_x
                };
                let rect = Position::parse(&format!("{vertical}-{horizontal}"))
                    .geometry(screen, height, 0.0);
                assert_eq!(rect.min, egui::pos2(expected_x, expected_y));
                assert_eq!(rect.width(), WIDTH);
            }
        }
        let default = Position::parse("bad-center").geometry(screen, height, 0.0);
        assert_eq!(
            default,
            Position::parse("bottom-right-extra").geometry(screen, height, 0.0)
        );
        assert_eq!(
            default,
            Position::parse("top-bad").geometry(screen, height, 0.0)
        );
        let mobile = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 800.0));
        assert_eq!(
            Position::parse("top-center").geometry(mobile, height, 0.0),
            Rect::from_min_size(egui::pos2(16.0, 16.0), egui::vec2(568.0, height))
        );
    }

    fn frame(
        toasts: &mut Toasts,
        context: &egui::Context,
        now: Instant,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| {
            toasts.tick(ui.ctx(), now, true);
            toasts
                .show(ui.ctx(), ThemeType::Dark, "bottom-right", now, true)
                .unwrap();
        });
        output.textures_delta.clear();
        output
    }

    #[test]
    fn native_toast는_실제_카드_내용_아이콘_닫기와_종료중_내용을_그린다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.settings_failed(
            &locale(),
            &AppError::Io("synthetic disk failure".into()),
            now,
        );
        frame(&mut toasts, &context, now, Vec::new());
        let now = now + Duration::from_millis(400);
        let output = frame(&mut toasts, &context, now, Vec::new());
        let has_text = |output: &egui::FullOutput, expected| {
            output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == expected))
        };
        assert!(has_text(&output, "Failed to save settings"));
        assert!(has_text(&output, "synthetic disk failure"));
        let texture = toasts.assets.textures[Kind::Error.index()].id();
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.brush.as_ref().is_some_and(|brush| brush.fill_texture_id == texture))));
        let close = toasts.bounds[0].min + egui::Vec2::splat(CLOSE_SIZE * 0.5);
        for pressed in [true, false] {
            frame(
                &mut toasts,
                &context,
                now,
                vec![
                    egui::Event::PointerMoved(close),
                    egui::Event::PointerButton {
                        pos: close,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert_eq!(toasts.entries[0].dismissed_at, Some(now));
        let output = frame(&mut toasts, &context, now + EXIT_DURATION / 2, Vec::new());
        assert!(has_text(&output, "Failed to save settings"));
        assert!(has_text(&output, "synthetic disk failure"));
        assert_eq!(toasts.entries[0].title, "Failed to save settings");
        assert_eq!(
            toasts.entries[0].description.as_deref(),
            Some("synthetic disk failure")
        );
        frame(&mut toasts, &context, now + EXIT_DURATION, Vec::new());
        assert!(toasts.entries.is_empty());
    }

    fn input_key(
        key: egui::Key,
        physical_key: egui::Key,
        modifiers: egui::Modifiers,
    ) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(physical_key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn focus_frame(
        toasts: &mut Toasts,
        context: &egui::Context,
        now: Instant,
        events: Vec<egui::Event>,
        focus_origin: bool,
        enabled: bool,
        discard: bool,
    ) -> (Id, egui::FullOutput) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events,
            focused: true,
            ..Default::default()
        };
        let mut origin = None;
        let mut output = context.run_ui(input, |ui| {
            let button = ui.button("Synthetic origin");
            origin = Some(button.id);
            if focus_origin {
                button.request_focus();
            }
            let has_hotkey = context.input(|input| input.raw.events.iter().any(|event| matches!(event, egui::Event::Key { physical_key: Some(egui::Key::T), modifiers, .. } if modifiers.alt)));
            toasts.tick(ui.ctx(), now, enabled);
            if has_hotkey {
                assert!(context.input(|input| input.events.iter().any(|event| matches!(event, egui::Event::Key { physical_key: Some(egui::Key::T), .. }))));
            }
            toasts
                .show(ui.ctx(), ThemeType::Dark, "bottom-right", now, enabled)
                .unwrap();
            if discard && context.current_pass_index() == 0 {
                context.request_discard("native toast keyboard test");
            }
        });
        output.textures_delta.clear();
        (origin.unwrap(), output)
    }

    #[test]
    fn native_toast_focus는_물리키_hotkey_escape_tab_복원과_중복pass를_보존한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("older".into(), now);
        toasts.warning("newer".into(), now);
        let front = toasts.entries[0].id;
        let list = toasts.mount.list();
        let (origin, _) = focus_frame(&mut toasts, &context, now, Vec::new(), true, true, false);
        focus_frame(&mut toasts, &context, now, Vec::new(), false, true, false);
        let hotkey = input_key(
            egui::Key::Y,
            egui::Key::T,
            egui::Modifiers {
                alt: true,
                ctrl: true,
                shift: true,
                ..Default::default()
            },
        );
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_secs(1),
            vec![hotkey],
            false,
            true,
            true,
        );
        assert_eq!(context.memory(|memory| memory.focused()), Some(list));
        assert!(toasts.expanded && toasts.paused && toasts.focus_within);
        assert_eq!(toasts.previous_focus, Some(origin));
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_secs(30),
            Vec::new(),
            false,
            true,
            false,
        );
        assert!(
            toasts
                .entries
                .iter()
                .all(|toast| toast.remaining == Duration::from_secs(3))
        );
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_secs(30),
            vec![input_key(
                egui::Key::Escape,
                egui::Key::Escape,
                Default::default(),
            )],
            false,
            true,
            false,
        );
        assert!(!toasts.expanded && !toasts.paused);
        assert_eq!(context.memory(|memory| memory.focused()), Some(list));
        assert_eq!(toasts.entries.len(), 2);
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(30700),
            vec![input_key(
                egui::Key::Tab,
                egui::Key::Tab,
                Default::default(),
            )],
            false,
            true,
            true,
        );
        let card = toasts.mount.card(front);
        assert_eq!(context.memory(|memory| memory.focused()), Some(card));
        let (_, drawing) = focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(30700) + EXIT_DURATION,
            Vec::new(),
            false,
            true,
            false,
        );
        let card_rect = context.read_response(card).unwrap().rect;
        assert!(drawing.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.rect == card_rect.expand(FOCUS_RING) && rect.stroke.width == FOCUS_RING)));
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(31100),
            vec![input_key(
                egui::Key::Tab,
                egui::Key::Tab,
                Default::default(),
            )],
            false,
            true,
            false,
        );
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(toasts.mount.close(front))
        );
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(31200),
            vec![input_key(
                egui::Key::Enter,
                egui::Key::Enter,
                Default::default(),
            )],
            false,
            true,
            true,
        );
        assert!(
            toasts
                .entries
                .iter()
                .find(|toast| toast.id == front)
                .unwrap()
                .dismissed_at
                .is_some()
        );
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(31400),
            Vec::new(),
            false,
            true,
            false,
        );
        assert_eq!(toasts.entries.len(), 1);
        assert_eq!(context.memory(|memory| memory.focused()), Some(origin));
        assert!(!toasts.focus_within);
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(31400),
            vec![input_key(
                egui::Key::T,
                egui::Key::T,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )],
            false,
            true,
            false,
        );
        assert_eq!(context.memory(|memory| memory.focused()), Some(list));
        focus_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(31400),
            vec![input_key(
                egui::Key::Tab,
                egui::Key::Tab,
                egui::Modifiers {
                    shift: true,
                    ..Default::default()
                },
            )],
            false,
            true,
            false,
        );
        assert_eq!(context.memory(|memory| memory.focused()), Some(origin));
    }

    #[test]
    fn native_toast_accessibility는_전체항목_문구_live_focus와_숨김항목_닫기를_보존한다() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        for index in 0..5 {
            toasts.settings_failed(
                &locale(),
                &AppError::Io(format!("synthetic failure {index}")),
                now,
            );
        }
        let hidden = toasts.entries[4].id;
        let (origin, _) = focus_frame(&mut toasts, &context, now, Vec::new(), true, true, false);
        let (_, output) = focus_frame(&mut toasts, &context, now, Vec::new(), false, true, false);
        let update = output.platform_output.accesskit_update.unwrap();
        let list = update
            .nodes
            .iter()
            .find(|(id, _)| *id == toasts.mount.list().accesskit_id())
            .unwrap();
        assert_eq!(list.1.role(), egui::accesskit::Role::List);
        assert_eq!(list.1.label(), Some(LIST_LABEL));
        assert_eq!(list.1.live(), Some(egui::accesskit::Live::Polite));
        assert!(!list.1.is_live_atomic());
        assert_eq!(
            update
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::ListItem)
                .count(),
            5
        );
        let descriptions = list
            .1
            .children()
            .iter()
            .map(|id| {
                let group = &update
                    .nodes
                    .iter()
                    .find(|(node_id, _)| node_id == id)
                    .unwrap()
                    .1;
                let item = group
                    .children()
                    .iter()
                    .find_map(|id| {
                        update.nodes.iter().find(|(node_id, node)| {
                            node_id == id && node.role() == egui::accesskit::Role::ListItem
                        })
                    })
                    .unwrap();
                assert_eq!(item.1.label(), Some("Failed to save settings"));
                assert!(item.1.value().is_none());
                item.1.description().unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            descriptions,
            [
                "synthetic failure 4",
                "synthetic failure 3",
                "synthetic failure 2",
                "synthetic failure 1",
                "synthetic failure 0"
            ]
        );
        let focus = egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Focus,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: toasts.mount.list().accesskit_id(),
            data: None,
        });
        focus_frame(&mut toasts, &context, now, vec![focus], false, true, false);
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(toasts.mount.list())
        );
        assert!(!toasts.expanded);
        let close = egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: toasts.mount.close(hidden).accesskit_id(),
            data: None,
        });
        focus_frame(&mut toasts, &context, now, vec![close], false, true, false);
        assert_eq!(
            toasts
                .entries
                .iter()
                .find(|toast| toast.id == hidden)
                .unwrap()
                .dismissed_at,
            Some(now)
        );
        let (_, output) = focus_frame(
            &mut toasts,
            &context,
            now,
            vec![input_key(
                egui::Key::T,
                egui::Key::T,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )],
            true,
            false,
            false,
        );
        assert_eq!(context.memory(|memory| memory.focused()), Some(origin));
        assert!(!toasts.focus_within);
        let update = output.platform_output.accesskit_update.unwrap();
        assert!(
            update
                .nodes
                .iter()
                .find(|(id, _)| *id == toasts.mount.list().accesskit_id())
                .unwrap()
                .1
                .is_hidden()
        );
        assert!(
            toasts
                .entries
                .iter()
                .filter(|toast| toast.id != hidden)
                .all(|toast| toast.dismissed_at.is_none())
        );
    }

    #[test]
    fn native_toast_reduced_motion_render는_즉시그림과_swipe_200ms제거를_보존한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.set_reduced_motion(true);
        toasts.warning("reduced synthetic".into(), now);
        let output = frame(&mut toasts, &context, now, Vec::new());
        let has_title = |output: &egui::FullOutput| {
            output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "reduced synthetic"))
        };
        assert!(has_title(&output));
        let rect = toasts.hit_targets[0].card;
        assert_eq!(rect.bottom(), 800.0 - VIEWPORT_OFFSET);
        frame(
            &mut toasts,
            &context,
            now,
            vec![input_key(
                egui::Key::T,
                egui::Key::T,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )],
        );
        let output = frame(
            &mut toasts,
            &context,
            now,
            vec![input_key(
                egui::Key::Tab,
                egui::Key::Tab,
                Default::default(),
            )],
        );
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == Color32::TRANSPARENT && rect.stroke.width == FOCUS_RING && rect.stroke.color.a() == FOCUS_ALPHA)));
        let origin = rect.center();
        let press = |pressed| egui::Event::PointerButton {
            pos: origin,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        frame(
            &mut toasts,
            &context,
            now,
            vec![egui::Event::PointerMoved(origin), press(true)],
        );
        for elapsed in [1, 2] {
            frame(
                &mut toasts,
                &context,
                now + Duration::from_millis(elapsed),
                vec![egui::Event::PointerMoved(origin + egui::vec2(50.0, 0.0))],
            );
        }
        let released = now + Duration::from_millis(3);
        let output = frame(&mut toasts, &context, released, vec![press(false)]);
        assert!(has_title(&output));
        assert_eq!(toasts.entries[0].dismissed_at, Some(released));
        let output = frame(
            &mut toasts,
            &context,
            released + EXIT_DURATION / 2,
            Vec::new(),
        );
        assert!(has_title(&output));
        assert_eq!(toasts.bounds[0].right(), rect.right() + 50.0);
        frame(&mut toasts, &context, released + EXIT_DURATION, Vec::new());
        assert!(toasts.entries.is_empty());
    }

    #[test]
    fn native_toast_shadow는_keyboard_focus와_blur를_200ms에_전환한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("focus shadow".into(), now);
        frame(&mut toasts, &context, now, Vec::new());
        let at = now + Duration::from_secs(1);
        frame(&mut toasts, &context, at, Vec::new());
        let hotkey = || {
            input_key(
                egui::Key::T,
                egui::Key::T,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )
        };
        frame(&mut toasts, &context, at, vec![hotkey()]);
        let rings = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == Color32::TRANSPARENT
                            && rect.stroke.color.r() == 0
                            && rect.stroke.color.g() == 0
                            && rect.stroke.color.b() == 0
                            && rect.stroke.color.a() > 0 =>
                    {
                        Some((rect.stroke.width, rect.stroke.color.a()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let output = frame(
            &mut toasts,
            &context,
            at,
            vec![input_key(
                egui::Key::Tab,
                egui::Key::Tab,
                Default::default(),
            )],
        );
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(toasts.mount.card(toasts.entries[0].id))
        );
        assert!(rings(&output).is_empty());
        let output = frame(
            &mut toasts,
            &context,
            at + Duration::from_millis(100),
            Vec::new(),
        );
        let middle = rings(&output);
        assert_eq!(middle.len(), 1);
        assert!(middle[0].0 > 0.0 && middle[0].0 < FOCUS_RING);
        assert!(middle[0].1 > 0 && middle[0].1 < FOCUS_ALPHA);
        let output = frame(&mut toasts, &context, at + EXIT_DURATION, Vec::new());
        assert_eq!(rings(&output), vec![(FOCUS_RING, FOCUS_ALPHA)]);
        let blur = at + Duration::from_millis(300);
        let output = frame(&mut toasts, &context, blur, vec![hotkey()]);
        assert_eq!(rings(&output), vec![(FOCUS_RING, FOCUS_ALPHA)]);
        let output = frame(
            &mut toasts,
            &context,
            blur + Duration::from_millis(100),
            Vec::new(),
        );
        let middle = rings(&output);
        assert_eq!(middle.len(), 1);
        assert!(middle[0].0 > 0.0 && middle[0].0 < FOCUS_RING);
        assert!(middle[0].1 > 0 && middle[0].1 < FOCUS_ALPHA);
        let output = frame(&mut toasts, &context, blur + EXIT_DURATION, Vec::new());
        assert!(rings(&output).is_empty());
        assert!(
            toasts
                .entries
                .iter()
                .all(|toast| toast.dismissed_at.is_none())
        );
    }

    #[test]
    fn native_toast_hover_parent는_정지포인터_escape와_드래그밖_상태를_보존한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("older".into(), now);
        toasts.warning("front".into(), now);
        frame(&mut toasts, &context, now, Vec::new());
        let at = now + Duration::from_secs(1);
        frame(&mut toasts, &context, at, Vec::new());
        let inside = toasts.bounds.last().unwrap().center();
        frame(
            &mut toasts,
            &context,
            at,
            vec![egui::Event::PointerMoved(inside)],
        );
        assert!(toasts.expanded);
        context.memory_mut(|memory| memory.request_focus(toasts.mount.list()));
        frame(&mut toasts, &context, at, Vec::new());
        assert!(toasts.focus_within);
        frame(
            &mut toasts,
            &context,
            at,
            vec![input_key(
                egui::Key::Escape,
                egui::Key::Escape,
                Default::default(),
            )],
        );
        assert!(!toasts.expanded);
        frame(&mut toasts, &context, at, Vec::new());
        assert!(!toasts.expanded);
        let press = |pressed| egui::Event::PointerButton {
            pos: inside,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        frame(
            &mut toasts,
            &context,
            at,
            vec![egui::Event::PointerMoved(inside)],
        );
        frame(&mut toasts, &context, at, vec![press(true)]);
        frame(&mut toasts, &context, at, vec![egui::Event::PointerGone]);
        assert!(toasts.expanded && toasts.paused);
        frame(
            &mut toasts,
            &context,
            at,
            vec![press(false), egui::Event::PointerGone],
        );
        assert!(toasts.expanded && toasts.paused);
        frame(
            &mut toasts,
            &context,
            at,
            vec![egui::Event::PointerMoved(inside)],
        );
        frame(&mut toasts, &context, at, vec![egui::Event::PointerGone]);
        assert!(!toasts.expanded && !toasts.paused);
        assert!(
            toasts
                .entries
                .iter()
                .all(|toast| toast.dismissed_at.is_none())
        );
    }

    #[test]
    fn native_toast_hover는_두theme의_닫기색을_200ms에_전환한다() {
        for theme in [ThemeType::Dark, ThemeType::Light] {
            let context = egui::Context::default();
            let now = Instant::now();
            let mut toasts = Toasts::new().unwrap();
            toasts.warning("hover synthetic".into(), now);
            let draw = |toasts: &mut Toasts, at, events| {
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1000.0, 800.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        toasts.tick(ui.ctx(), at, true);
                        toasts
                            .show(ui.ctx(), theme, "bottom-right", at, true)
                            .unwrap();
                    },
                );
                output.textures_delta.clear();
                output
            };
            draw(&mut toasts, now, Vec::new());
            let settled = now + Duration::from_millis(400);
            draw(&mut toasts, settled, Vec::new());
            let output = draw(&mut toasts, settled, Vec::new());
            let circle = |output: &egui::FullOutput| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Circle(circle) if circle.radius == CLOSE_SIZE * 0.5 => {
                            Some((circle.fill, circle.stroke.color))
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            let (background, border, _) = colors(theme, Kind::Warning);
            assert_eq!(circle(&output), (background, border));
            let close = context
                .read_response(toasts.mount.close(toasts.entries[0].id))
                .unwrap()
                .rect
                .center();
            let hovered = now + Duration::from_secs(1);
            draw(&mut toasts, hovered, vec![egui::Event::PointerMoved(close)]);
            let output = draw(
                &mut toasts,
                hovered + Duration::from_millis(100),
                Vec::new(),
            );
            let expected_middle = match theme {
                ThemeType::Dark => Color32::from_rgb(31, 31, 25),
                ThemeType::Light => Color32::from_rgb(249, 249, 246),
            };
            assert_eq!(circle(&output).0, expected_middle);
            let output = draw(
                &mut toasts,
                hovered + Duration::from_millis(200),
                Vec::new(),
            );
            let [expected_background, expected_border] = match theme {
                ThemeType::Dark => DARK_CLOSE_HOVER,
                ThemeType::Light => LIGHT_CLOSE_HOVER,
            }
            .map(|[r, g, b]| Color32::from_rgb(r, g, b));
            assert_eq!(circle(&output), (expected_background, expected_border));
            let left = hovered + Duration::from_millis(300);
            draw(&mut toasts, left, vec![egui::Event::PointerGone]);
            let output = draw(&mut toasts, left + Duration::from_millis(200), Vec::new());
            assert_eq!(circle(&output), (background, border));
            assert!(toasts.entries[0].dismissed_at.is_none());
        }
    }

    #[test]
    fn native_toast_swipe_render는_raw_capture_복귀와_200ms_종료를_연결한다() {
        const GEOMETRY_TOLERANCE: f32 = 0.001;
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("swipe synthetic".into(), now);
        frame(&mut toasts, &context, now, Vec::new());
        let start = now + Duration::from_secs(1);
        frame(&mut toasts, &context, start, Vec::new());
        frame(&mut toasts, &context, start, Vec::new());
        let origin = toasts.bounds[0].center();
        let initial = toasts.bounds[0];
        frame(
            &mut toasts,
            &context,
            start,
            vec![
                egui::Event::PointerMoved(origin),
                egui::Event::PointerButton {
                    pos: origin,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        assert!(toasts.swipe_owner.is_some());
        frame(
            &mut toasts,
            &context,
            start + Duration::from_millis(100),
            vec![egui::Event::PointerMoved(origin + egui::vec2(2.0, 0.0))],
        );
        assert_eq!(
            toasts.swipe_owner.as_ref().unwrap().capture.amount,
            egui::Vec2::ZERO
        );
        frame(
            &mut toasts,
            &context,
            start + Duration::from_millis(300),
            vec![egui::Event::PointerMoved(origin + egui::vec2(20.0, 0.0))],
        );
        assert!((toasts.bounds[0].left() - initial.left() - 20.0).abs() < GEOMETRY_TOLERANCE);
        frame(
            &mut toasts,
            &context,
            start + Duration::from_millis(1000),
            vec![egui::Event::PointerButton {
                pos: origin + egui::vec2(20.0, 0.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert!(toasts.swipe_owner.is_none());
        assert!(toasts.entries[0].dismissed_at.is_none());
        assert!((toasts.bounds[0].left() - initial.left() - 20.0).abs() < GEOMETRY_TOLERANCE);
        frame(
            &mut toasts,
            &context,
            start + Duration::from_millis(1400),
            vec![egui::Event::PointerGone],
        );
        assert!((toasts.bounds[0].left() - initial.left()).abs() < GEOMETRY_TOLERANCE);
        let second = start + Duration::from_millis(1500);
        frame(
            &mut toasts,
            &context,
            second,
            vec![
                egui::Event::PointerMoved(origin),
                egui::Event::PointerButton {
                    pos: origin,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            &mut toasts,
            &context,
            second + Duration::from_millis(50),
            vec![egui::Event::PointerMoved(origin + egui::vec2(2.0, 0.0))],
        );
        frame(
            &mut toasts,
            &context,
            second + Duration::from_millis(100),
            vec![egui::Event::PointerMoved(origin + egui::vec2(50.0, 0.0))],
        );
        let released = second + Duration::from_millis(150);
        frame(
            &mut toasts,
            &context,
            released,
            vec![egui::Event::PointerButton {
                pos: origin + egui::vec2(50.0, 0.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert_eq!(toasts.entries[0].dismissed_at, Some(released));
        assert!(toasts.entries[0].swipe_exit.is_some());
        assert!((toasts.bounds[0].left() - initial.left() - 50.0).abs() < GEOMETRY_TOLERANCE);
        frame(
            &mut toasts,
            &context,
            released + EXIT_DURATION / 2,
            vec![egui::Event::PointerGone],
        );
        assert!(toasts.bounds[0].left() > initial.left() + 50.0);
        assert_eq!(toasts.entries.len(), 1);
        frame(&mut toasts, &context, released + EXIT_DURATION, Vec::new());
        assert!(toasts.entries.is_empty());
        assert!(toasts.hit_targets.is_empty());
    }

    #[test]
    fn native_toast_motion_render는_음수scale의_그림과_닫기_hit을_일치시킨다() {
        const GEOMETRY_TOLERANCE: f32 = 0.001;
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        toasts.warning("older".into(), now);
        frame(&mut toasts, &context, now, Vec::new());
        frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(400),
            Vec::new(),
        );
        let older = toasts.entries[0].id;
        toasts.warning("newer".into(), now + Duration::from_millis(400));
        frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(400),
            Vec::new(),
        );
        let settled = now + Duration::from_millis(800);
        frame(&mut toasts, &context, settled, Vec::new());
        let output = frame(&mut toasts, &context, settled, Vec::new());
        let older_rect = context
            .read_response(toasts.mount.card(older))
            .unwrap()
            .rect;
        assert!(
            (older_rect.width() - WIDTH * 1.05).abs() < GEOMETRY_TOLERANCE,
            "older={older_rect:?}, bounds={:?}, expanded={}",
            toasts.bounds,
            toasts.expanded
        );
        let close_id = toasts.mount.close(older);
        let close_rect = context.read_response(close_id).unwrap().rect;
        assert!((close_rect.width() - CLOSE_SIZE * 1.05).abs() < GEOMETRY_TOLERANCE);
        assert!(close_rect.center().x > older_rect.center().x);
        assert!(close_rect.center().y > older_rect.center().y);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Mesh(mesh) if !mesh.vertices.is_empty() && mesh.vertices.iter().all(|vertex| vertex.pos.x.is_finite() && vertex.pos.y.is_finite()))));
        for pressed in [true, false] {
            frame(
                &mut toasts,
                &context,
                settled,
                vec![
                    egui::Event::PointerMoved(close_rect.center()),
                    egui::Event::PointerButton {
                        pos: close_rect.center(),
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert_eq!(
            toasts
                .entries
                .iter()
                .find(|toast| toast.id == older)
                .unwrap()
                .dismissed_at,
            Some(settled)
        );
        assert!(toasts.entries[0].dismissed_at.is_none());
        frame(
            &mut toasts,
            &context,
            settled + EXIT_DURATION / 2,
            vec![egui::Event::PointerGone],
        );
        assert!(toasts.entries.iter().any(|toast| toast.id == older));
        frame(&mut toasts, &context, settled + EXIT_DURATION, Vec::new());
        assert_eq!(toasts.entries.len(), 1);
        assert_eq!(toasts.entries[0].title, "newer");
    }

    #[test]
    fn native_toast_stack은_숨겨진행_타이머와_확장간격_입력을_보존한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::new().unwrap();
        for index in 0..5 {
            toasts.warning(format!("warning {index}"), now);
        }
        frame(&mut toasts, &context, now, Vec::new());
        let output = frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(400),
            Vec::new(),
        );
        assert_eq!(toasts.bounds.len(), VISIBLE);
        assert_eq!(output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().starts_with("warning"))).count(), 1);
        let icon_texture = toasts.assets.textures[Kind::Warning.index()].id();
        let icon = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == icon_texture) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(icon.width(), ICON_SIZE);
        assert_eq!(
            icon.left(),
            1000.0 - VIEWPORT_OFFSET - WIDTH
                + PADDING
                + BORDER
                + ICON_LEFT_MARGIN
                + ICON_SVG_MARGIN
        );
        let front = toasts.bounds.last().unwrap().center();
        frame(
            &mut toasts,
            &context,
            now + Duration::from_secs(1),
            vec![egui::Event::PointerMoved(front)],
        );
        assert!(toasts.paused && toasts.hovered);
        let output = frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(1400),
            Vec::new(),
        );
        assert_eq!(output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().starts_with("warning"))).count(), VISIBLE);
        let first = toasts.bounds[VISIBLE - 1];
        let second = toasts.bounds[VISIBLE - 2];
        let gap = egui::pos2(first.center().x, (first.min.y + second.max.y) * 0.5);
        frame(
            &mut toasts,
            &context,
            now + Duration::from_secs(10),
            vec![egui::Event::PointerMoved(gap)],
        );
        assert!(toasts.paused && toasts.hovered);
        assert!(
            toasts
                .entries
                .iter()
                .all(|toast| toast.remaining == Duration::from_secs(3))
        );
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events: vec![egui::Event::PointerGone],
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .minimized = Some(true);
        let mut output = context.run_ui(input, |ui| {
            toasts.tick(ui.ctx(), now + Duration::from_secs(20), true)
        });
        output.textures_delta.clear();
        assert!(toasts.paused);
        frame(
            &mut toasts,
            &context,
            now + Duration::from_secs(30),
            vec![egui::Event::PointerGone],
        );
        assert!(!toasts.paused);
        frame(
            &mut toasts,
            &context,
            now + Duration::from_secs(33),
            Vec::new(),
        );
        assert!(
            toasts
                .entries
                .iter()
                .all(|toast| toast.dismissed_at.is_some())
        );
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Probe {
        Retry,
        PullRemote,
    }

    fn action_frame(
        toasts: &mut Toasts<Probe>,
        context: &egui::Context,
        now: Instant,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| {
            toasts.tick(ui.ctx(), now, true);
            toasts
                .show(ui.ctx(), ThemeType::Light, "bottom-right", now, true)
                .unwrap();
        });
        output.textures_delta.clear();
        output
    }

    #[test]
    fn 일반_toast는_네_종류와_선택적_설명_액션을_보존한다() {
        let now = Instant::now();
        let mut toasts = Toasts::<Probe>::with_actions().unwrap();
        toasts.success("Synthetic success".into(), now);
        assert!(toasts.entries[0].kind == Kind::Success);
        toasts.error("Synthetic error".into(), now);
        assert!(toasts.entries[0].kind == Kind::Error);
        toasts.info("Synthetic info".into(), now);
        assert!(toasts.entries[0].kind == Kind::Info);
        toasts.warning("Synthetic warning".into(), now);
        assert!(toasts.entries[0].kind == Kind::Warning);
        assert!(toasts.entries.iter().all(|toast| {
            toast.description.is_none() && toast.action.is_none() && toast.remaining == LIFETIME
        }));
        toasts.notify(
            Kind::Warning,
            "Synthetic conflict".into(),
            Options {
                description: Some("Synthetic conflict description".into()),
                action: Some(Action {
                    label: "Pull Remote".into(),
                    id: Probe::PullRemote,
                }),
            },
            now,
        );
        assert_eq!(toasts.entries.len(), 5);
        assert_eq!(toasts.entries[0].title, "Synthetic conflict");
        assert_eq!(
            toasts.entries[0].description.as_deref(),
            Some("Synthetic conflict description")
        );
        assert!(
            toasts.entries[0].action
                == Some(Action {
                    label: "Pull Remote".into(),
                    id: Probe::PullRemote,
                })
        );
        let inspection = toasts.inspection();
        assert!(inspection[0].kind == Kind::Warning);
        assert_eq!(inspection[0].action_label.as_deref(), Some("Pull Remote"));
        assert!(inspection[2].kind == Kind::Info);
        assert!(inspection[2].action_label.is_none());
        let context = egui::Context::default();
        toasts.assets.prepare(&context).unwrap();
        assert_ne!(
            toasts.assets.textures[Kind::Info.index()].id(),
            toasts.assets.textures[Kind::Error.index()].id()
        );
        assert_eq!(
            colors(ThemeType::Light, Kind::Info),
            (
                Color32::from_rgb(240, 248, 255),
                Color32::from_rgb(221, 231, 253),
                Color32::from_rgb(9, 115, 220)
            )
        );
        assert_eq!(
            colors(ThemeType::Dark, Kind::Info),
            (
                Color32::from_rgb(0, 13, 31),
                Color32::from_rgb(25, 35, 62),
                Color32::from_rgb(88, 150, 243)
            )
        );
    }

    #[test]
    fn 일반_toast는_살아있는_같은_알림만_표시중으로_답한다() {
        let now = Instant::now();
        let mut toasts = Toasts::<Probe>::with_actions().unwrap();
        assert!(!toasts.is_showing(Kind::Error, "Synthetic failure"));
        toasts.error("Synthetic failure".into(), now);
        assert!(toasts.is_showing(Kind::Error, "Synthetic failure"));
        assert!(!toasts.is_showing(Kind::Warning, "Synthetic failure"));
        assert!(!toasts.is_showing(Kind::Error, "Synthetic other"));
        toasts.update(now + LIFETIME, false);
        assert!(toasts.entries[0].dismissed_at.is_some());
        assert!(!toasts.is_showing(Kind::Error, "Synthetic failure"));
    }

    #[test]
    fn 일반_toast_액션은_버튼을_그리고_식별자를_한번만_돌려준_뒤_닫는다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::<Probe>::with_actions().unwrap();
        toasts.notify(
            Kind::Error,
            "Synthetic create failure".into(),
            Options {
                description: None,
                action: Some(Action {
                    label: "Retry".into(),
                    id: Probe::Retry,
                }),
            },
            now,
        );
        action_frame(&mut toasts, &context, now, Vec::new());
        let now = now + Duration::from_millis(400);
        let output = action_frame(&mut toasts, &context, now, Vec::new());
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(texts.iter().any(|text| text == "Synthetic create failure"));
        assert!(texts.iter().any(|text| text == "Retry"));
        let card = toasts.hit_targets[0].card;
        let button = toasts.hit_targets[0].action.unwrap();
        let is_near = |left: f32, right: f32| (left - right).abs() < 0.01;
        assert!(is_near(
            card.height(),
            ACTION_HEIGHT + (PADDING + BORDER) * 2.0
        ));
        assert!(is_near(button.height(), ACTION_HEIGHT));
        assert!(is_near(button.right(), card.right() - PADDING - BORDER));
        assert!(is_near(button.center().y, card.center().y));
        let [background, _] = LIGHT_ACTION;
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Rect(rect)
                if rect.rect == button
                    && rect.fill == Color32::from_rgb(background[0], background[1], background[2])
        )));
        assert!(toasts.take_actions().is_empty());
        action_frame(
            &mut toasts,
            &context,
            now,
            vec![
                egui::Event::PointerMoved(button.center()),
                egui::Event::PointerButton {
                    pos: button.center(),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        assert!(toasts.swipe_owner.is_none());
        assert!(toasts.entries[0].dismissed_at.is_none());
        action_frame(
            &mut toasts,
            &context,
            now,
            vec![egui::Event::PointerButton {
                pos: button.center(),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert_eq!(toasts.take_actions(), vec![Probe::Retry]);
        assert_eq!(toasts.entries[0].dismissed_at, Some(now));
        action_frame(&mut toasts, &context, now + EXIT_DURATION / 2, Vec::new());
        assert!(toasts.take_actions().is_empty());
        action_frame(&mut toasts, &context, now + EXIT_DURATION, Vec::new());
        assert!(toasts.entries.is_empty());
        assert!(toasts.take_actions().is_empty());
    }

    #[test]
    fn 일반_toast_액션이_없으면_기존_높이와_본문_너비를_유지한다() {
        let context = egui::Context::default();
        let now = Instant::now();
        let mut toasts = Toasts::<Probe>::with_actions().unwrap();
        toasts.info("Synthetic info".into(), now);
        action_frame(&mut toasts, &context, now, Vec::new());
        let output = action_frame(
            &mut toasts,
            &context,
            now + Duration::from_millis(400),
            Vec::new(),
        );
        assert!(toasts.hit_targets[0].action.is_none());
        let icon_space = ICON_WRAPPER + ICON_LEFT_MARGIN + ICON_RIGHT_MARGIN + CONTENT_GAP;
        let title = layout(
            &context,
            "Synthetic info".into(),
            Color32::WHITE,
            WIDTH - (PADDING + BORDER) * 2.0 - icon_space,
            TITLE_LINE_HEIGHT,
        );
        assert!(
            (toasts.hit_targets[0].card.height()
                - (title.size().y.max(ICON_WRAPPER) + (PADDING + BORDER) * 2.0))
                .abs()
                < 0.01
        );
        let texture = toasts.assets.textures[Kind::Info.index()].id();
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Rect(rect)
                if rect
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == texture)
        )));
        let (background, _, _) = colors(ThemeType::Light, Kind::Info);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Rect(rect) if rect.fill == background
        )));
    }
}
