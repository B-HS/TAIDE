use std::path::{Path, PathBuf};

use crate::error::AppResult;
use crate::infra::shell_quote::posix_quote;

/// Marks a spawned shell as already carrying TAIDE's OSC 133 integration.
/// Planted into the child's env whenever [`prepare`] actually injects, and
/// checked (against *this* process's own env — see [`already_integrated`])
/// before deciding to inject again, so it doubles as both a dedup guard and a
/// manual opt-out: a user (or a future settings-driven toggle) who sets this
/// variable before TAIDE's own process starts gets no injection at all.
pub const SHELL_INTEGRATION_ENV_VAR: &str = "TAIDE_SHELL_INTEGRATION";
const SHELL_INTEGRATION_ENV_VALUE: &str = "1";

/// Carries the pre-injection `ZDOTDIR` value (if the user had one) into the
/// temp-ZDOTDIR zsh session so the injected `.zshrc` can restore it before
/// re-sourcing the user's real dotfiles.
const ORIGINAL_ZDOTDIR_ENV_VAR: &str = "TAIDE_ORIGINAL_ZDOTDIR";

/// A resolved shell we know how to inject OSC 133 support into, or a
/// recognized shell that deliberately gets none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
    Zsh,
    Bash,
    /// fish has shipped *native* OSC 133 prompt/command marking since
    /// fish-shell#10352, merged into the **fish 4.0** milestone — confirmed
    /// empirically here against fish 4.8.1 (`fish_prompt`/`fish_preexec`/
    /// `fish_postexec` already emit `133;A`/`133;B`/`133;C`/`133;D` with no
    /// rc injection at all). This variant exists so [`prepare`] treats "fish
    /// needs no injection" as an explicit, intentional outcome instead of
    /// falling through the same branch as a genuinely unrecognized shell.
    ///
    /// **Known gap (acknowledged, not auto-fallback)**: fish versions before
    /// 4.0 (still in use — 4.0 only shipped in 2025) get zero OSC 133 command
    /// blocks under this treatment, since [`prepare`] returns `None` for
    /// `FishNative` unconditionally rather than checking `fish --version`.
    /// Adding a version probe + an event-function-based injection fallback
    /// for pre-4.0 fish (`docs/research/xterm-pty.md` §8 has the snippet) is
    /// deferred — see the Wave E contract's decision log — rather than
    /// silently degrading a shell the app otherwise treats as fully
    /// supported.
    FishNative,
}

fn classify_shell(path: &Path) -> Option<ShellKind> {
    match path.file_name()?.to_str()? {
        "zsh" => Some(ShellKind::Zsh),
        "bash" => Some(ShellKind::Bash),
        "fish" => Some(ShellKind::FishNative),
        _ => None,
    }
}

/// What [`prepare`] hands back to the pty spawn path.
pub struct ShellIntegrationPlan {
    /// `Some((program, args))` when the plan must take over the
    /// `CommandBuilder`'s program/argv entirely — bash's `--init-file` has no
    /// env-only equivalent, so this also means the caller no longer runs the
    /// shell as a login shell (see the module-level bash notes below).
    /// `None` means the caller keeps its existing program selection
    /// (`config.shell` / `CommandBuilder::new_default_prog()`) completely
    /// untouched and only layers `extra_env` on top.
    pub override_program: Option<(PathBuf, Vec<String>)>,
    pub extra_env: Vec<(String, String)>,
    /// The temp directory this plan created on disk (`fresh_temp_dir`), so the pty spawn path
    /// (`infra::pty::spawn`) can stash it on the resulting `PtySession` and remove it deterministically
    /// on session teardown (`PtySession`'s `Drop`) instead of relying solely on the `rm -rf` line each
    /// injected startup script self-executes. That self-cleanup only runs if the shell actually reaches
    /// the bottom of the injected script — a shell that crashes, is killed, or errors out earlier in a
    /// sourced dotfile never gets there, leaking this directory under the OS temp dir for the life of
    /// the machine (not just the app).
    pub temp_dir: PathBuf,
}

