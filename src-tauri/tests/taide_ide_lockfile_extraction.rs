use std::path::PathBuf;

use taide_ide::lockfile::{
    build_lockfile_content, lockfile_path, remove_lockfile, resolve_lockfile_dir, write_lockfile_atomic, IdeLockfileContent, IDE_NAME,
    IDE_TRANSPORT,
};

const TEST_PORT: u32 = 55_126;

#[test]
fn ide_lockfile의_경로와_wire는_독립_crate에서_동일하다() {
    let dir = resolve_lockfile_dir(Some("/custom/config"), Some("/home/user")).unwrap();
    assert_eq!(dir, PathBuf::from("/custom/config").join("ide"));
    assert_eq!(lockfile_path(&dir, TEST_PORT), dir.join(format!("{TEST_PORT}.lock")));

    let content = build_lockfile_content(1234, vec!["/repo".to_string()], "a".repeat(32));
    assert_eq!(content.ide_name, IDE_NAME);
    assert_eq!(content.transport, IDE_TRANSPORT);
    let model_content: taide_model::ide::IdeLockfileContent = content.clone();
    let facade_content: taide_lib::domain::ide::lockfile::IdeLockfileContent = model_content;
    assert_eq!(facade_content, content);
}

#[test]
fn ide_lockfile의_원자_쓰기와_삭제는_독립_crate에서_동일하다() {
    let dir = std::env::temp_dir().join(format!("taide-ide-lock-extraction-{}", uuid::Uuid::new_v4()));
    let content = build_lockfile_content(1234, vec!["/repo".to_string()], "a".repeat(32));

    write_lockfile_atomic(&dir, TEST_PORT, &content).unwrap();
    let raw = std::fs::read_to_string(lockfile_path(&dir, TEST_PORT)).unwrap();
    let decoded: IdeLockfileContent = serde_json::from_str(&raw).unwrap();
    assert_eq!(decoded, content);

    remove_lockfile(&dir, TEST_PORT).unwrap();
    assert!(!lockfile_path(&dir, TEST_PORT).exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
