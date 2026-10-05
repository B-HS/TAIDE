use std::collections::BTreeMap;

use iced::widget::{
    Column, Row, button, checkbox, column, container, pick_list, row, scrollable, space, text,
    text_input,
};
use iced::{Element, Fill, Size, Subscription, Task, Theme, window};
use taide_native_shell_spike::{Locale, Pane, ShellFixture, TREE_ROW_COUNT};

const MAIN_SIZE: Size = Size::new(1100.0, 760.0);
const AUXILIARY_SIZE: Size = Size::new(760.0, 560.0);
const TREE_WIDTH: f32 = 220.0;
const TREE_ROW_HEIGHT: f32 = 22.0;
const VIRTUAL_ROW_WINDOW: usize = 50;
const CONTENT_SPACING: f32 = 8.0;
const INPUT_HEIGHT: f32 = 150.0;

fn main() -> iced::Result {
    iced::daemon(ShellSpike::new, ShellSpike::update, ShellSpike::view)
        .title(ShellSpike::title)
        .theme(ShellSpike::theme)
        .subscription(ShellSpike::subscription)
        .run()
}

#[derive(Clone, Debug)]
enum Message {
    Opened(window::Id, Pane),
    Closed(window::Id),
    SelectRow(usize),
    SelectTab(usize, Pane),
    MoveTab(usize, Pane),
    Input(String),
    Theme(bool),
    Locale(String),
    ChooseFile,
    Copy,
    Scroll(window::Id, scrollable::Viewport),
    InputMethod(iced::advanced::input_method::Event),
    FileDropped,
}

struct ShellSpike {
    fixture: ShellFixture,
    windows: BTreeMap<window::Id, Pane>,
    row_offsets: BTreeMap<window::Id, usize>,
    ime_preedit_count: usize,
    ime_commit_count: usize,
    drop_count: usize,
    dialog_count: usize,
}

impl ShellSpike {
    fn new() -> (Self, Task<Message>) {
        let (_, main) = window::open(window::Settings {
            size: MAIN_SIZE,
            ..Default::default()
        });
        let (_, auxiliary) = window::open(window::Settings {
            size: AUXILIARY_SIZE,
            ..Default::default()
        });
        (
            Self {
                fixture: ShellFixture::default(),
                windows: BTreeMap::new(),
                row_offsets: BTreeMap::new(),
                ime_preedit_count: 0,
                ime_commit_count: 0,
                drop_count: 0,
                dialog_count: 0,
            },
            Task::batch([
                main.map(|id| Message::Opened(id, Pane::Main)),
                auxiliary.map(|id| Message::Opened(id, Pane::Auxiliary)),
            ]),
        )
    }

    fn title(&self, id: window::Id) -> String {
        match self.windows.get(&id) {
            Some(Pane::Auxiliary) => "TAIDE M8 iced auxiliary".to_string(),
            _ => "TAIDE M8 iced spike".to_string(),
        }
    }

