/// The user's home directory as the environment reports it, or `None` when neither variable is
/// set. `HOME` on unix, `USERPROFILE` on Windows — read from the environment rather than a
/// platform crate so this module stays dependency-free and testable by injection ([`expand_home`]
/// takes the home as an argument for exactly that reason).
pub fn home_dir_env() -> Option<String> {
    select_home_dir(std::env::var("HOME").ok(), std::env::var("USERPROFILE").ok())
}

/// The precedence [`home_dir_env`] applies, split out so it can be unit-tested directly:
/// `set_var`/`remove_var` in a test would race every other test that reads `HOME` concurrently
/// (`domain::project::service`'s `~` expansion test does, on the same test binary).
///
/// `HOME` wins when it is set at all — including when it is set to the empty string, which stays
/// the caller's problem to notice rather than a silent switch to a different account's profile.
/// `USERPROFILE` is the Windows fallback, and unix shells that unset `HOME` land on it too if the
/// process happens to carry one.
fn select_home_dir(home: Option<String>, user_profile: Option<String>) -> Option<String> {
    home.or(user_profile)
}

/// Expands a leading `~` against `home`, but only in the two forms where `~` actually stands for
/// that home directory: `~` alone, and a `~/…` prefix.
///
/// `~user` names a *different* account's home, which this has no way to resolve — pasting `home` in
/// front of the remainder turned `~alice/notes.md` into `/Users/me` + `alice/notes.md`, a path that
/// silently pointed somewhere else — so that form, like every path that does not start with `~`, is
/// returned untouched and left to fail resolution honestly. `home` is `None` when the environment
/// has no home variable, which resolves the same way.
///
/// Shared infra rather than one domain's private helper because `~` is a *shell* convention no
/// filesystem call honors: every surface that accepts a path the user typed by hand has to expand
/// it itself or silently fail to find the file. Two do today — the terminal's path-link resolver
/// (`domain::terminal::service::resolve_terminal_path`) and "Open by path…"
/// (`domain::project::service::open_project`).
pub fn expand_home(path: &str, home: Option<&str>) -> String {
    let Some(rest) = path.strip_prefix('~') else {
        return path.to_string();
    };

    if !rest.is_empty() && !rest.starts_with('/') {
        return path.to_string();
    }

    match home {
        Some(home) => format!("{home}{rest}"),
        None => path.to_string(),
    }
}

/// [`expand_home`] against the environment's own home directory — the form every production caller
/// wants, kept separate so tests can still inject a fixed home.
pub fn expand_home_from_env(path: &str) -> String {
    expand_home(path, home_dir_env().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_HOME: &str = "/Users/tester";

    #[test]
    fn 물결_단독과_슬래시_접두만_홈으로_확장된다() {
        assert_eq!(expand_home("~", Some(TEST_HOME)), TEST_HOME);
        assert_eq!(expand_home("~/src/main.rs", Some(TEST_HOME)), format!("{TEST_HOME}/src/main.rs"));
    }

    #[test]
    fn 다른_사용자의_홈_표기는_원문_그대로_남는다() {
        assert_eq!(expand_home("~alice/notes.md", Some(TEST_HOME)), "~alice/notes.md");
        assert_eq!(expand_home("~alice", Some(TEST_HOME)), "~alice");
    }

    #[test]
    fn home_이_없으면_물결을_확장하지_않는다() {
        assert_eq!(expand_home("~/src/main.rs", None), "~/src/main.rs");
        assert_eq!(expand_home("~", None), "~");
    }

    #[test]
    fn home이_없으면_user_profile로_폴백하고_둘_다_없으면_none이다() {
        assert_eq!(
            select_home_dir(Some(TEST_HOME.to_string()), Some("C:\\Users\\other".to_string())),
            Some(TEST_HOME.to_string())
        );
        assert_eq!(
            select_home_dir(None, Some("C:\\Users\\tester".to_string())),
            Some("C:\\Users\\tester".to_string())
        );
        assert_eq!(select_home_dir(None, None), None);
    }

    #[test]
    fn 물결로_시작하지_않는_경로는_그대로_돌려준다() {
        assert_eq!(expand_home("/abs/path", Some(TEST_HOME)), "/abs/path");
        assert_eq!(expand_home("relative/path", Some(TEST_HOME)), "relative/path");
        assert_eq!(expand_home("", Some(TEST_HOME)), "");
    }
}
