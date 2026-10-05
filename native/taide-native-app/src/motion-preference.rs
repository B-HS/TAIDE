use eframe::egui;

#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSNotificationCenter, NSNotificationName, NSObjectProtocol};

pub(crate) struct Preference {
    #[cfg(target_os = "macos")]
    _observer: Observer,
}

impl Preference {
    pub(crate) fn new(context: egui::Context) -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                _observer: Observer::new(
                    NSWorkspace::sharedWorkspace().notificationCenter(),
                    unsafe { NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification },
                    move || context.request_repaint(),
                ),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = context;
            Self {}
        }
    }

    pub(crate) fn current(&self) -> Option<bool> {
        #[cfg(target_os = "macos")]
        {
            Some(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion())
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }
}

#[cfg(target_os = "macos")]
struct Observer {
    center: Retained<NSNotificationCenter>,
    token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

#[cfg(target_os = "macos")]
impl Observer {
    fn new(
        center: Retained<NSNotificationCenter>,
        name: &NSNotificationName,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let callback = RcBlock::new(move |_| wake());
        let token = unsafe {
            center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &callback)
        };
        Self { center, token }
    }
}

#[cfg(target_os = "macos")]
impl Drop for Observer {
    fn drop(&mut self) {
        unsafe {
            self.center
                .removeObserver(AsRef::<AnyObject>::as_ref(&self.token))
        };
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use objc2_foundation::NSString;

    use super::*;

    #[test]
    fn native_motion_preference는_private_알림과_창별해제를_보존한다() {
        let center = NSNotificationCenter::new();
        let name = NSString::from_str("TAIDESyntheticMotionPreferenceChanged");
        let unrelated = NSString::from_str("TAIDESyntheticUnrelatedPreferenceChanged");
        let first = Arc::new(AtomicUsize::new(0));
        let second = Arc::new(AtomicUsize::new(0));
        let first_count = first.clone();
        let first_observer = Observer::new(center.clone(), &name, move || {
            first_count.fetch_add(1, Ordering::SeqCst);
        });
        let second_count = second.clone();
        let second_observer = Observer::new(center.clone(), &name, move || {
            second_count.fetch_add(1, Ordering::SeqCst);
        });
        unsafe { center.postNotificationName_object(&unrelated, None) };
        assert_eq!(first.load(Ordering::SeqCst), 0);
        assert_eq!(second.load(Ordering::SeqCst), 0);
        unsafe { center.postNotificationName_object(&name, None) };
        assert_eq!(first.load(Ordering::SeqCst), 1);
        assert_eq!(second.load(Ordering::SeqCst), 1);
        drop(first_observer);
        unsafe { center.postNotificationName_object(&name, None) };
        assert_eq!(first.load(Ordering::SeqCst), 1);
        assert_eq!(second.load(Ordering::SeqCst), 2);
        drop(second_observer);
        unsafe { center.postNotificationName_object(&name, None) };
        assert_eq!(first.load(Ordering::SeqCst), 1);
        assert_eq!(second.load(Ordering::SeqCst), 2);
    }
}
