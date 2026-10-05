use eframe::egui::ViewportId;
use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::layout::SplitDir;
use taide_model::project::ShellSlotTree;
use taide_model::tree::TreeEntryKind;
use taide_native_app::explorer::Explorer;
use taide_native_app::explorer_clipboard::{Entry, Mode};
use taide_native_app::explorer_clipboard_owners::ClipboardOwners;

fn leaf(slot: &ShellSlotId, project: &ProjectId) -> ShellSlotTree {
    ShellSlotTree::Leaf {
        slot_id: slot.clone(),
        project_id: project.clone(),
    }
}

#[test]
fn 클립보드는_슬롯수명과_회신owner를_따르고_같은슬롯의_프로젝트교체에는_유지한다() {
    let first_slot = ShellSlotId::new();
    let second_slot = ShellSlotId::new();
    let first_project = ProjectId::new();
    let second_project = ProjectId::new();
    let mut owners = ClipboardOwners::new(ViewportId::ROOT);
    let mut separate_window = ClipboardOwners::new(ViewportId::from_hash_of("synthetic auxiliary"));
    let mut tree = ShellSlotTree::Split {
        dir: SplitDir::Horizontal,
        children: vec![
            leaf(&first_slot, &first_project),
            leaf(&second_slot, &first_project),
        ],
        sizes: vec![1.0, 1.0],
    };
    owners.reconcile(Some(&tree));
    let first = owners.mount(&first_slot).unwrap();
    let second = owners.mount(&second_slot).unwrap();
    let window = separate_window.mount(&first_slot).unwrap();
    assert_ne!(first, second);
    assert_ne!(first, window);
    let cut = Entry {
        mode: Mode::Cut,
        path: "/synthetic/first.txt".into(),
        kind: TreeEntryKind::File,
    };
    let copy = Entry {
        mode: Mode::Copy,
        path: "/synthetic/second.txt".into(),
        kind: TreeEntryKind::File,
    };
    assert!(owners.replace(first, Some(cut.clone())));
    assert!(owners.clipboard(second).is_none());
    assert!(separate_window.clipboard(window).is_none());
    assert!(owners.replace(second, Some(copy.clone())));
    assert!(separate_window.replace(window, Some(copy.clone())));
    separate_window.paste_finished(
        Some(first),
        true,
        Some("/synthetic/wrong-window.txt".into()),
    );
    assert_eq!(separate_window.clipboard(window), Some(&copy));
    assert!(separate_window.take_reveal(window).is_none());
    for _ in 0..2 {
        owners.reconcile(Some(&tree));
    }
    assert_eq!(owners.mount(&first_slot).unwrap(), first);
    assert_eq!(owners.clipboard(first), Some(&cut));
    tree = ShellSlotTree::Split {
        dir: SplitDir::Horizontal,
        children: vec![
            leaf(&first_slot, &second_project),
            leaf(&second_slot, &first_project),
        ],
        sizes: vec![1.0, 1.0],
    };
    owners.reconcile(Some(&tree));
    assert_eq!(owners.mount(&first_slot).unwrap(), first);
    assert_eq!(owners.clipboard(first), Some(&cut));
    owners.paste_finished(Some(first), true, Some("/synthetic/moved.txt".into()));
    assert!(owners.clipboard(first).is_none());
    assert_eq!(owners.clipboard(second), Some(&copy));
    let mut explorer = Explorer::default();
    explorer.replace_clipboard(owners.clipboard(second).cloned());
    assert_eq!(explorer.clipboard(), Some(&copy));
    explorer.replace_clipboard(owners.clipboard(first).cloned());
    explorer.paste_finished(false, owners.take_reveal(first));
    assert_eq!(explorer.selected.as_deref(), Some("/synthetic/moved.txt"));
    assert!(owners.take_reveal(first).is_none());
    let survivor = leaf(&second_slot, &first_project);
    owners.reconcile(Some(&survivor));
    assert!(!owners.replace(first, Some(cut.clone())));
    let remounted = owners.mount(&first_slot).unwrap();
    assert_ne!(remounted, first);
    assert!(owners.clipboard(remounted).is_none());
    assert!(owners.replace(remounted, Some(copy.clone())));
    owners.paste_finished(Some(first), true, Some("/synthetic/stale.txt".into()));
    assert_eq!(owners.clipboard(remounted), Some(&copy));
    assert!(owners.take_reveal(remounted).is_none());
    assert!(owners.replace(remounted, Some(cut.clone())));
    assert!(owners.replace(remounted, Some(copy.clone())));
    owners.paste_finished(Some(remounted), true, None);
    assert!(owners.clipboard(remounted).is_none());
    assert_eq!(owners.clipboard(second), Some(&copy));
    owners.paste_finished(Some(second), false, Some("/synthetic/copied.txt".into()));
    assert_eq!(owners.clipboard(second), Some(&copy));
    assert_eq!(
        owners.take_reveal(second).as_deref(),
        Some("/synthetic/copied.txt")
    );
    owners.reconcile(None);
    owners.paste_finished(Some(second), true, Some("/synthetic/closed.txt".into()));
    assert!(!owners.replace(second, Some(copy)));
    assert!(owners.take_reveal(second).is_none());
    assert!(separate_window.clipboard(window).is_some());
}