fn fresh_temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!("taide-shell-integration-{}", uuid::Uuid::new_v4()))
}

const ZSH_SCRIPT_TEMPLATE: &str = r#"if [ -n "$__TAIDE_ORIG_ZDOTDIR__" ]; then
    export ZDOTDIR="$__TAIDE_ORIG_ZDOTDIR__"
else
    unset ZDOTDIR
fi

typeset -g POWERLEVEL9K_INSTANT_PROMPT="${POWERLEVEL9K_INSTANT_PROMPT:-quiet}"

_taide_rc_dir="${ZDOTDIR:-$HOME}"
if [ -f "$_taide_rc_dir/.zshrc" ]; then
    source "$_taide_rc_dir/.zshrc"
fi
unset _taide_rc_dir

autoload -Uz add-zsh-hook
_taide_precmd()  { print -Pn "\e]133;D;$?\e\\"; print -Pn "\e]133;A\e\\" }
_taide_preexec() { print -Pn "\e]133;C\e\\" }
add-zsh-hook precmd  _taide_precmd
add-zsh-hook preexec _taide_preexec
PS1="%{$(printf '\e]133;B\e\\')%}$PS1"

rm -rf __TAIDE_TEMP_DIR__ 2>/dev/null
"#;

/// Builds the injected `.zshrc`. Hooks and the `PS1` marker are appended
/// *after* `source`-ing the user's real `.zshrc` (rather than before), so
/// nothing the user's own config does — resetting `PS1`, clearing
/// `precmd_functions` (some frameworks do this during their own init) —
/// can strip our integration; ours always applies last and therefore wins.
/// `ZDOTDIR` is restored to its original value (or unset) as literally the
/// first statement, before anything else runs, so `powerlevel10k`'s instant
/// prompt block — which must be the first thing a zsh startup performs — sees
/// the same `ZDOTDIR` it would have without TAIDE in the picture. This also
/// means a login shell's `.zlogin` (read by zsh *after* `.zshrc` returns) is
/// already resolving against the user's real `ZDOTDIR` by then, so it needs
/// no temp-dir counterpart of its own — see [`zsh_passthrough_dotfile_script`]
/// for why `.zshenv`/`.zprofile` are different (they're read *before* this
/// file, while `ZDOTDIR` is still the temp dir).
/// `POWERLEVEL9K_INSTANT_PROMPT=quiet` is set (only if the user hasn't
/// already chosen a value) as a defensive fallback in case some interaction
/// we haven't foreseen still trips p10k's "console output during
/// initialization" warning; it downgrades that to a cosmetic prompt
/// repositioning instead of a scary warning (`POWERLEVEL9K_INSTANT_PROMPT=quiet`
/// is p10k's own documented remedy for that warning).
fn zsh_integration_script(temp_dir: &Path) -> String {
    ZSH_SCRIPT_TEMPLATE
        .replace("__TAIDE_ORIG_ZDOTDIR__", ORIGINAL_ZDOTDIR_ENV_VAR)
        .replace("__TAIDE_TEMP_DIR__", &posix_quote(&temp_dir.to_string_lossy()))
}

