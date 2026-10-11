use std::time::Duration;

use egui::{
    Button, Event, EventFilter, Id, Key, Popup, PopupCloseBehavior, Rect, Response, Sense,
    SetOpenCommand, Ui,
};
use taide_native_editor::document::EditorError;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

use super::{Action, Entry, Group, Item, Session, right_click};
use crate::editor_geometry::EditorGeometry;

const SEARCH_TIMEOUT: Duration = Duration::from_secs(1);
const FOCUS_FILTER: EventFilter = EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

pub struct Request<'a> {
    pub source: ViewId,
    pub owner: ViewId,
    pub response: &'a Response,
    pub geometry: &'a EditorGeometry,
    pub keyboard_open: bool,
    pub excluded: &'a [Rect],
}

#[derive(Clone)]
pub struct Invocation {
    pub session: Session,
    pub action: Action,
}

#[derive(Clone, Default)]
pub struct Geometry {
    pub rect: Option<Rect>,
    pub items: Vec<(Action, Rect)>,
    pub submenus: Vec<Rect>,
}

#[derive(Default)]
pub struct Output {
    pub invocation: Option<Invocation>,
    pub focus_ids: Vec<Id>,
    pub opened: bool,
    pub selection_changed: bool,
    pub geometry: Geometry,
}

#[derive(Clone, Default)]
struct Search {
    query: String,
    expires: f64,
    frame: Option<u64>,
    before: Option<(String, f64)>,
}

#[derive(Default)]
struct Navigation {
    selected: Option<Id>,
    open_submenu: Option<Id>,
}

#[derive(Clone, Default)]
struct State {
    session: Option<Session>,
    opened_frame: Option<u64>,
    pending_focus: Option<Id>,
    root_ids: Vec<Id>,
    submenu_ids: Vec<Id>,
    submenu_focus_pending: bool,
    root_search: Search,
    submenu_search: Search,
}

#[derive(Clone, Copy)]
struct KeyboardRequest {
    frame: u64,
    index: usize,
}

pub fn request_keyboard(context: &egui::Context, body: Id, index: usize) {
    let frame = context.cumulative_frame_nr();
    context.data_mut(|data| {
        data.insert_temp(
            body.with("editor-context-menu-keyboard-request"),
            KeyboardRequest { frame, index },
        );
    });
}

pub fn owns_event(context: &egui::Context, body: Id, event_index: usize) -> bool {
    if event_index == usize::MAX {
        return false;
    }
    let owner = body.with("editor-context-menu-owner");
    let state = context.data(|data| {
        data.get_temp::<State>(body.with("editor-context-menu-state"))
            .unwrap_or_default()
    });
    let mut focused = context.keyboard_focus_before_events();
    let events = context.input(|input| input.raw.events.clone());
    let requested = context.data(|data| {
        data.get_temp::<KeyboardRequest>(body.with("editor-context-menu-keyboard-request"))
    });
    if let Some(requested) = requested
        && requested.frame == context.cumulative_frame_nr()
        && requested.index < event_index
        && !events
            .iter()
            .skip(requested.index + 1)
            .take(event_index - requested.index)
            .any(|event| {
                matches!(
                    event,
                    Event::PointerButton { pressed: true, .. }
                        | Event::WindowFocused(false)
                        | Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                            action: egui::accesskit::Action::Focus,
                            ..
                        })
                )
            })
    {
        return true;
    }
    for (index, event) in events.iter().enumerate().take(event_index + 1) {
        if let Some(node) = context.keyboard_focus_request_at(index) {
            focused = context.previous_keyboard_target(node);
        } else if matches!(
            event,
            Event::PointerButton { pressed: true, .. } | Event::WindowFocused(false)
        ) {
            focused = None;
        }
        if index < event_index
            && let Some(node) = context.keyboard_focus_after_request_at(index)
        {
            focused = context.previous_keyboard_target(node);
        }
    }
    focused.is_some_and(|id| {
        id == owner
            || state.root_ids.contains(&id)
            || state.submenu_ids.contains(&id)
            || state
                .root_ids
                .iter()
                .any(|trigger| trigger.with("editor-submenu-owner") == id)
    })
}

