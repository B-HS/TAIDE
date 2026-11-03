use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use taide_model::error::AppErrorKind;
use taide_model::ids::ProjectId;

use super::*;

const BUNDLE_MACOS_DIR: &str = "TAIDE.app/Contents/MacOS";
const APP_EXECUTABLE: &str = "taide-native";
const CLI_SIDECAR: &str = "taide-cli";
#[cfg(target_os = "macos")]
const FOREIGN_TARGET: &str = "other-tool";
const LINK_DIR: &str = "bin";
const LINK_NAME: &str = "taide";

struct Fixture {
    root: PathBuf,
    executable: PathBuf,
    target: PathBuf,
    link: PathBuf,
    scripts: Arc<Mutex<Vec<String>>>,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "taide-native-agent-cli-{name}-{}",
            ProjectId::new()
        ));
        let bundle = root.join(BUNDLE_MACOS_DIR);
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::create_dir_all(root.join(LINK_DIR)).unwrap();
        let executable = bundle.join(APP_EXECUTABLE);
        let target = bundle.join(CLI_SIDECAR);
        std::fs::write(&executable, "").unwrap();
        std::fs::write(&target, "").unwrap();
        Self {
            link: root.join(LINK_DIR).join(LINK_NAME),
            root,
            executable,
            target,
            scripts: Arc::default(),
        }
    }

    fn ports(
        &self,
        executable: &Path,
        effect: impl Fn(&Path) -> AppResult<()> + Send + Sync + 'static,
    ) -> Ports {
        let executable = executable.to_path_buf();
        let scripts = self.scripts.clone();
        let link = self.link.clone();
        let effect_link = link.clone();
        Ports {
            executable: Arc::new(move || Ok(executable.clone())),
            link,
            osascript: Arc::new(move |script| {
                scripts.lock().unwrap().push(script.to_owned());
                effect(&effect_link)
            }),
        }
    }

    fn linking_ports(&self) -> Ports {
        let target = self.target.clone();
        self.ports(&self.executable, move |link| {
            if std::fs::symlink_metadata(link).is_ok() {
                std::fs::remove_file(link)?;
            }
            std::os::unix::fs::symlink(&target, link)?;
            Ok(())
        })
    }

    #[cfg(target_os = "macos")]
    fn cancelling_ports(&self) -> Ports {
        self.ports(&self.executable, |_| Ok(()))
    }

    fn scripts(&self) -> Vec<String> {
        self.scripts.lock().unwrap().clone()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.root));
    }
}

#[cfg(target_os = "macos")]
fn is_live(status: &CliInstallStatus) -> bool {
    status.installed && !status.dangling
}