/// Builds a temp-`ZDOTDIR` passthrough file (`.zshenv` or `.zprofile`).
///
/// zsh resolves `$ZDOTDIR/.zshenv` (always) and, for login shells,
/// `$ZDOTDIR/.zprofile` *before* it ever reads `.zshrc` — at that point
/// `ZDOTDIR` is still pointed at TAIDE's temp directory, which (before this
/// function existed) held only `.zshrc`. That silently dropped the user's
/// real `~/.zshenv`/`~/.zprofile` (PATH/env setup — e.g. Homebrew's default
/// `~/.zprofile` `shellenv` line) on every injected session. This mirrors VS
/// Code's `shellIntegration-env.zsh`/`-profile.zsh`: temporarily point
/// `ZDOTDIR` at the user's real dotfiles directory just long enough to
/// `source` the requested file (so anything *it* sources relative to
/// `$ZDOTDIR` resolves correctly too), then switch `ZDOTDIR` back to the temp
/// directory so zsh's own next startup-file lookup (`.zprofile` after
/// `.zshenv`, `.zshrc` after `.zprofile`) keeps resolving there instead of
/// falling through to the user's real files untouched by our OSC 133 hooks.
fn zsh_passthrough_dotfile_script(temp_dir: &Path, dotfile_name: &str) -> String {
    format!(
        r#"if [ -n "$__TAIDE_ORIG_ZDOTDIR__" ]; then
    _taide_user_dir="$__TAIDE_ORIG_ZDOTDIR__"
else
    _taide_user_dir="$HOME"
fi

if [ -f "$_taide_user_dir/{dotfile_name}" ]; then
    ZDOTDIR="$_taide_user_dir"
    source "$_taide_user_dir/{dotfile_name}"
    ZDOTDIR=__TAIDE_TEMP_DIR__
fi
unset _taide_user_dir
"#
    )
    .replace("__TAIDE_ORIG_ZDOTDIR__", ORIGINAL_ZDOTDIR_ENV_VAR)
    .replace("__TAIDE_TEMP_DIR__", &posix_quote(&temp_dir.to_string_lossy()))
}

fn zsh_env_script(temp_dir: &Path) -> String {
    zsh_passthrough_dotfile_script(temp_dir, ".zshenv")
}

fn zsh_profile_script(temp_dir: &Path) -> String {
    zsh_passthrough_dotfile_script(temp_dir, ".zprofile")
}

fn prepare_zsh() -> AppResult<ShellIntegrationPlan> {
    let temp_dir = fresh_temp_dir();
    std::fs::create_dir_all(&temp_dir)?;
    std::fs::write(temp_dir.join(".zshenv"), zsh_env_script(&temp_dir))?;
    std::fs::write(temp_dir.join(".zprofile"), zsh_profile_script(&temp_dir))?;
    std::fs::write(temp_dir.join(".zshrc"), zsh_integration_script(&temp_dir))?;

    let mut extra_env = vec![
        ("ZDOTDIR".to_string(), temp_dir.to_string_lossy().to_string()),
        (SHELL_INTEGRATION_ENV_VAR.to_string(), SHELL_INTEGRATION_ENV_VALUE.to_string()),
    ];
    if let Ok(original) = std::env::var("ZDOTDIR") {
        extra_env.push((ORIGINAL_ZDOTDIR_ENV_VAR.to_string(), original));
    }

    Ok(ShellIntegrationPlan {
        override_program: None,
        extra_env,
        temp_dir,
    })
}

/// bash's `--init-file`/`--rcfile` only takes effect for "an interactive
/// shell that is not a login shell" (bash(1)) — a login shell (argv0
/// prefixed with `-`, or `-l`/`--login` passed) ignores it entirely and goes
/// through `/etc/profile` + the first of `~/.bash_profile`/`~/.bash_login`/
/// `~/.profile` instead. Since injecting requires an explicit
/// `CommandBuilder::new(path)` (which — unlike `new_default_prog()` — never
/// login-prefixes argv0), a bash session that *would* have been spawned as a
/// login shell (`config.shell` was `None`, i.e. the auto-detected default)
/// needs that same profile cascade replicated here by hand, or users whose
/// environment setup lives only in `.bash_profile`/`.profile` (common on
/// login-shell-by-default setups) would silently lose it. A session that was
/// already spawned via an explicit shell override was already a *non-login*
/// shell before this feature existed (`CommandBuilder::new(shell)`, no `-`
/// prefix), so that case keeps today's plain `~/.bashrc` behavior.
const BASH_LOGIN_SOURCE_BLOCK: &str = r#"if [ -r /etc/profile ]; then
    . /etc/profile
fi
if [ -r "$HOME/.bash_profile" ]; then
    . "$HOME/.bash_profile"
elif [ -r "$HOME/.bash_login" ]; then
    . "$HOME/.bash_login"
elif [ -r "$HOME/.profile" ]; then
    . "$HOME/.profile"
fi"#;

const BASH_NONLOGIN_SOURCE_BLOCK: &str = r#"if [ -r "$HOME/.bashrc" ]; then
    . "$HOME/.bashrc"
