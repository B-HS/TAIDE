use taide_lsp::session::LspSessionRoots;

#[test]
fn lsp_root_참조_수는_기존_root와_새_root를_구별한다() {
    let roots = LspSessionRoots::new("/workspace/a".to_string());

    assert_eq!(roots.paths(), vec!["/workspace/a"]);
    assert!(!roots.acquire("/workspace/a".to_string()));
    assert!(roots.acquire("/workspace/b".to_string()));
    assert_eq!(roots.paths(), vec!["/workspace/a", "/workspace/b"]);

    let first_release = roots.release("/workspace/a");
    assert!(first_release.removed_root.is_none());
    assert!(first_release.has_remaining_roots);

    let second_release = roots.release("/workspace/a");
    assert_eq!(second_release.removed_root.as_deref(), Some("/workspace/a"));
    assert!(second_release.has_remaining_roots);
    assert_eq!(roots.paths(), vec!["/workspace/b"]);

    let last_release = roots.release("/workspace/b");
    assert_eq!(last_release.removed_root.as_deref(), Some("/workspace/b"));
    assert!(!last_release.has_remaining_roots);
}

#[test]
fn 알수없는_lsp_root_해제는_기존_참조를_보존한다() {
    let roots = LspSessionRoots::new("/workspace/a".to_string());

    let released = roots.release("/workspace/missing");

    assert!(released.removed_root.is_none());
    assert!(released.has_remaining_roots);
    assert_eq!(roots.paths(), vec!["/workspace/a"]);
}