#[cfg(target_os = "macos")]
#[test]
fn 설치는_번들_밖_실행파일을_dev_build_오류로_거절하고_관리자_스크립트를_부르지_않는다() {
    let fixture = Fixture::new("dev-build");
    let outside = fixture.root.join(APP_EXECUTABLE);
    let ports = fixture.ports(&outside, |_| Ok(()));
    for action in [Action::Install, Action::Uninstall] {
        let error = run(&ports, action).unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument, "{action:?}");
        assert!(
            error.to_string().contains("not a dev build"),
            "{action:?}: {error}"
        );
    }
    assert!(fixture.scripts().is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn 실제_포트는_원본_링크_경로를_쓰고_번들이_아닌_개발_빌드에서_링크를_건드리기_전에_거절한다() {
    let ports = Ports::system();
    assert_eq!(
        ports.link,
        Path::new(crate::agent_hooks::TAIDE_CLI_TARGET_PATH)
    );
    let executable = (ports.executable)().unwrap();
    assert!(!service::is_running_from_macos_app_bundle(&executable));
    for action in [Action::Install, Action::Uninstall] {
        let error = run(&ports, action).unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument, "{action:?}");
        assert!(
            error.to_string().contains("not a dev build"),
            "{action:?}: {error}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn 설치는_미설치일_때만_관리자_스크립트로_번들_cli에_연결하고_결과_상태를_돌려준다() {
    let fixture = Fixture::new("install");
    let ports = fixture.linking_ports();
    assert!(!status(&ports).installed);

    let installed = run(&ports, Action::Install).unwrap();
    assert!(is_live(&installed));
    assert_eq!(installed.resolved_path.as_deref(), fixture.target.to_str());
    assert_eq!(installed.target_path, fixture.link.to_str().unwrap());
    let scripts = fixture.scripts();
    assert_eq!(scripts.len(), 1);
    assert!(scripts[0].contains(fixture.target.to_str().unwrap()));
    assert!(scripts[0].contains(fixture.link.to_str().unwrap()));
    assert!(scripts[0].contains("with administrator privileges"));

    let again = run(&ports, Action::Install).unwrap();
    assert!(is_live(&again));
    assert_eq!(fixture.scripts().len(), 1);
}

#[cfg(target_os = "macos")]
#[test]
fn 관리자_대화상자_취소는_오류없이_미설치_상태를_돌려주고_실패는_그대로_전한다() {
    let fixture = Fixture::new("cancel");
    let cancelled = run(&fixture.cancelling_ports(), Action::Install).unwrap();
    assert!(!cancelled.installed);
    assert_eq!(fixture.scripts().len(), 1);

    let failing = fixture.ports(&fixture.executable, |_| {
        Err(AppError::Internal("osascript failed: synthetic".into()))
    });
    let error = run(&failing, Action::Install).unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert_eq!(fixture.scripts().len(), 2);
}

#[cfg(target_os = "macos")]
#[test]
fn 제거는_없는_링크를_그대로_두고_taide_소유가_아닌_링크를_지우지_않으며_소유_링크만_지운다() {
    let fixture = Fixture::new("uninstall");
    let ports = fixture.linking_ports();
    let missing = run(&ports, Action::Uninstall).unwrap();
    assert!(!missing.installed);

    let foreign = fixture.root.join(FOREIGN_TARGET);
    std::fs::write(&foreign, "").unwrap();
    std::os::unix::fs::symlink(&foreign, &fixture.link).unwrap();
    let error = run(&ports, Action::Uninstall).unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert!(
        error.to_string().contains("not managed by TAIDE"),
        "{error}"
    );
    assert!(std::fs::symlink_metadata(&fixture.link).is_ok());

    std::fs::remove_file(&fixture.link).unwrap();
    std::os::unix::fs::symlink(&fixture.target, &fixture.link).unwrap();
    assert!(is_live(&status(&ports)));
    let removed = run(&ports, Action::Uninstall).unwrap();
    assert!(!removed.installed);
    assert!(std::fs::symlink_metadata(&fixture.link).is_err());
    assert!(fixture.target.exists());
    assert!(fixture.scripts().is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn 외부_에디터_연결은_살아있는_링크면_설치하지_않고_끊긴_링크면_다시_설치한다() {
    let fixture = Fixture::new("connect");
    std::os::unix::fs::symlink(&fixture.target, &fixture.link).unwrap();
    let live = fixture.ports(&fixture.root.join(APP_EXECUTABLE), |_| Ok(()));
    let connected = run(&live, Action::ConnectExternalEditor).unwrap();
    assert!(is_live(&connected));
    assert!(fixture.scripts().is_empty());

    std::fs::remove_file(&fixture.link).unwrap();
    std::os::unix::fs::symlink(fixture.root.join("moved").join(CLI_SIDECAR), &fixture.link)
        .unwrap();
    let ports = fixture.linking_ports();
    let dangling = run(&ports, Action::Status).unwrap();
    assert!(dangling.installed && dangling.dangling);
    let reconnected = run(&ports, Action::ConnectExternalEditor).unwrap();
    assert!(is_live(&reconnected));
    assert_eq!(fixture.scripts().len(), 1);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn macos가_아닌_빌드는_설치와_제거를_지원하지_않는다는_오류를_돌려준다() {
    let fixture = Fixture::new("unsupported");
    let ports = fixture.linking_ports();
    for action in [
        Action::Install,
        Action::Uninstall,
        Action::ConnectExternalEditor,
    ] {
        let error = run(&ports, action).unwrap_err();
        assert!(
            error.to_string().contains("only supported on macOS"),
            "{action:?}: {error}"
        );
    }
    assert!(fixture.scripts().is_empty());
}
