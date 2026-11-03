use std::path::Path;
use std::process::Command;

const AUTHOR: &str = "Synthetic Test";
const AUTHOR_EMAIL: &str = "synthetic@example.invalid";
const INITIAL_MESSAGE: &str = "test(fixture): initial";

/// Runs the git CLI inside a test repository with the user's global and system configuration,
/// hooks and file system monitor out of the way.
pub(crate) fn git(root: &Path, empty: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("-c")
        .arg(format!("core.hooksPath={}", empty.display()))
        .arg("-c")
        .arg("core.fsmonitor=false")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "synthetic git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

/// An empty repository on `main` whose local configuration pins identity, hooks, signing,
/// credentials and protocols (`remote-git-tests.rs` fixture).
pub(crate) fn init(root: &Path, empty: &Path) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::create_dir_all(empty).unwrap();
    let template = empty.to_str().unwrap();
    git(
        root,
        empty,
        &["init", "--initial-branch=main", "--template", template],
    );
    for (key, value) in [
        ("user.name", AUTHOR),
        ("user.email", AUTHOR_EMAIL),
        ("core.hooksPath", template),
        ("init.templateDir", template),
        ("core.fsmonitor", "false"),
        ("core.attributesFile", "/dev/null"),
        ("core.excludesFile", "/dev/null"),
        ("commit.gpgSign", "false"),
        ("tag.gpgSign", "false"),
        ("credential.helper", ""),
        ("credential.interactive", "false"),
        ("protocol.allow", "never"),
        ("protocol.file.allow", "always"),
        ("pull.rebase", "false"),
        ("push.default", "upstream"),
    ] {
        git(root, empty, &["config", "--local", key, value]);
    }
}

/// [`init`] plus one commit holding `files`.
pub(crate) fn repository(root: &Path, empty: &Path, files: &[(&str, &str)]) {
    init(root, empty);
    for (name, text) in files {
        std::fs::write(root.join(name), text).unwrap();
        git(root, empty, &["add", "--", name]);
    }
    git(root, empty, &["commit", "-m", INITIAL_MESSAGE]);
}
