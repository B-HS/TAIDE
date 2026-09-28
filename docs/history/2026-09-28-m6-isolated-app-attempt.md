# M6 격리 앱 기동 첫 시도

## 대상

- `src-tauri/src/lib.rs`의 setup·IDE 서버·Exit 배선
- `src-tauri/tauri.conf.json`, `src-tauri/tauri.dev.conf.json`, `scripts/tauri.ts`
- `crates/taide-ide/src/lockfile.rs`, `src-tauri/src/domain/ide/commands.rs`

## 실제 결과

사용자 A 선택에 따라 전용 `net.gumyo.taide.m6test.*` identifier로 `node_modules/.bin/tauri dev`를 실행했습니다. 사전 확인에서 해당 identifier의 앱 데이터·로그 경로는 없고 개발 서버 포트 5173은 비어 있었습니다. 설치된 Tauri CLI 2.11.4의 도움말과 [공식 CLI 문서](https://v2.tauri.app/reference/cli/)는 여러 `--config`가 순서대로 병합돼 마지막 충돌 값이 우선한다고 명시합니다. 기존 개발 설정 뒤에 고유 identifier를 전달했으며 `cargo run`은 약 8분 뒤 빌드 완료·앱 기동까지 진행했습니다.

기동 로그에서 키링 서비스명은 고유 identifier였고 복원 프로젝트 0개·watcher 0개였습니다. 반면 IDE 서버는 포트 45059를 열고 사용자 홈 `.claude/ide`에 lockfile을 썼습니다. `lockfile_dir()`은 identifier가 아닌 `CLAUDE_CONFIG_DIR` 또는 사용자 홈을 사용합니다. 이 부작용을 확인하자 더 이상 GUI 조작·직접 Exit 검사를 하지 않고 개발 세션에 `Ctrl+C`를 보냈습니다. 이후 앱 실행 파일 점유와 두 포트 45059·5173의 리스너는 없었지만 `.claude/ide/45059.lock`은 남았습니다. 토큰이 들어갈 수 있어 파일 내용은 읽지 않았고 삭제도 하지 않았습니다. 전용 identifier의 앱 데이터 경로는 생성되지 않았고 전용 로그 경로만 생겼습니다.

## 판정과 다음 조건

이번 결과는 앱 기동의 관찰값이지 M6의 GUI·직접 Exit 합격 증거가 아닙니다. 앱 디버그 실행은 computer-use 앱 목록에서 식별되지 않아 창을 직접 조작하지도 못했습니다. 정확한 lockfile 하나의 정리 방법에 대한 사용자 답을 기다립니다. 다음 시도는 새 임시 경로를 `CLAUDE_CONFIG_DIR`에 주입해 IDE lockfile·stale cleanup을 사용자 홈에서 분리하고, GUI 접근 수단과 종료 시 비강제 cleanup 경계를 먼저 확인해야 합니다. 기존 사용자 설정·키링·lockfile 내용은 조회하지 않습니다.

## 실행하지 않은 번들 준비

이후 프로젝트의 `bun run tauri build --debug --bundles app --no-sign` 경로에 별도 `net.gumyo.taide.m6bundle.*` identifier를 마지막 설정으로 넣고 `CARGO_NET_OFFLINE=true`에서 실행했습니다. sidecar 재빌드, Vite 3930개 모듈의 프런트엔드 빌드, Rust dev profile 21분 9초 빌드와 macOS `.app` 번들 생성이 exit 0으로 끝났습니다. [공식 macOS 앱 번들 안내](https://v2.tauri.app/distribute/macos-application-bundle/)의 방식으로 생성된 `target/debug/bundle/macos/TAIDE.app`의 `Info.plist`는 전용 identifier였고 실행 파일도 존재합니다. `--no-sign`을 사용한 로컬 실기 준비물이므로 서명·공증이나 배포 적합성의 증거는 아닙니다.

이 번들은 실행하지 않았습니다. computer-use 앱 목록에서도 아직 설치된 앱으로 나타나지 않아 GUI 연결 가능 여부는 미검증입니다. 기존 사용자 홈 lockfile은 그대로 있고 작업 트리는 깨끗합니다.

## 후속 시작 경계 감사

`Settings::default`는 `ide_integration_enabled`를 켜고 `agent_hooks_enabled`와 `remote_access_enabled`를 끕니다. 전용 identifier의 첫 실행은 복원 프로젝트가 0개였고, `agent_actions::poll_agents`는 프로젝트 목록이 비어 있으면 foreground PID probe를 호출하지 않습니다. 반면 IDE 자동 시작은 기본값만으로도 사용자 홈 lockfile을 만들 수 있어 `CLAUDE_CONFIG_DIR`의 전용 경로가 필요합니다.

`src-tauri/src/lib.rs::run`의 macOS `fix_path_env::fix()`는 앱 상태 복원보다 먼저 실행됩니다. 현재 고정된 `fix-path-env-rs`의 `fix_vars` 구현은 `SHELL` 또는 기본 `/bin/zsh`를 `-ilc`로 실행하고 홈 디렉터리를 작업 디렉터리로 사용해 로그인 셸 설정에서 `PATH`를 읽습니다. 전용 앱 identifier와 IDE 경로만으로는 이 시작 경계까지 격리되지 않습니다. 다음 실기에서 셸 설정을 어떻게 분리할지는 아직 검증하지 않았으며, 앱을 재실행하거나 사용자 프로필을 수정하지 않았습니다.