    fn theme(&self, _id: window::Id) -> Theme {
        if self.fixture.is_dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::close_events().map(Message::Closed),
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::InputMethod(event) => Some(Message::InputMethod(event)),
                iced::Event::Window(window::Event::FileDropped(_)) => Some(Message::FileDropped),
                _ => None,
            }),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id, pane) => {
                self.windows.insert(id, pane);
            }
            Message::Closed(id) => {
                if self.windows.remove(&id) == Some(Pane::Auxiliary) {
                    self.fixture.return_auxiliary_tabs();
                }
                self.row_offsets.remove(&id);
                if self.windows.is_empty() {
                    return iced::exit();
                }
            }
            Message::SelectRow(row) => self.fixture.selected_row = Some(row),
            Message::SelectTab(tab, pane) => match pane {
                Pane::Main => self.fixture.active_main = Some(tab),
                Pane::Auxiliary => self.fixture.active_auxiliary = Some(tab),
            },
            Message::MoveTab(tab, pane) => {
                self.fixture.move_tab(tab, pane);
            }
            Message::Input(value) => self.fixture.input = value,
            Message::Theme(value) => self.fixture.is_dark = value,
            Message::Locale(value) => {
                self.fixture.locale = match value.as_str() {
                    "한국어" => Locale::Korean,
                    "日本語" => Locale::Japanese,
                    _ => Locale::English,
                }
            }
            Message::ChooseFile => {
                if rfd::FileDialog::new()
                    .set_title("TAIDE M8 iced isolated probe")
                    .pick_file()
                    .is_some()
                {
                    self.dialog_count += 1;
                }
            }
            Message::Copy => return iced::clipboard::write(self.fixture.input.clone()),
            Message::Scroll(id, viewport) => {
                let offset = (viewport.absolute_offset().y / TREE_ROW_HEIGHT).floor() as usize;
                self.row_offsets.insert(
                    id,
                    offset.min(TREE_ROW_COUNT.saturating_sub(VIRTUAL_ROW_WINDOW)),
                );
            }
            Message::InputMethod(event) => match event {
                iced::advanced::input_method::Event::Preedit(..) => self.ime_preedit_count += 1,
                iced::advanced::input_method::Event::Commit(_) => self.ime_commit_count += 1,
                _ => {}
            },
            Message::FileDropped => self.drop_count += 1,
        }
        Task::none()
    }

    fn view(&self, id: window::Id) -> Element<'_, Message> {
        let pane = self.windows.get(&id).copied().unwrap_or(Pane::Main);
        let locale = match self.fixture.locale {
            Locale::Korean => "한국어",
            Locale::Japanese => "日本語",
            Locale::English => "English",
        }
        .to_string();
        let toolbar = row![
            text("TAIDE M8 / iced"),
            button(self.fixture.locale.label("dialog")).on_press(Message::ChooseFile),
            checkbox(self.fixture.is_dark)
                .label(self.fixture.locale.label("theme"))
                .on_toggle(Message::Theme),
            pick_list(
                [
                    "English".to_string(),
                    "한국어".to_string(),
                    "日本語".to_string()
                ],
                Some(locale),
                Message::Locale
            ),
        ]
        .spacing(CONTENT_SPACING);
        let start = self.row_offsets.get(&id).copied().unwrap_or_default();
        let end = (start + VIRTUAL_ROW_WINDOW).min(TREE_ROW_COUNT);
        let rows = (start..end).map(|index| {
            button(text(format!("fixture-{index:05}.rs")))
                .padding(0)
                .width(Fill)
                .height(TREE_ROW_HEIGHT)
                .on_press(Message::SelectRow(index))
                .into()
        });
        let tree_content = column![
            space().height(start as f32 * TREE_ROW_HEIGHT),
            Column::with_children(rows),
            space().height((TREE_ROW_COUNT - end) as f32 * TREE_ROW_HEIGHT),
        ];
        let tree = column![
            text(self.fixture.locale.label("tree")),
            scrollable(tree_content)
                .height(Fill)
                .on_scroll(move |viewport| Message::Scroll(id, viewport)),
        ]
        .width(TREE_WIDTH);
        let tabs = self
            .fixture
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, owner)| **owner == pane)
            .map(|(index, _)| {
                button(text(format!("Tab {index:02}")))
                    .on_press(Message::SelectTab(index, pane))
                    .into()
            });
        let active = match pane {
            Pane::Main => self.fixture.active_main,
            Pane::Auxiliary => self.fixture.active_auxiliary,
        };
        let destination = match pane {
            Pane::Main => Pane::Auxiliary,
            Pane::Auxiliary => Pane::Main,
        };
        let move_button = button("Move active tab to other window")
            .on_press_maybe(active.map(|tab| Message::MoveTab(tab, destination)));
        let editor = column![
            scrollable(Row::with_children(tabs).spacing(CONTENT_SPACING)).direction(
                scrollable::Direction::Horizontal(scrollable::Scrollbar::new())
            ),
            move_button,
            text(self.fixture.locale.label("input")),
            container(text_input("IME input probe", &self.fixture.input).on_input(Message::Input))
                .height(INPUT_HEIGHT),
            button("Copy fixture text").on_press(Message::Copy),
            text("CJK: 한글 日本語 中文"),
        ]
        .spacing(CONTENT_SPACING)
        .width(Fill);
        let status = text(format!(
            "Isolated fixture | rows {} / {} | tabs {} | IME preedit {} / commit {} | drops {} | dialogs {}",
            end - start,
            TREE_ROW_COUNT,
            self.fixture.tabs.len(),
            self.ime_preedit_count,
            self.ime_commit_count,
            self.drop_count,
            self.dialog_count,
        ));
        container(
            column![
                toolbar,
                row![tree, editor].spacing(CONTENT_SPACING).height(Fill),
                status
            ]
            .spacing(CONTENT_SPACING),
        )
        .padding(CONTENT_SPACING)
        .into()
    }
}
