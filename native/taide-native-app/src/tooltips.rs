#[cfg(not(test))]
pub use taide_native_ui::tooltips::{Appearance, Provider, Trigger};

#[cfg(test)]
use eframe::egui;
#[cfg(test)]
use taide_native_ui::tooltip_trigger;

#[cfg(test)]
pub use compatibility::{Appearance, Provider, Trigger};

#[cfg(test)]
mod compatibility {
    include!("../../taide-native-ui/src/tooltips.rs");

    mod theme_tests {
        include!("theme-tooltip-tests.rs");
    }

    mod icon_tests {
        include!("icon-tooltip-tests.rs");
    }

    mod preview_tests {
        include!("preview-tooltip-tests.rs");
    }

    mod placement_tests {
        include!("tooltip-placement-tests.rs");
    }

    mod motion_tests {
        include!("tooltip-motion-tests.rs");
    }

    mod hit_tests {
        include!("tooltip-hit-tests.rs");
    }

    mod controlled_tests {
        include!("tooltip-controlled-tests.rs");
    }

    mod key_tests {
        include!("tooltip-key-tests.rs");
    }

    mod scroll_tests {
        include!("tooltip-scroll-tests.rs");
    }

    mod modal_tests {
        include!("tooltip-modal-tests.rs");
    }
}