fi"#;

const BASH_SCRIPT_TEMPLATE: &str = r#"__TAIDE_SOURCE_BLOCK__

_taide_prompt() {
    local taide_status=$?
    printf '\e]133;D;%s\e\\' "$taide_status"
    printf '\e]133;A\e\\'
}
PROMPT_COMMAND="_taide_prompt${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
PS0='\[\e]133;C\e\\\]'"$PS0"
PS1='\[\e]133;B\e\\\]'"$PS1"

rm -rf __TAIDE_TEMP_DIR__ 2>/dev/null
"#;

/// Builds the `--init-file` script. Like the zsh script, the user's real
/// config is sourced *first* and the OSC 133 hooks/markers are layered on
/// *after*, so a `.bashrc`/`.bash_profile` that reassigns `PS1` or
/// `PROMPT_COMMAND` outright (very common) can't clobber ours.
fn bash_integration_script(temp_dir: &Path, was_login: bool) -> String {
    let source_block = if was_login {
        BASH_LOGIN_SOURCE_BLOCK
    } else {
        BASH_NONLOGIN_SOURCE_BLOCK
    };
    BASH_SCRIPT_TEMPLATE
        .replace("__TAIDE_SOURCE_BLOCK__", source_block)
        .replace("__TAIDE_TEMP_DIR__", &posix_quote(&temp_dir.to_string_lossy()))
}