fn item(ui: &mut Ui, item: &Item, output: &mut Output) -> Response {
    let mut button = Button::new(&item.label).space_on_release(false);
    if let Some(shortcut) = &item.shortcut {
        button = button.shortcut_text(shortcut);
    }
    let response = ui.add(button);
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::MenuItem)
    });
    output.focus_ids.push(response.id);
    output.geometry.items.push((item.action, response.rect));
    response
}

fn menu_owner(ui: &mut Ui, id: Id, output: &mut Output) -> Response {
    let response = ui.interact(ui.max_rect(), id, Sense::focusable_noninteractive());
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::Other));
    response
        .ctx
        .accesskit_node_builder(id, |node| node.set_role(egui::accesskit::Role::Menu));
    ui.memory_mut(|memory| memory.set_focus_lock_filter(id, FOCUS_FILTER));
    output.focus_ids.push(id);
    response
}

fn consume_key(ui: &Ui, keys: &[Key]) -> bool {
    ui.input_mut(|input| {
        let key = input.events.iter().find_map(|event| match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if keys.contains(key) => Some((*modifiers, *key)),
            _ => None,
        });
        key.is_some_and(|(modifiers, key)| input.consume_key(modifiers, key))
    })
}

fn navigate(
    ui: &Ui,
    owner: Id,
    items: &[(Id, String)],
    submenus: &[Id],
    search: &mut Search,
    initial: bool,
) -> Navigation {
    let before = ui.ctx().keyboard_focus_before_events();
    let current = ui.memory(|memory| memory.focused());
    let mut focused = before
        .filter(|id| *id == owner || items.iter().any(|(item, _)| item == id))
        .or_else(|| current.filter(|id| *id == owner || items.iter().any(|(item, _)| item == id)));
    if initial && focused.is_none() {
        focused = Some(owner);
    }
    if focused.is_none() || items.is_empty() {
        return Navigation::default();
    }
    let frame = ui.ctx().cumulative_frame_nr();
    if search.frame == Some(frame) {
        if let Some((query, expires)) = &search.before {
            search.query.clone_from(query);
            search.expires = *expires;
        }
    } else {
        search.frame = Some(frame);
        search.before = Some((search.query.clone(), search.expires));
    }
    let time = ui.input(|input| input.time);
    if time >= search.expires {
        search.query.clear();
    }
    let mut events = ui.input(|input| input.events.clone()).into_iter();
    let mut remaining = Vec::new();
    let mut navigation = Navigation::default();
    let mut move_focus = false;
    while let Some(event) = events.next() {
        let index = focused.and_then(|id| items.iter().position(|(item, _)| *item == id));
        match &event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !modifiers.ctrl && !modifiers.alt && !modifiers.command && !modifiers.mac_cmd => {
                let next = match key {
                    Key::ArrowDown => Some(index.map_or(0, |index| (index + 1) % items.len())),
                    Key::ArrowUp => Some(index.map_or(items.len() - 1, |index| {
                        (index + items.len() - 1) % items.len()
                    })),
                    Key::Home => Some(0),
                    Key::End => Some(items.len() - 1),
                    Key::ArrowRight if focused.is_some_and(|id| submenus.contains(&id)) => {
                        navigation.open_submenu = focused;
                        remaining.extend(events);
                        break;
                    }
                    Key::Enter | Key::Space
                        if index.is_some() && (*key == Key::Enter || search.query.is_empty()) =>
                    {
                        if focused.is_some_and(|id| submenus.contains(&id)) {
                            navigation.open_submenu = focused;
                        } else {
                            navigation.selected = focused;
                        }
                        remaining.extend(events);
                        break;
                    }
                    _ => {
                        remaining.push(event);
                        continue;
                    }
                };
                if let Some(next) = next {
                    focused = Some(items[next].0);
                    move_focus = true;
                }
            }
            Event::Text(text) => {
                search.query.push_str(text);
                search.expires = time + SEARCH_TIMEOUT.as_secs_f64();
                ui.ctx().request_repaint_after(SEARCH_TIMEOUT);
                let lower = search.query.to_lowercase();
                let first = lower.chars().next();
                let repeated =
                    first.is_some_and(|first| lower.chars().all(|character| character == first));
                let query = if repeated {
                    first.unwrap().to_string()
                } else {
                    lower
                };
                let start = index.map_or(0, |index| {
                    if query.chars().count() == 1 {
                        (index + 1) % items.len()
                    } else {
                        index
                    }
                });
                if let Some(next) = (0..items.len())
                    .map(|offset| (start + offset) % items.len())
                    .find(|index| items[*index].1.trim().to_lowercase().starts_with(&query))
                {
                    focused = Some(items[next].0);
                    move_focus = true;
                }
            }
            _ => remaining.push(event),
        }
    }
    ui.input_mut(|input| input.events = remaining);
    if move_focus && let Some(focused) = focused {
        ui.memory_mut(|memory| {
            memory.move_focus(egui::FocusDirection::None);
            memory.request_focus(focused);
        });
    }
    navigation
}

