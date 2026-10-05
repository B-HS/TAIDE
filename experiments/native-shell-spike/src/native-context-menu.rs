use eframe::egui;
use muda::{ContextMenu, Menu, MenuItem};
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use taide_native_shell_spike::Pane;

pub fn keyboard_request_for_response(ui: &mut egui::Ui, response: &egui::Response) -> bool {
    let is_focused = response.has_focus();
    ui.input_mut(|input| take_keyboard_request(input, is_focused))
}

pub fn take_keyboard_request(input: &mut egui::InputState, is_focused: bool) -> bool {
    if !is_focused {
        return false;
    }
    let mut should_open = false;
    input.events.retain(|event| {
        let egui::Event::Key {
            key: egui::Key::F10,
            pressed: true,
            repeat,
            modifiers,
            ..
        } = event
        else {
            return true;
        };
        if *modifiers != egui::Modifiers::SHIFT {
            return true;
        }
        should_open |= !repeat;
        false
    });
    should_open
}

pub fn keyboard_position(
    rect: egui::Rect,
    clip: egui::Rect,
    zoom: f32,
) -> Result<muda::dpi::Position, &'static str> {
    if !rect.is_finite()
        || !clip.is_finite()
        || !clip.is_positive()
        || !zoom.is_finite()
        || zoom <= 0.0
    {
        return Err("Native menu has an invalid keyboard anchor");
    }
    let anchor = clip.clamp(rect.left_bottom());
    Ok(muda::dpi::LogicalPosition::new(
        f64::from(anchor.x) * f64::from(zoom),
        f64::from(anchor.y) * f64::from(zoom),
    )
    .into())
}

pub fn show_tab_menu(
    pane: Pane,
    label: &str,
    position: Option<muda::dpi::Position>,
) -> Result<bool, &'static str> {
    let main_thread = MainThreadMarker::new().ok_or("Native menu requires the main thread")?;
    let application = NSApplication::sharedApplication(main_thread);
    let window = application
        .keyWindow()
        .ok_or("Native menu has no key window")?;
    if window.title().to_string() != pane.window_title() {
        return Err("Native menu rejected a different key window");
    }
    let view = window
        .contentView()
        .ok_or("Native menu has no content view")?;
    if view
        .window()
        .is_none_or(|owner| owner.windowNumber() != window.windowNumber())
    {
        return Err("Native menu content view is detached from its window");
    }
    let menu = Menu::new();
    let item = MenuItem::new(label, true, None);
    menu.append(&item)
        .map_err(|_| "Native menu item could not be created")?;
    let pointer = std::ptr::from_ref(&*view).cast();
    Ok(unsafe { menu.show_context_menu_for_nsview(pointer, position) })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIP_SIZE: [f32; 2] = [100.0, 80.0];
    const TAB_ORIGIN: [f32; 2] = [10.0, 20.0];
    const TAB_SIZE: [f32; 2] = [40.0, 12.0];
    const ZOOM: f32 = 2.0;

    #[test]
    fn keyboard_ui_boundary는_egui_focus를_input_잠금_밖에서_읽고_한번_소비한다() {
        let context = egui::Context::default();
        let input = egui::RawInput {
            focused: true,
            events: vec![egui::Event::Key {
                key: egui::Key::F10,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| {
            let response = ui.button("synthetic tab");
            let other = ui.button("other synthetic tab");
            response.request_focus();
            assert!(!keyboard_request_for_response(ui, &other));
            assert!(keyboard_request_for_response(ui, &response));
            assert!(!keyboard_request_for_response(ui, &response));
        });
        output.textures_delta.clear();
    }

    #[test]
    fn keyboard_context은_focus_exact_modifiers_non_repeat와_logical_anchor를_검사한다() {
        let press = |modifiers, repeat| egui::Event::Key {
            key: egui::Key::F10,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers,
        };
        let mut input = egui::InputState::default();
        input.events.push(press(egui::Modifiers::SHIFT, false));
        assert!(!take_keyboard_request(&mut input, false));
        assert_eq!(input.events.len(), 1);
        assert!(take_keyboard_request(&mut input, true));
        assert!(input.events.is_empty());
        assert!(!take_keyboard_request(&mut input, true));
        input.events.push(press(egui::Modifiers::SHIFT, true));
        assert!(!take_keyboard_request(&mut input, true));
        assert!(input.events.is_empty());
        for modifiers in [
            egui::Modifiers::NONE,
            egui::Modifiers::ALT | egui::Modifiers::SHIFT,
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
        ] {
            input.events.push(press(modifiers, false));
        }
        assert!(!take_keyboard_request(&mut input, true));
        assert_eq!(input.events.len(), 3);
        let rect = egui::Rect::from_min_size(TAB_ORIGIN.into(), TAB_SIZE.into());
        let clip = egui::Rect::from_min_size(egui::Pos2::ZERO, CLIP_SIZE.into());
        let position = keyboard_position(rect, clip, ZOOM)
            .unwrap()
            .to_logical::<f64>(1.0);
        assert_eq!(position.x, 20.0);
        assert_eq!(position.y, 64.0);
        let offscreen = rect.translate(egui::vec2(200.0, 200.0));
        let clamped = keyboard_position(offscreen, clip, ZOOM)
            .unwrap()
            .to_logical::<f64>(1.0);
        assert_eq!(clamped.x, 200.0);
        assert_eq!(clamped.y, 160.0);
        assert!(keyboard_position(rect, clip, 0.0).is_err());
        assert!(keyboard_position(rect, clip, f32::NAN).is_err());
        assert!(keyboard_position(egui::Rect::NOTHING, clip, ZOOM).is_err());
        assert!(keyboard_position(rect, egui::Rect::NOTHING, ZOOM).is_err());
    }
}