fn prepare_bash(shell_path: &Path, was_login: bool) -> AppResult<ShellIntegrationPlan> {
    let temp_dir = fresh_temp_dir();
    std::fs::create_dir_all(&temp_dir)?;
    let script_path = temp_dir.join("init.bash");
    std::fs::write(&script_path, bash_integration_script(&temp_dir, was_login))?;

    Ok(ShellIntegrationPlan {
        override_program: Some((
            shell_path.to_path_buf(),
            vec!["--init-file".to_string(), script_path.to_string_lossy().to_string()],
        )),
        extra_env: vec![(SHELL_INTEGRATION_ENV_VAR.to_string(), SHELL_INTEGRATION_ENV_VALUE.to_string())],
        temp_dir,
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn resolve_default_shell_path() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("SHELL")?);
    path.is_file().then_some(path)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn already_integrated() -> bool {
    std::env::var(SHELL_INTEGRATION_ENV_VAR).is_ok()
}

/// Decides whether — and how — to inject OSC 133 shell integration for a
/// pty about to be spawned with the given (optional, explicit) shell
/// override. Returns `None` when the shell is unsupported, already
/// integrated (dedup), opted out (same check), or fish (native support — see
/// [`ShellKind::FishNative`]); the caller is expected to fall back to its
/// pre-existing, unmodified spawn behavior in every `None` case, so a user
/// who gets no injection sees zero behavior change.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn prepare(shell_override: Option<&str>) -> Option<ShellIntegrationPlan> {
    if already_integrated() {
        return None;
    }

    let was_login = shell_override.is_none();
    let shell_path = match shell_override {
        Some(explicit) => PathBuf::from(explicit),
        None => resolve_default_shell_path()?,
    };

    let outcome = match classify_shell(&shell_path)? {
        ShellKind::Zsh => prepare_zsh(),
        ShellKind::Bash => prepare_bash(&shell_path, was_login),
        ShellKind::FishNative => return None,
    };

    match outcome {
        Ok(plan) => Some(plan),
        Err(error) => {
            log::warn!("셸 통합(OSC 133) 준비 실패, 주입 없이 계속합니다: {error}");
            None
        }
    }
}

#[cfg(target_os = "windows")]
pub fn prepare(_shell_override: Option<&str>) -> Option<ShellIntegrationPlan> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("taide-shell-integration-test-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn 셸_이름으로_zsh_bash_fish를_인식한다() {
        assert_eq!(classify_shell(Path::new("/bin/zsh")), Some(ShellKind::Zsh));
        assert_eq!(classify_shell(Path::new("/opt/homebrew/bin/bash")), Some(ShellKind::Bash));
        assert_eq!(classify_shell(Path::new("/usr/local/bin/fish")), Some(ShellKind::FishNative));
    }

    #[test]
    fn 알수없는_셸은_인식되지_않는다() {
        assert_eq!(classify_shell(Path::new("/usr/bin/tcsh")), None);
        assert_eq!(classify_shell(Path::new("/usr/bin/nu")), None);
        assert_eq!(classify_shell(Path::new("C:\\Windows\\System32\\cmd.exe")), None);
    }

    #[test]
    fn zsh_스크립트는_원래_zdotdir을_복원한_뒤_사용자_rc를_소싱하고_마지막에_훅을_추가한다() {
        let dir = temp_dir("zsh-order");
        let script = zsh_integration_script(&dir);

        let restore_pos = script.find("TAIDE_ORIGINAL_ZDOTDIR").expect("restore 블록 존재");
        let source_pos = script.find("source \"$_taide_rc_dir/.zshrc\"").expect("소싱 라인 존재");
        let hook_pos = script.find("add-zsh-hook precmd").expect("훅 등록 라인 존재");
        let ps1_pos = script.rfind("PS1=").expect("PS1 wrap 라인 존재");

        assert!(restore_pos < source_pos, "ZDOTDIR 복원이 rc 소싱보다 먼저여야 한다");
        assert!(source_pos < hook_pos, "rc 소싱이 훅 등록보다 먼저여야 한다");
        assert!(hook_pos < ps1_pos, "PS1 래핑이 훅 등록보다 뒤여야 한다");
        assert!(script.contains("POWERLEVEL9K_INSTANT_PROMPT:-quiet"));
        assert!(
            script.contains(&dir.to_string_lossy().to_string()),
            "임시 디렉터리 자가정리 경로가 포함되어야 한다"
        );
    }

    #[test]
    fn zsh_env_스크립트는_사용자_zshenv를_소싱한_뒤_zdotdir을_다시_임시_디렉터리로_되돌린다() {
        let dir = temp_dir("zsh-env");
        let script = zsh_env_script(&dir);

        assert!(script.contains("TAIDE_ORIGINAL_ZDOTDIR"));
        let source_pos = script.find("source \"$_taide_user_dir/.zshenv\"").expect("소싱 라인 존재");
        let reset_pos = script.rfind("ZDOTDIR=").expect("임시 디렉터리 복귀 라인 존재");
        assert!(
            source_pos < reset_pos,
            "사용자 .zshenv 소싱 후 ZDOTDIR을 임시 디렉터리로 되돌려야 한다"
        );
        assert!(script.contains(&dir.to_string_lossy().to_string()));
    }

    #[test]
    fn zsh_profile_스크립트는_사용자_zprofile을_같은_방식으로_소싱한다() {
        let dir = temp_dir("zsh-profile");
        let script = zsh_profile_script(&dir);

        assert!(script.contains("TAIDE_ORIGINAL_ZDOTDIR"));
        let source_pos = script.find("source \"$_taide_user_dir/.zprofile\"").expect("소싱 라인 존재");
        let reset_pos = script.rfind("ZDOTDIR=").expect("임시 디렉터리 복귀 라인 존재");
        assert!(
            source_pos < reset_pos,
            "사용자 .zprofile 소싱 후 ZDOTDIR을 임시 디렉터리로 되돌려야 한다"
        );
    }

    #[test]
    fn prepare_zsh는_zshenv_zprofile_zshrc_세_파일을_모두_임시_디렉터리에_생성한다() {
        let plan = prepare_zsh().expect("prepare_zsh는 성공해야 한다");
        let temp_dir = plan
            .extra_env
            .iter()
            .find(|(key, _)| key == "ZDOTDIR")
            .map(|(_, value)| PathBuf::from(value))
            .expect("ZDOTDIR이 extra_env에 있어야 한다");

        assert!(temp_dir.join(".zshenv").is_file(), ".zshenv가 생성되어야 한다");
        assert!(temp_dir.join(".zprofile").is_file(), ".zprofile이 생성되어야 한다");
        assert!(temp_dir.join(".zshrc").is_file(), ".zshrc가 생성되어야 한다");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn bash_스크립트는_로그인_여부에_따라_다른_소싱_체인을_쓴다() {
        let dir = temp_dir("bash-login");
        let login_script = bash_integration_script(&dir, true);
        let nonlogin_script = bash_integration_script(&dir, false);

        assert!(login_script.contains("/etc/profile"));
        assert!(login_script.contains(".bash_profile"));
        assert!(!nonlogin_script.contains("/etc/profile"));
        assert!(nonlogin_script.contains(".bashrc"));
    }

    #[test]
    fn bash_스크립트는_prompt_command와_ps0_ps1을_체이닝한다() {
        let dir = temp_dir("bash-chain");
        let script = bash_integration_script(&dir, false);

        assert!(script.contains(r#"PROMPT_COMMAND="_taide_prompt${PROMPT_COMMAND:+; $PROMPT_COMMAND}""#));
        assert!(script.contains(r#"PS0='\[\e]133;C\e\\\]'"$PS0""#));
        assert!(script.contains(r#"PS1='\[\e]133;B\e\\\]'"$PS1""#));
    }

    #[test]
    fn 임시_디렉터리_경로는_홑따옴표로_안전하게_감싸진다() {
        let dir = PathBuf::from("/tmp/has space");
        let script = zsh_integration_script(&dir);
        assert!(script.contains("rm -rf '/tmp/has space' 2>/dev/null"));
    }

    #[test]
    fn 이미_통합된_환경변수가_있으면_주입을_건너뛴다() {
        let original = std::env::var(SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::set_var(SHELL_INTEGRATION_ENV_VAR, "1");

        let plan = prepare(Some("/bin/zsh"));

        match original {
            Some(value) => std::env::set_var(SHELL_INTEGRATION_ENV_VAR, value),
            None => std::env::remove_var(SHELL_INTEGRATION_ENV_VAR),
        }

        assert!(plan.is_none());
    }

    #[test]
    fn fish는_네이티브_지원이므로_주입하지_않는다() {
        let original = std::env::var(SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::remove_var(SHELL_INTEGRATION_ENV_VAR);

        let plan = prepare(Some("/usr/local/bin/fish"));

        if let Some(value) = original {
            std::env::set_var(SHELL_INTEGRATION_ENV_VAR, value);
        }

        assert!(plan.is_none());
    }

    #[test]
    fn 명시적_zsh_override는_커맨드_프로그램을_바꾸지_않고_env만_추가한다() {
        let original = std::env::var(SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::remove_var(SHELL_INTEGRATION_ENV_VAR);

        let plan = prepare(Some("/bin/zsh")).expect("zsh는 주입되어야 한다");

        if let Some(value) = original {
            std::env::set_var(SHELL_INTEGRATION_ENV_VAR, value);
        }

        assert!(plan.override_program.is_none());
        assert!(plan.extra_env.iter().any(|(key, _)| key == "ZDOTDIR"));
    }

    #[test]
    fn 명시적_bash_override는_커맨드_프로그램과_init_file_인자를_설정한다() {
        let original = std::env::var(SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::remove_var(SHELL_INTEGRATION_ENV_VAR);

        let plan = prepare(Some("/bin/bash")).expect("bash는 주입되어야 한다");

        if let Some(value) = original {
            std::env::set_var(SHELL_INTEGRATION_ENV_VAR, value);
        }

        let (program, args) = plan.override_program.expect("bash는 명시적 프로그램/인자가 필요하다");
        assert_eq!(program, PathBuf::from("/bin/bash"));
        assert_eq!(args[0], "--init-file");
    }

    #[test]
    fn 알수없는_셸_override는_아무것도_주입하지_않는다() {
        let original = std::env::var(SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::remove_var(SHELL_INTEGRATION_ENV_VAR);

        let plan = prepare(Some("/usr/bin/tcsh"));

        if let Some(value) = original {
            std::env::set_var(SHELL_INTEGRATION_ENV_VAR, value);
        }

        assert!(plan.is_none());
    }
}