pub fn show(
    ui: &mut Ui,
    store: &mut EditorStore,
    request: Request<'_>,
    mut groups: impl FnMut(&EditorStore, ViewId) -> Vec<Group>,
) -> Result<Output, EditorError> {
    let Request {
        source,
        owner,
        response,
        geometry,
        keyboard_open,
        excluded,
    } = request;
    let state_id = response.id.with("editor-context-menu-state");
    let menu_owner_id = response.id.with("editor-context-menu-owner");
    let viewport = ui.ctx().viewport_id();
    let mut state = ui
        .ctx()
        .data_mut(|data| data.get_temp::<State>(state_id).unwrap_or_default());
    let popup = Popup::context_menu(response);
    let popup_id = popup.get_id();
    let mut output = Output::default();
    ui.ctx().register_pointer_keyboard_focus(response.id);
    ui.ctx()
        .register_context_menu_keyboard_focus(response.id, menu_owner_id);
    let enabled = ui.is_enabled() && response.enabled() && ui.input(|input| input.raw.focused);
    let frame = ui.ctx().cumulative_frame_nr();
    let keyboard_open = keyboard_open
        || ui.ctx().data(|data| {
            data.get_temp::<KeyboardRequest>(
                response.id.with("editor-context-menu-keyboard-request"),
            )
            .is_some_and(|request| request.frame == frame)
        });
    let pointer_open = enabled
        && response.contains_pointer()
        && ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary));
    let opening = if pointer_open {
        ui.input(|input| input.pointer.interact_pos())
            .filter(|position| !excluded.iter().any(|rect| rect.contains(*position)))
            .and_then(|position| {
                let document = store
                    .views()
                    .get(source)
                    .and_then(|view| store.documents().snapshot(view.document).ok())?;
                geometry
                    .context_menu_byte_at(position, document.rope.len_bytes())
                    .map(|head| (position, Some(head)))
            })
    } else if enabled && keyboard_open && state.session.is_none() {
        store
            .views()
            .get(source)
            .and_then(|view| view.selection.selections.get(view.selection.primary))
            .and_then(|selection| geometry.caret_rect(selection.head))
            .map(|caret| (caret.left_bottom(), None))
    } else {
        None
    };
    let was_open = state.session.is_some();
    if let Some((anchor, head)) = opening {
        ui.ctx().data_mut(|data| {
            data.remove::<KeyboardRequest>(
                response.id.with("editor-context-menu-keyboard-request"),
            );
        });
        let before = store
            .views()
            .get(source)
            .ok_or(EditorError::NotFound)?
            .selection
            .clone();
        if let Some(head) = head {
            right_click(store, source, head)?;
        }
        output.selection_changed = store
            .views()
            .get(source)
            .ok_or(EditorError::NotFound)?
            .selection
            != before;
        state = State {
            session: Some(Session::open(
                store,
                source,
                owner,
                viewport,
                anchor,
                groups(store, source),
            )?),
            opened_frame: Some(ui.ctx().cumulative_frame_nr()),
            ..State::default()
        };
    }
    output.opened = state.opened_frame == Some(ui.ctx().cumulative_frame_nr());
    let invalid = !enabled
        || pointer_open && opening.is_none()
        || state.session.as_ref().is_some_and(|session| {
            !session.matches(store, source, owner, viewport)
                || session.groups != groups(store, source)
        });
    let body_pointer_click = response.clicked()
        && ui.input(|input| {
            input.raw.events.iter().any(|event| {
                matches!(
                    event,
                    Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    }
                )
            })
        });
    if invalid || body_pointer_click || state.session.is_none() {
        Popup::close_id(ui.ctx(), popup_id);
        state = State::default();
        if enabled && (was_open && !pointer_open || pointer_open && opening.is_none()) {
            response.request_focus();
        }
        ui.ctx().data_mut(|data| data.insert_temp(state_id, state));
        return Ok(output);
    }
    let session = state.session.clone().ok_or(EditorError::NotFound)?;
    let popup = popup
        .open_memory(opening.is_some().then_some(SetOpenCommand::Bool(true)))
        .at_position(session.anchor)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside);
    let mut action = None;
    let entries = session
        .groups
        .iter()
        .flat_map(|group| &group.entries)
        .collect::<Vec<_>>();
    let mut open_submenu = None;
    let root_prepared = state.root_ids.len() == entries.len();
    if root_prepared {
        let root_items = state
            .root_ids
            .iter()
            .copied()
            .zip(entries.iter().map(|entry| match entry {
                Entry::Item(item) => item.label.clone(),
                Entry::Submenu { label, .. } => label.clone(),
            }))
            .collect::<Vec<_>>();
        let submenus = state
            .root_ids
            .iter()
            .copied()
            .zip(&entries)
            .filter_map(|(id, entry)| matches!(entry, Entry::Submenu { .. }).then_some(id))
            .collect::<Vec<_>>();
        let navigation = navigate(
            ui,
            menu_owner_id,
            &root_items,
            &submenus,
            &mut state.root_search,
            output.opened,
        );
        open_submenu = navigation.open_submenu;
        if let Some(selected) = navigation.selected
            && let Some(index) = state.root_ids.iter().position(|id| *id == selected)
            && let Entry::Item(item) = entries[index]
        {
            action = Some(item.action);
        }
    }
    let mut sizing = false;
    let shown = popup.show(|menu_ui| {
        sizing = menu_ui.is_sizing_pass();
        let owner_response = menu_owner(menu_ui, menu_owner_id, &mut output);
        if opening.is_some() {
            owner_response.request_focus();
        }
        let mut root_items = Vec::new();
        for (group_index, group) in session.groups.iter().enumerate() {
            if group_index > 0 {
                let separator = menu_ui.separator();
                separator.ctx.accesskit_node_builder(separator.id, |node| {
                    node.set_role(egui::accesskit::Role::Splitter)
                });
            }
            for entry in &group.entries {
                match entry {
                    Entry::Item(menu_item) => {
                        let response = item(menu_ui, menu_item, &mut output);
                        root_items.push((response.id, menu_item.label.clone()));
                        if response.clicked() {
                            action = Some(menu_item.action);
                        }
                    }
                    Entry::Submenu { label, items } => {
                        let trigger_id = menu_ui.next_auto_id();
                        let submenu_owner = trigger_id.with("editor-submenu-owner");
                        let focus = menu_ui.ctx().keyboard_focus_before_events();
                        let mut open_override = None;
                        if open_submenu == Some(trigger_id)
                            || focus == Some(trigger_id)
                                && consume_key(menu_ui, &[Key::ArrowRight, Key::Enter, Key::Space])
                        {
                            open_override = Some(true);
                            state.submenu_focus_pending = true;
                        }
                        if focus.is_some_and(|id| {
                            id == submenu_owner || state.submenu_ids.contains(&id)
                        }) && consume_key(menu_ui, &[Key::ArrowLeft, Key::Escape])
                        {
                            open_override = Some(false);
                            menu_ui.memory_mut(|memory| memory.request_focus(trigger_id));
                        }
                        let mut submenu_button = egui::containers::menu::SubMenuButton::new(label);
                        submenu_button.sub_menu =
                            submenu_button.sub_menu.open_override(open_override);
                        let (trigger, shown) = submenu_button.ui(menu_ui, |submenu_ui| {
                            let owner = menu_owner(submenu_ui, submenu_owner, &mut output);
                            let prepared = state.submenu_ids.len() == items.len();
                            if prepared {
                                let existing = state
                                    .submenu_ids
                                    .iter()
                                    .copied()
                                    .zip(items.iter().map(|item| item.label.clone()))
                                    .collect::<Vec<_>>();
                                if let Some(selected) = navigate(
                                    submenu_ui,
                                    submenu_owner,
                                    &existing,
                                    &[],
                                    &mut state.submenu_search,
                                    false,
                                )
                                .selected
                                    && let Some(index) =
                                        state.submenu_ids.iter().position(|id| *id == selected)
                                {
                                    action = Some(items[index].action);
                                }
                            }
                            let mut shown_items = Vec::new();
                            for menu_item in items {
                                let response = item(submenu_ui, menu_item, &mut output);
                                shown_items.push((response.id, menu_item.label.clone()));
                                if response.clicked() {
                                    action = Some(menu_item.action);
                                }
                            }
                            if state.submenu_focus_pending {
                                state.submenu_focus_pending = false;
                                if let Some((id, _)) = shown_items.first() {
                                    submenu_ui.memory_mut(|memory| memory.request_focus(*id));
                                } else {
                                    owner.request_focus();
                                }
                            }
                            if !prepared
                                && let Some(selected) = navigate(
                                    submenu_ui,
                                    submenu_owner,
                                    &shown_items,
                                    &[],
                                    &mut state.submenu_search,
                                    true,
                                )
                                .selected
                                && let Some(index) =
                                    shown_items.iter().position(|(id, _)| *id == selected)
                            {
                                action = Some(items[index].action);
                            }
                            state.submenu_ids = shown_items.iter().map(|(id, _)| *id).collect();
                        });
                        if shown.is_none() {
                            state.submenu_ids.clear();
                            state.submenu_search = Search::default();
                        }
                        trigger.ctx.accesskit_node_builder(trigger.id, |node| {
                            node.set_role(egui::accesskit::Role::MenuItem);
                            node.set_has_popup(egui::accesskit::HasPopup::Menu);
                        });
                        output.focus_ids.push(trigger.id);
                        output.geometry.submenus.push(trigger.rect);
                        root_items.push((trigger.id, label.clone()));
                    }
                }
            }
        }
        if !root_prepared
            && let Some(selected) = navigate(
                menu_ui,
                menu_owner_id,
                &root_items,
                &[],
                &mut state.root_search,
                output.opened,
            )
            .selected
        {
            for (index, entry) in session
                .groups
                .iter()
                .flat_map(|group| &group.entries)
                .enumerate()
            {
                if root_items[index].0 == selected
                    && let Entry::Item(item) = entry
                {
                    action = Some(item.action);
                }
            }
        }
        state.root_ids = root_items.iter().map(|(id, _)| *id).collect();
        if !sizing && let Some(id) = state.pending_focus.take() {
            menu_ui.memory_mut(|memory| memory.request_focus(id));
        }
        if sizing {
            state.pending_focus = menu_ui
                .memory(|memory| memory.focused())
                .filter(|id| output.focus_ids.contains(id))
                .or(Some(menu_owner_id));
        }
        let owns_root = menu_ui
            .memory(|memory| memory.focused())
            .is_some_and(|id| id == menu_owner_id || state.root_ids.contains(&id));
        if action.is_some() || owns_root && consume_key(menu_ui, &[Key::Escape]) {
            menu_ui.close();
        }
    });
    if let Some(shown) = shown {
        output.geometry.rect = Some(shown.response.rect);
    }
    if sizing && action.is_none() {
        ui.ctx()
            .request_discard("Editor context menu initial size is now available");
    }
    if let Some(action) = action
        && session.matches(store, source, owner, viewport)
    {
        output.invocation = Some(Invocation { session, action });
        Popup::close_id(ui.ctx(), popup_id);
    }
    if !Popup::is_id_open(ui.ctx(), popup_id) {
        state = State::default();
        response.request_focus();
    }
    ui.ctx().data_mut(|data| data.insert_temp(state_id, state));
    Ok(output)
}
