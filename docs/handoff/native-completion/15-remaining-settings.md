# settings 영역 남은 작업 (Rust-native 전환)

> 작성일: 2026-11-04 · 기준: 저장소 `/Users/hyunseokbyun/development/TAIDE`, 브랜치 `to_rust_native`, HEAD `0c52b438`(settings lane 병합 `3864e1db` 뒤)
> Git 영역 비교 기준: worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(브랜치 `lane/git`, HEAD `50892b59`). 이 worktree는 `to_rust_native`를 병합하는 중입니다. 충돌 표시(`UU`) 파일 8개는 표식(`<<<<<<<`)이 이미 지워졌지만 아직 스테이징되지 않았고, 작업 트리에 커밋되지 않은 수정(`domains/git.rs`의 pull/push 알림, `domains/shell.rs`의 `Event::IdeDiff` 분기와 `notifications_tests` 공개 범위 확대 등)이 있습니다. settings 행은 두 기능표에서 글자 하나까지 같고(`diff`로 확인), settings 소스·검사 파일도 두 쪽이 같습니다(`diff -q`로 확인).
> 독자: 이 저장소를 처음 보는 구현 담당 AI. 이 문서와 여기서 가리키는 파일 구간만 읽고 같은 품질로 구현할 수 있게 썼습니다. 줄 번호는 위 HEAD 기준이며 바뀔 수 있으니 항상 심볼 이름으로 다시 찾으십시오(`grep -n "fn 심볼" 파일`).
> 같은 경로의 2026-11-03 판을 2026-11-04 기준으로 다시 확인해 고친 문서입니다. 바뀐 점: git lane 병합 작업 트리의 진행 반영(settings-22·23·67·80), settings-42의 상수 삭제 범위 정정, TS 줄 번호 정정, 검사 도우미 재사용 경로 구체화.

## 0. 먼저 읽을 것

### 0.1 약어

| 약어 | 절대 경로 |
| --- | --- |
| `TS` | `/Users/hyunseokbyun/development/TAIDE/src` (원본 React 앱, 동작의 정본) |
| `TAURI` | `/Users/hyunseokbyun/development/TAIDE/src-tauri/src` (원본 Rust 백엔드 명령) |
| `CR` | `/Users/hyunseokbyun/development/TAIDE/crates` (Tauri 앱과 native 앱이 함께 쓰는 공유 crate, 루트 workspace) |
| `APP` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src` (native 실행 파일 crate) |
| `UI` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src` (egui 화면 crate) |
| `TERM` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-terminal/src` |
| `LOC` | `/Users/hyunseokbyun/development/TAIDE/crates/taide-locale/resources/locales/{en,ko,ja}.json` (평면 키 `"settings.xxx"`) |
| `GITLANE` | `/Users/hyunseokbyun/development/TAIDE-lanes/git` (git lane worktree, 병합 중) |
| `MATRIX` | `/Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-09-native-function-matrix.md` |
| `SPEC` | `/Users/hyunseokbyun/development/TAIDE/docs/research/2026-10-28-lane-settings-spec.md` (lane 시작 시점 사양, 39행 대상) |
| `QA-S1`~`QA-S5` | `/Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-11-02-native-lane-settings-{s1-interface-performance,s2-lsp-ai,s3-plugins-vsix,s4-sync-remote,s5-finish}.md` |
| `HO-10`, `HO-11`, `HO-12`, `HO-14`, `HO-16` | 같은 폴더의 `10-remaining-shell.md`, `11-remaining-editor-preview.md`, `12-remaining-explorer-search.md`, `14-remaining-git.md`, `16-remaining-commands-ui.md` (다른 영역 인계 문서) |

### 0.2 완료 기준과 금지 사항 (모든 행 공통)

- 완료(complete) = 원본 TS의 화면·상태·상호작용 대응 + 실제 앱 경로(`APP/main.rs` → `NativeApplication`) + 그 동작을 직접 확인하는 자동 검사(`MATRIX` 3~5줄, `docs/memory/2026-10-28-native-lane-operations.md` 4절). 원본의 결함·내부 수치는 재현하지 않습니다(`docs/acknowledge/2026-10-09-native-full-resume.md` 11줄, `docs/acknowledge/2026-10-09-native-find-regex-decisions.md` 5·7줄).
- 사용자 결정 규칙: "큰 변경이 아니면 추천안"(`docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 11줄). settings 사양 4.1절 결정 8개는 모두 추천안으로 확정됐습니다(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`).
- 금지: GUI 실행, 실제 OS 알림 전송, 시스템 설정 앱 열기, 실제 클립보드, Keychain, `/usr/local/bin` 쓰기와 관리자 프롬프트, 실제 사용자 홈(`~/.codex` 등)·실제 프로젝트 파일 쓰기, 네트워크(GitHub·LSP 다운로드·모델 목록), 실제 파일 선택 창, 합성 OS 입력, `APP/../tests/explorer-delete.rs` 실행(사용자 휴지통), 동결 crate `native/taide-remote-web` 소스·lockfile 변경, 새 외부 의존성, `#[allow]`·`@ts-ignore`류 우회, 검사 기대값 조작, 코드 주석(공개 API와 기존 검사 도우미 관례의 영어 `///`만 예외). 모두 주입 포트와 가짜 구현으로 검사합니다(`SPEC` 0절 금지 문단, 4.2절).
- 사용자 문자열은 원본과 같은 locale 키를 씁니다. 이 문서에 나오는 키는 모두 `LOC` 세 파일에 각 1건씩 있습니다(`grep -c '"키"' en.json ko.json ja.json`으로 확인함).
- 매직넘버 금지: 의미 있는 숫자는 검사 파일 상단 `const`로 둡니다(기존 파일이 `const DEADLINE`, `const SETTLE_FRAMES`처럼 씁니다).
- 같은 도우미가 두 번째로 필요해지면 복사하지 말고 기존 것의 공개 범위를 넓혀 재사용합니다(`~/.claude/convention/common.md` 3.3 "2회 이상" 규칙). 다른 영역 소유 검사 파일의 공개 범위를 넓히면 그 영역 QA에도 한 줄 적습니다.

### 0.3 실행 명령 (메인 저장소 기준)

lane이 끝났으므로 남은 작업은 `to_rust_native`(메인 저장소)에서 합니다. 병합 게이트 스크립트 `/Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-03-native-merge-gate.sh`가 쓰는 target을 그대로 씁니다. 아래 `<T>`는 `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`입니다(작업 규칙상 한 줄 절대 경로 명령만 쓰므로 실제로는 펼쳐 씁니다).

```
# 앱 lib(실제 App 검사 포함). 필터는 검사 함수 이름이나 모듈 경로의 일부(한국어 가능)
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- <필터> --test-threads=2
# UI lib
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --lib --features native-host,inspection --locked --offline --target-dir <T> -- <필터>
# 공유 crate(루트 workspace)
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml -p taide-theme --lib --locked --offline --target-dir <T>
# 컴파일 확인
cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --all-targets --locked --offline --target-dir <T>
# 형식
cargo fmt --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml -- --check
# UI crate나 taide-model을 바꿨을 때만: 동결 미러 Wasm 컴파일
cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir <T>
```

- LSP를 쓰는 앱 검사가 "mock 실행 파일 없음"으로 실패하면 먼저 `cargo build --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir <T>`를 실행합니다(`docs/memory/2026-10-07-native-transition-operations.md` 6절). 이 문서의 settings 검사는 LSP mock이 필요 없습니다.
- 앱 lib 검사는 `--test-threads=2`(많아도 4)로 돌립니다. 실제 App 검사가 동시에 많이 돌면 호스트 큐(용량 64) 포화로 흔들립니다(`docs/memory/2026-10-28-native-lane-operations.md` 1절 표).
- 알려진 기존 실패(이 영역과 무관, 고치지 않음): `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`(`APP/editor-fonts.rs` 514줄), `CR/taide-runtime/tests/agent_probe.rs`의 소스 문자열 단언 1건(`docs/PROCESS.md` W1 항목 "2026-11-03 settings 병합").

### 0.4 egui 0.36과 Rust 2024를 모르는 독자를 위한 최소 지식

- egui는 즉시 모드(immediate mode) GUI입니다. 상태는 앱이 들고 있고, 매 프레임 UI 함수를 다시 호출해 그립니다. 이 저장소의 검사는 창 없이 `egui::Context::run_ui(RawInput, |ui| { .. })`로 한 프레임을 돌리고, 반환된 `egui::FullOutput`의 `shapes`(그린 도형 목록)와 `platform_output.accesskit_update`(접근성 트리)를 봅니다.
    - 글자: `egui::Shape::Text(t)`의 `t.galley.job.text`(또는 `t.galley.text()`), 위치는 `t.galley.rect.translate(t.pos.to_vec2())`. 실제 예: `APP/application-settings-sections-tests.rs` `painted`(234줄).
    - 사각형·원·텍스처: `egui::Shape::Rect(r)`의 `r.fill`·`r.rect`, `egui::Shape::Circle(c)`의 `c.fill`, `egui::Shape::Mesh(m)`의 `m.texture_id`. 도형이 `Shape::Vec`으로 중첩될 수 있으니 펼쳐서 봅니다(`painted`가 선례). 텍스처 이름은 `context.tex_manager().read().meta(id)`의 `name`이나 `allocated()`로 찾습니다(선례 `APP/application-error-boundary-tests.rs` `texture_ids` 211줄).
    - 접근성 노드: `context.enable_accesskit()` 뒤 `output.platform_output.accesskit_update.nodes`에서 `node.label()`·`node.bounds()`. 버튼 위치 찾기에 씁니다(`APP/application-settings-sections-tests.rs` `nodes` 263줄).
    - `RawInput`의 기본값은 `focused: true`입니다(`APP/../vendor/egui-input/src/data/input/raw_input.rs` 106줄. 이 저장소는 `[patch.crates-io] egui = { path = "vendor/egui-input" }`로 패치한 egui를 씁니다, `APP/../Cargo.toml` 111~112줄). 창 포커스가 없는 상황은 `focused: false`로 만듭니다. 포인터 클릭은 `focused: false`에서도 처리됩니다(같은 vendor `src/context.rs` 652줄은 포커스일 때만 포인터 포커스 요청을 기록할 뿐 클릭 자체는 막지 않습니다).
    - 입력 사건: `egui::Event::PointerMoved`, `PointerButton { pos, button, pressed, modifiers }`, `Text(String)`, `Key { key, physical_key, pressed, repeat, modifiers }`. `crate::application::project_rail_tests::pointer(pos, button)`가 이동+누름+뗌 3개를 한 프레임 사건으로 만듭니다(`APP/application-project-rail-tests.rs` 192줄).
- eframe 0.36.2(vendored, `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/vendor/eframe`)는 `App::logic(ctx, frame)`(입력 처리·백그라운드, `src/epi.rs` 167줄)과 `App::ui(ui, frame)`(그리기, 182줄)를 나눕니다. 검사는 `eframe::CreationContext::_new_kittest(context)`로 앱을 만들고 `eframe::Frame::_new_kittest()`로 두 함수를 직접 부릅니다. 검사 도우미는 `logic`을 `run_ui` **전에** 부르므로(`Scene::step` 168~184줄) `logic` 안의 `context.input(..)`은 **직전 프레임**의 입력을 읽습니다.
- native crate(`APP`, `UI`)는 Rust 2024 edition입니다(각 `Cargo.toml` `edition = "2024"`, `rust-version = "1.95"`). `if a && let Some(b) = c { .. }`(let chain)을 이미 많이 씁니다(예: `APP/domains/settings.rs` `apply_settings_reply`의 `Reply::AppFileWritten` arm). 반면 `CR/taide-theme`는 edition 2021(`rust-version = "1.89"`)이라 let chain을 쓸 수 없습니다.
- 검사 파일은 소스 파일 끝에 `#[cfg(test)] #[path = "../application-xxx-tests.rs"] mod xxx_tests;`로 붙입니다. 그 모듈은 붙인 모듈의 자식이라 부모와 조상 모듈의 비공개 필드·함수를 그대로 씁니다. `pub(super)`는 "부모 모듈과 그 자손에게 공개", `pub(in crate::application)`는 "`crate::application`과 그 자손에게 공개"입니다. `domains`는 `crate::application`의 자식(`APP/application.rs` 34~35줄 `#[path = "domains/mod.rs"] pub mod domains;`, `APP/lib.rs` 371줄 `pub use application::domains;`)이라 `crate::domains::x`와 `crate::application::domains::x`는 같은 경로입니다.

### 0.5 다른 영역 병합 상태를 확인하는 명령 (작업 시작 때마다)

아래는 메인 저장소 파일을 읽는 한 줄 명령입니다. 결과에 따라 3절 각 행의 (g)가 달라집니다.

| 확인 | 명령 | 결과 해석 |
| --- | --- | --- |
| git lane 병합 | `grep -c "fn notify_git_remote" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/domains/git.rs` | 1이면 git lane이 메인에 병합됨(2026-11-04 메인은 0, `GITLANE` 작업 트리에는 1) |
| 알림 검사 도우미 공개 | `grep -c "pub(in crate::application) mod notifications_tests" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/domains/shell.rs` | 1이면 `FakePlatform`·`launch_with_platform`을 settings 검사에서 쓸 수 있음(메인 0, `GITLANE` 작업 트리 1) |
| 검색 치환 알림 | `grep -c "notify_event" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/domains/explorer_search.rs` | 1 이상이면 explorer-search-33의 알림 한 줄이 들어옴(메인·`GITLANE` 모두 0) |
| IDE diff 수신 | `grep -c "Event::IdeDiff" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/domains/shell.rs` | 1이면 git-49 수신이 shell 수신함 경로로 옮겨짐(메인 0, `GITLANE` 작업 트리 1) |
| status 라벨 잔여 | `grep -c "status = Some(" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/application.rs` 와 같은 명령을 `domains/*.rs`, `git-editor.rs`에 | 합계(아래 settings-78)가 0이 되어야 상태바 라벨 제거 가능 |

## 1. 요약

### 1.1 남은 행 수

`MATRIX`의 settings 표(제목 544줄, 행 550~632줄, 83행)를 `sed -n 550,632p <MATRIX> | awk -F'|' '{gsub(/ /,"",$5); print $5}' | sort | uniq -c`로 세면 complete 72, partial 10, not-applicable 1(settings-81, 대상 아님)입니다. 남은 행은 **10행, 모두 partial**(unwired 0, missing 0)입니다.

주의: `MATRIX` 상단 요약표 19줄(`settings | 83 | 43 | 10 | 28 | 1 | 1 | 0`)과 서문 7줄의 전체 수는 lane 병합 전 값 그대로라 틀렸습니다. 행을 바꿀 때 이 줄들도 실제 행 수로 다시 세십시오(1.5절).

| ID | 기능 | 기능표 줄 | 남은 것(한 줄) | 막는 것 |
| --- | --- | --- | --- | --- |
| settings-5 | 탭 소유권(보조 창 포함) | 554 | 병합본에서 기존 App 검사 1회 실행·기록 | 없음 |
| settings-22 | 시험 알림과 억제 사유 toast | 571 | 전달 성공·플랫폼 오류 App 검사(가짜 platform, 포커스 없음) | git lane 병합(검사 도우미 공개 범위, 0.5절) |
| settings-23 | OS 알림 범주 7종 | 572 | 검색 치환 알림 한 줄(explorer-search 소유), git pull/push 알림 병합, 범주 묶음 App 검사 | git lane 병합, explorer-search-33 |
| settings-28 | 표시·입력 옵션의 편집기 반영 | 577 | App 소비자 검사 4종(wordWrap·rulers·lineNumbers·renderWhitespace), diff 2종·semantic 소비자 | editor-7, git-42 showMoves, git lane 병합 |
| settings-30 | Terminal section | 579 | Settings 경로로 바꾼 scrollback이 실행 중 세션 history에 반영되는 App 검사 | 없음 |
| settings-42 | VSIX 가져오기 대화상자 | 591 | `taide-theme` `service.rs` 검사 모듈의 색·대비 도우미 중복 제거 | 없음 |
| settings-48 | Sync 뒤 데이터 반영 | 597 | 동기화 다운로드 뒤 테마 목록·화면 테마·settings.json 탭 갱신 App 검사 | 없음 |
| settings-67 | 편집한 키의 실제 동작 반영 | 616 | 실행 경로 없는 명령의 소비자(다른 영역 몫), 거절 목록 재측정·분류 | commands-ui(wave 3)·git 병합·editor |
| settings-78 | 오류 표시 경로 | 627 | 남은 `status` 대입 처리와 상태바 라벨·필드 제거 | 모든 lane 병합(특히 git), 메인 통합 |
| settings-80 | Interface 설정 소비처 | 629 | minimap App 검사, searchOnType 끔 App 검사, IDE diff 자동 열기(git-49) 병합본 실행 | git lane 병합(IDE diff만) |

성격별: 바로 가능 6(5, 30, 42, 48, 28의 App 검사 4종, 80의 minimap·searchOnType 끔), git lane 병합 대기 3(22, 23, 80의 IDE diff), 다른 영역 완료 대기 3(28의 semantic·showMoves, 67, 78).

### 1.2 현재 구현 위치

| 층 | 파일 | 핵심 심볼(줄) |
| --- | --- | --- |
| UI 화면 | `UI/settings-view.rs` | `Views`(175), retained `View`(`mod retained` 190~213: `catalog`, `active`, `viewport`, `preview`, `host`), `Views::preview`(221), `inspection`(243, feature `inspection`), `begin_frame`·`finish_frame`(252·257, `retain(seen)`), `sections_data`·`sections_data_mut`(277·281), `observe_theme_revision`(285), `invalidate_catalog`(301), `Views::show`(451, 동결 미러가 부르므로 시그니처 고정), `Views::show_with_host`(463), `Catalog { themes: AppResult<Vec<ThemeSummary>> }`(97) |
| UI 호스트 section | `UI/settings-sections-view.rs` | `Host<'a>`(159), `OwnerState`(183), `interface`(501), `notifications`(891), `performance`(950), `lsp`(1022), `ai`(1329), `plugins`(1715), `vsix_dialog`(2030), `sync`(2568), `remote`(2843) |
| UI 데이터 캐시 | `UI/settings-sections-data.rs` | `Query`(66), `Action`(117), `suppression_label`(198), `Request`(304), `Reply`(310), `Notice`(413), `Data`(547), `observe_window_focus`(651), `send_test_notification`(664), `record_sync_status`(686), `take_requests`(855), `Data::accept`(867, `Reply::TestNotification` 1168), `Request::failed`(1246) |
| UI 대화상자 | `UI/settings-dialogs.rs` | `AlertDialog`·`Dialog` 계약(트랩·Escape·바깥 클릭·포커스 복귀) |
| UI 설정 변경 | `UI/settings-controls.rs`, `UI/settings-code-controls.rs` | `Section`(19, `BASIC` 37, `HOSTED` 48), `Switch`(81), `Numeric`(364), `Change`(518), `HostedChange`(530); 코드 쪽 `Switch`(13), `Selection`(299), `Change`(509, `Rulers` 513) |
| 앱 영역 | `APP/domains/settings.rs` | `Command`(34), `Reply`(68), `State`(92), `ReportedFailures`(108), `unreported_failure`(113), `Events`(120, `record` 132), `dispatch`(203), `execute_sections`(470), `poll_settings_revisions`(939), `apply_settings_reply`(962), `accept_sections_reply`(1094), `apply_settings_intent`(1142), `sync_theme_preview`(1218), `apply_presentation_appearances`(1258), `pick_settings_files`(1368), `tick_settings_reads`(1403), `load_settings_requests`(1443), `AppSurfaces::show_settings_tab`(1503), `show_app_file_tab`(1600), 검사 모듈 5개(1645~1663) |
| 앱 포트 | `APP/settings-ports.rs` | `Ports`(183: `detect_lsp_servers`, `lsp_install`, `pick_file`, `gist`, `agents`, `cli`, `open_notification_settings`, `perf`, `font_families`) |
| 앱 기타 | `APP/settings-sync-time.rs`(`format_system_time`), `APP/app-file-views.rs`(`observe_settings_revision` 201, `settings_command` 317, `open_command` 331), `APP/presentation-refresh.rs`(`Changes::record` 32, `editor_presentation` 219) | |
| 공유 crate | `CR/taide-runtime/src/{sync_actions,notification_actions,vsix_actions,agent_host,app_actions}.rs`, `CR/taide-theme/src/convert/*` | `apply_sync_download`(280), `notification_notify`(12), `vsix_theme_candidates`, `convert_vscode_theme` |

### 1.3 다른 영역과의 의존·소유 경계

소유 규칙의 정본은 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 채택 2·3번째 항목입니다: 알림 생산자·첫 전달 안내·Sync 테마 재로드·보조 창 수명은 shell, git 알림·IDE diff·diff 설정 소비는 git, reveal·searchOnType·검색 치환 알림은 explorer-search가 만들고 settings는 재사용·확인·행 갱신만 합니다. OS 알림 진입점은 shell의 `notify_event` 하나입니다.

| 선행 산출물 | 소유 행 | 메인(`0c52b438`) 상태 | `GITLANE`(병합 작업 트리) | settings 행 |
| --- | --- | --- | --- | --- |
| 알림 진입점 `notify_event`, 생산자, 주입 platform | shell-91 | complete. `APP/domains/shell.rs` `notify_event`(1175), `crate::application::Overrides.platform`(`APP/application.rs` 76~79, 적용 348~358) | 같음 | 22, 23 |
| 가짜 platform 검사 도우미 `FakePlatform`·`launch_with_platform` | shell-91 검사 | `APP/application-notifications-tests.rs` 26·55줄, 모듈이 `domains::shell` 안의 비공개 `mod notifications_tests`(1643)라 settings 검사에서 보이지 않음 | 모듈·두 심볼이 `pub(in crate::application)`로 열림 | 22, 23 |
| `SyncStateChanged` 뒤 테마·로케일 재로드 | shell-93 | partial(남은 것은 Git 이벤트, settings와 무관). `APP/presentation-refresh.rs` 32~50 반영됨 | 같음 | 48 |
| 보조 창 수명 | shell-5 | partial(남은 것: 보조 창 WKWebView·터미널 raw input·Settings theme preview의 보조 창 적용, `HO-10`) | 같음 | 5 |
| 실행 중 scrollback | terminal-agent-11 | complete. `TERM/lib.rs` `TerminalCore::set_history`(465), `TERM/session.rs` `configure_history`(270), `APP/terminal_host.rs` `Hub::configure_history`(624), `APP/domains/terminal_agent.rs` `tick_terminal_agent_live_sessions`(553) | 같음 | 30 |
| 자동 reveal | explorer-search-25 | complete | 같음 | 80 |
| searchOnType·debounce | explorer-search-34 | complete | 같음 | 80 |
| 검색 치환 OS 알림 | explorer-search-33 | partial("OS 알림 한 줄" 남음, 계획 `HO-12` 3.1절) | 같음(알림 없음) | 23 |
| git pull/push OS 알림 | git-7 | unwired(Git 표면 없음) | partial로 표시돼 있으나 작업 트리에 `notify_git_remote`(`domains/git.rs` 1515)와 App 검사 `실제_app_sync의_pull과_push는_성공과_실패를_프로젝트_제목의_os_알림으로_보낸다`(`application-git-header-tests.rs` 575)가 있음(`HO-14` 3.1절) | 23 |
| IDE diff 자동 열기 | git-49 | unwired. `APP/ide-tools.rs` 209가 `AppEvent::IdeDiffRequested`를 발행하지만 받는 곳이 없어 시간 초과로 rejected | complete. 작업 트리에서 수신이 shell 수신함의 `Event::IdeDiff(event) => self.accept_claude_diff_event(event)`로 옮겨지는 중 | 80 |
| diff 설정 2종 | git-42 | unwired | partial(hideUnchanged 완료, showMoves 남음) | 28 |
| semantic token | editor-7 | missing(계획 `HO-11` 3.1절) | 같음 | 28 |
| 리거처 | editor-4 | not-applicable(egui 한계, 결정 1 A) | 같음 | 28 |
| 실행 경로 없는 명령 | commands-ui 여러 행, git, editor | wave 3 미착수(`HO-16`) | git 명령 일부 연결 | 67 |
| status 라벨 결정 공유 | commands-ui-96 | partial, settings-78 결과를 기다림(`HO-16` commands-ui-96 (d)) | 같음 | 78 |

### 1.4 권장 구현 순서

판단 기준: (1) 다른 영역 선행이 없고 settings 소유 파일만 바꾸는 행부터, (2) 같은 검사 도우미 파일을 바꾸는 행은 이어서, (3) git lane 병합에 걸린 행은 0.5절 확인이 1이 된 뒤.

1. 지금 바로(메인, 선행 없음):
    1. settings-5 — 코드 변경 없이 기존 검사 실행·기록. 가장 작음.
    2. settings-30 — `application-settings-consumers-tests.rs`에 검사 1개.
    3. settings-48 — `application-settings-sync-remote-tests.rs`에 검사 1개.
    4. settings-42 — `CR/taide-theme/src/service.rs` 검사 모듈만. 다른 행과 파일이 겹치지 않음.
    5. settings-80 minimap·searchOnType 끔 — `application-settings-consumers-tests.rs`.
    6. settings-28 App 검사 4종 — 같은 파일. 5번에서 만든 파일 열기 도우미를 함께 씁니다.
2. git lane 병합 직후(0.5절 첫 두 확인이 1):
    1. settings-22 — `Scene`에 platform·포커스 인자를 더함(이 변경을 23이 재사용).
    2. settings-80 — 병합본에서 git-49 검사 실행 → 1의 5번과 합쳐 complete.
    3. settings-28 — hideUnchanged 근거 연결(여전히 partial).
    4. settings-23 — explorer-search-33의 알림 한 줄이 들어온 뒤(0.5절 셋째 확인) 범주 묶음 App 검사.
3. 다른 영역 완료 뒤: settings-28(editor-7, git-42 showMoves), settings-67(commands-ui wave 3), settings-78(모든 lane 병합 뒤 메인이 일괄 처리, wave 4 직전 권장).

### 1.5 행을 닫을 때마다 하는 일

1. 검사 파일과 제품 코드 변경, 0.3절의 최소 검증(바꾼 crate의 관련 검사 + `cargo fmt -- --check`, UI·`taide-model`을 바꿨으면 Wasm check, 공유 crate를 바꿨으면 `cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml -p taide --locked --offline --target-dir <T>`).
2. `MATRIX`의 해당 행 4열(현재 판정)과 6열(범위/잔여 동작)을 고칩니다. 6열에는 근거 검사 이름, QA 문서 경로, 남은 것이 있으면 "남은 것: …"을 적습니다. 과대 판정 금지(4절 1).
3. `MATRIX` 상단 요약표의 settings 줄과 합계 줄, 서문 7줄을 실제 행 수로 다시 셉니다(1.1절 명령).
4. QA 문서 `docs/quality-assurance/<날짜>-native-settings-<주제>.md`를 씁니다. 형식은 `QA-S5`와 같게: 1. 대상 ID와 판정, 2. 원본 동작 근거, 3. 구현, 4. 사양·원본과 다른 점, 5. 검증 표(명령·결과), 6. 남은 범위. `docs/PROCESS.md` 진행 항목에 한 줄을 더합니다.
5. Git 커밋·푸시는 메인 오케스트레이터만 합니다. 작업자는 git 상태를 바꾸지 않습니다. `GITLANE` worktree 파일은 읽기만 합니다.

## 2. 영역 공통 지식

### 2.1 Settings 탭이 그려지는 경로

1. `APP/main.rs` → `NativeApplication`(`APP/application.rs`)이 eframe `App`입니다. `logic`(1356) → `background_tick`(1071), `ui`(1361) → `frame_ui`(1383).
2. `background_tick` 순서(같은 함수 안): `tick_terminal_agent_live_sessions`(1098) → `poll`(1100, 호스트 응답 처리. 765줄 `HostReply::Settings(reply) => self.apply_settings_reply(context, reply)`, 767줄 `HostReply::SettingsFailed`) → `consume_provider_events`(1101) → `sync_theme_preview`(1102) → … → `tick_settings_reads`(1124).
3. `frame_ui`: `settings_views.begin_frame()` → `show_window`(메인) → `show_auxiliary_windows`(보조 창마다 같은 `show_window`) → `finish_window_frames`(1427: `settings_views.finish_frame()`으로 이번 프레임에 그려지지 않은 owner의 View를 버림, 다시 `sync_theme_preview`) → `finish_frame`.
4. 탭 내용: `AppSurfaces::tab_content`(`APP/domains/editor.rs` 5353)가 `TabKind::Settings`면 `show_settings_tab`, `TabKind::AppFile { target }`이면 `show_app_file_tab`을 부릅니다.
5. `show_settings_tab`은 `Views::show_with_host(ui, Owner { project, pane, tab }, &settings, locale, appearance, Host { format_time, projects, is_macos, is_dev_build, is_perf_enabled })`를 부르고 `Output`을 호스트 명령으로 바꿉니다: `changes` → `Command::UpdateSettings`, `themes` → `ThemeEdit`, `snippets` → `SnippetEdit`, `folders` → `OpenSettingsFolder`, `open_settings_file` → `app_file_views::settings_command`, `open_app_file` → `app_file_views::open_command`, `open_keybindings` → `ShellIntent::Commands(OpenKeybindings)`.
6. `show_window` 끝(`APP/application.rs` 2070~2074)에서 `pick_settings_files(frame)`와 `load_settings_requests(..)`가 화면에서 쌓인 요청을 호스트에 제출합니다.

### 2.2 상태 위치

- 앱 루트 `NativeApplication.settings_state: domains::settings::State`(필드: `settings_views`, `app_file_views`, `settings_appearance`, `resolved_theme`, `preview_theme`, `presentation_changes`, `presentation_refresh`, `presentation_failures`, `presentation_system_language`, `sections_ports`, `sections_events`, `reported_failures`). `NativeApplication`의 필드는 비공개지만 검사 모듈은 `crate::application`의 자손이라 그대로 읽습니다.
- 호스트 section의 조회 캐시·진행 상태는 UI `Views` 안의 `Data` 하나를 모든 Settings owner(메인·보조 창)가 공유합니다(`Views::sections_data_mut`). owner별로 돌려줘야 하는 결과(VSIX 추출, 충돌 대화상자, 발급 URL, perf 결과)는 ticket으로 주소를 붙입니다(`QA-S3` 4절 4, `QA-S4` 4절 3).
- owner별 retained 상태는 `View`(활성 TOC `active`, 카탈로그 `catalog`, section 상태 `host: OwnerState`)입니다. 한 프레임에 그려지지 않은 owner는 `Views::finish_frame`의 `retain(seen)`으로 사라집니다(원본 컴포넌트 unmount와 같음). 그래서 Settings 탭이 비활성인 동안에는 `inspection()`에 그 owner가 없습니다.
- 앱 공용 상태(설정 값)는 `services.state.settings`(`parking_lot::RwLock<Settings>`)입니다. 디스크는 `services.state.paths.settings_file()`, 테마 파일은 `services.state.paths.themes_dir()`.

### 2.3 호스트 명령·응답·사건·intent의 흐름

- 루트 enum은 `APP/host.rs`: `HostCommand::Settings(domains::settings::Command)`(22줄), `HostReply::Settings(domains::settings::Reply)`(33줄), 공용 실패 `HostReply::SettingsFailed(AppError)`(35줄). worker 분배는 `host.rs` 345줄 `HostCommand::Settings(command) => domains::settings::dispatch(worker, command).await`.
- 호스트 section 행동 하나의 왕복(예: 시험 알림):
    1. UI 버튼 클릭 → `Data::send_test_notification(title, body)`(`UI/settings-sections-view.rs` `notifications` 안, 제목 `settings.notificationsSendTest`, 본문 `notification.enableHint`) → `Data::act(Action::SendTestNotification { title, body, any_window_focused })`. `any_window_focused`는 `tick_settings_reads`가 매 프레임 `observe_window_focus(self.is_any_window_focused(context))`로 넣은 값입니다.
    2. 같은 프레임 끝 `load_settings_requests`가 `Data::take_requests()`를 꺼내 `HostCommand::Settings(Command::Sections { request, ports })`로 제출. 제출 실패면 `request.failed(..)`를 `Data::accept`에만 넣습니다(실패 toast는 `submit`이 이미 띄우므로 중복 없음, `QA-S5` 7.3).
    3. worker `dispatch`의 `Command::Sections` 분기(430~466)가 `spawn_transient`로 `execute_sections`(470)를 돌리고 `HostReply::Settings(Reply::Sections(reply))`를 보냅니다. 예외: `Action::UpdateSettings`는 순서 보장을 위해 spawn 없이 바로 실행합니다(432~438).
    4. 다음 프레임 `apply_settings_reply` → `Reply::Sections`(1085, `closing`이면 버림) → `accept_sections_reply`(1094) → `Data::accept`가 돌려준 `Notice`를 toast로(`Success`·`SuccessWith`·`Warning`·`Failure`·`Error { error, fallback }`·`SettingsFailed`).
- 일반 설정 스위치·숫자: `Output.changes` → `HostCommand::Settings(Command::UpdateSettings(change))` → `dispatch` 398~428 → `crate::host::update_control_settings(services, reconcile, patch)` → 저장 + `AppEvent::SettingsChanged` + 통합 재조정. 검사는 이 경로를 `scene.application().submit(HostCommand::Settings(Command::UpdateSettings(change)))`로 직접 탑니다(선례 `APP/application-settings-consumers-tests.rs` `update` 95줄).
- 사건(`AppEvent`) 수신: `PaintSink::publish`(`APP/application.rs` 62~73)가 `trees`·`file_indexes`·`presentation`·`title_branches`·`app_menu`·`providers`·`settings` 기록기에 사건을 넣습니다. settings 기록기 `Events::record`는 `LspInstallProgress`, `SyncStateChanged`, `RemoteStateChanged`의 마지막 값만 보관하고 `tick_settings_reads`가 `Data`로 옮깁니다(Settings가 닫혀 있어도). 테마·설정 revision은 `presentation_refresh::Changes`(`SettingsChanged`·`ThemeChanged`·`SyncStateChanged`는 `revision`, `ThemeChanged`·`SyncStateChanged`는 `themes`, `SettingsChanged`는 `settings`)가 세고, `poll_settings_revisions`(939)가 `Views::observe_theme_revision`·`app_file_views.observe_settings_revision`으로 무효화합니다. presentation 재적재는 `tick_settings_reads` 끝의 `presentation_refresh.next(..)` → `Command::RefreshPresentation` → `Reply::Presentation`(성공이면 `resolved_theme` 교체 + `apply_presentation_appearances`, 실패면 `log::warn!` + `presentation_failures.theme = true`).
- intent: `ShellIntent::Settings(intents::settings::Intent)`(`UI/intents/settings.rs`: `OpenSettings`, `OpenSettingsFile`, `ChangeEditorFontSize { increase }`, `#[cfg(feature = "native-host")] ToggleEditorMinimap`) → `apply_settings_intent`(1142). settings 전용 `ShellSurfaces` 메서드는 없습니다(`docs/research/2026-10-28-native-lane-seams.md` 186줄).

### 2.4 검사 하네스

#### UI 하네스 (`UI/settings-sections-view-tests.rs`, 모듈 `settings_view::sections_view::tests`)

`Scene::new()`(40줄)가 임시 `AppState`, `taide-dark` 테마, 영어 locale로 `Views`를 만들고, `Scene::draw(events, hosted)`(69줄)가 `context.run_ui`로 한 프레임을 그립니다(`hosted == true`면 `show_with_host`). 호스트 응답은 `Scene::answer(|query, generation| Some(Reply::..))`(126줄)로 직접 넣고, 행동 요청(`Action`)은 반환값으로 받습니다. 실행: UI lib 명령 + 필터 `sections_view`.

#### 앱 하네스 (`APP/application-settings-sections-tests.rs` `Scene`, 모듈 `domains::settings::settings_sections_tests`)

```rust
let mut scene = Scene::launch(None, |state| { /* 시작 전 설정 */ });  // Settings 탭이 열린 프로젝트 1개("sections")
let mut scene = Scene::launch_empty_pane(|_| {});                     // 같은 프로젝트, 빈 pane(Welcome 탭도 비움)
scene.ports(Ports { agents: missing_cli_agents(), ..Default::default() }); // 생성 뒤 포트 교체
let label = scene.text("settings.notificationsSendTest");             // locale 문구
let output = scene.until("phase", |application, output| !nodes(output, &label).is_empty());
scene.click(nodes(&output, &label)[0].center());                       // 누름+뗌 프레임 + 빈 프레임
scene.step(vec![egui::Event::Text(";".into())]);                      // 한 프레임
assert!(successes(scene.application()).contains(&label));              // toast 확인(errors, successes)
```

- `Scene::assemble`(58줄)은 임시 디렉터리에 `data/`와 `project/`를 만들고 `isolated_state(data)`, `insert_project`, Settings 탭 layout, 메모리 가짜 Keychain(`Keychain` 544줄)으로 `NativeApplication::new_with_secret_store`(`APP/application.rs` 290)를 부릅니다. 실제 Keychain을 읽지 않게 하려는 것입니다(`QA-S4` 4절 10).
- `Scene::step`(168줄)은 `logic` 다음 `run_ui(ui)`를 돌리고 `textures_delta`를 비웁니다. `RawInput.focused`를 지정하지 않아 항상 `true`입니다(settings-22에서 바꿉니다).
- `Scene` 필드(`fixture` 등)는 비공개라 형제 모듈(`settings_consumers_tests` 등)에서 `scene.fixture`를 쓸 수 없습니다. 프로젝트 경로는 `application.services.state.projects.read()[&project].root`(문자열)로 읽습니다.
- 파일 편집기 탭이 필요하면 `실제_auto_tab_토글은_편집기_자동완성_요청_경로를_켠다`(770~892줄)가 선례입니다: 프로젝트 폴더에 파일을 쓰고 `HostCommand::Editor(crate::domains::editor::Command::OpenFileTab { project, pane: None, path, preview: false })`를 `submit`, `application.files.get(&path)`와 `application.store.views().for_document(id)`로 view를 기다리고, `(view.id, document, view.key.tab.clone())`을 얻습니다. 탭 전환은 `application.controller.submit(ShellMutation::ActivateTab(tab))`.
- 다른 검사 파일에서 쓰는 공용 도구: `crate::application::project_rail_tests::{Fixture, isolated_state, insert_project, project_ref, launch, pointer, step, node_rect}`, `crate::application::auxiliary_window_tests::{install_viewports, file_tab, auxiliary_window, viewport_of, step_until}`(보조 viewport), `crate::application::inlay_hints_tests::key`(키 사건), `crate::application::search_panel_tests::{Launched, launch_project, texts, role_rect, message, click}`(검색 패널).
- 이 기계에는 실제 `/usr/local/bin/taide`가 있어 CLI 상태 문구가 화면에 끼어듭니다. Settings 탭을 그리는 새 검사는 항상 `agents: missing_cli_agents()` 포트를 넣습니다(`QA-S1` 4절 11).
- 실행 경로 기록 형식(QA에 적는 것): `main.rs → NativeApplication → show_window → domains::settings::show_settings_tab → Views::show_with_host → sections_view::<section> → load_settings_requests → domains::settings::dispatch`.

#### 검사 모듈 가시성 정리 (재사용 전에 확인)

| 모듈(선언 위치) | cfg | 다른 영역 검사에서 보이는가 |
| --- | --- | --- |
| `domains::settings::settings_sections_tests`(`domains/settings.rs` 1645) | `test` | settings 검사끼리 `super::settings_sections_tests::…`로 씀. `pub(super)` 항목만 |
| `application::project_rail_tests`(`application.rs` 2961) | `all(test, unix)` | `crate::application`의 자손이면 `pub(super)` 항목이 보임 |
| `application::search_panel_tests`(`application.rs` 3010) | `test` | 같음. `open_search_view`(138)·`panel_generation`(154)은 비공개 |
| `application::error_boundary_tests`(`application.rs` 2990) | `all(test, unix)` | `texture_ids`(211)는 비공개 |
| `domains::shell::notifications_tests`(`domains/shell.rs` 1643) | `all(test, unix)` | 메인: 모듈이 비공개라 안 보임. `GITLANE` 병합 뒤: `pub(in crate::application)` |

`project_rail_tests`가 unix 전용이라 settings 검사도 사실상 unix에서만 컴파일됩니다. unix 전용 모듈을 직접 가져오는 새 검사 함수·모듈에는 `#[cfg(unix)]`(함수) 또는 `#[cfg(all(test, unix))]`(모듈)를 붙입니다(선례 `GITLANE/native/taide-native-app/src/application-git-header-tests.rs` 573~574줄).

## 3. 행별 상세

각 행은 (a) ID·기능명·상태, (b) 원본 TS 동작, (c) native 현재 자산과 정확히 빠진 것, (d) 구현 방법, (e) 자동 검사, (f) 완료 판정, (g) 막는 요소 순서입니다.

---

### settings-5

**(a)** settings-5 · 탭 소유권(보조 창 포함 프로젝트별 Owner) · partial(근거 묶음 shell-window, 기능표 554줄)

**(b) 원본**
- `TS/widgets/settings-view/settings-view.tsx` 98~106 문서 주석: `projectId`는 이 탭을 가진 프로젝트이고 전역 활성 프로젝트가 아닙니다. 탭이 `layoutMoveTabToWindow`로 보조 창에 가면 그 창은 자기 프로젝트로 고정되고, Settings에서 여는 새 탭(settings.json, 프롬프트 3종)은 이 프로젝트의 layout에 열려야 합니다.
- 컨테이너 상태(110~115줄: `activeSectionId` 초깃값은 첫 TOC 항목 = Appearance, `issuedRemoteUrl`, `isSyncConflictOpen`)는 탭이 다른 창으로 옮겨져 다시 mount되면 초기화됩니다.
- 125줄 `useOpenAppFileTab(projectId)`, 131줄 `handleOpenSettingsFile`. `TS/entities/layout/layout.query.ts` 117~126 `useOpenAppFileTab`: 그 창의 focused pane(`currentWindowFocusedPane(layout)`)에 `preview: false`로 열고, 실패하면 `toast.error(describeIpcError(error))`.
- 문자열: `app.openSettingsFile`, `prompts.editEntry`, `prompts.autoTabTitle`(프롬프트 제목), `tab.moveToNewWindow`, `settings.title`.

**(c) native**
- 자산: `UI/settings-owner.rs` `Owner { project, pane, tab }`, `APP/app-file-views.rs` `open_command(project, layout, scope, target, title)`(보조 창이면 그 창 focused pane, 다른 프로젝트면 `None`)·`settings_command`(위임), `show_settings_tab`이 `self.scope`를 넘김, `Views::finish_frame`의 `retain(seen)`.
- 검사는 이미 있습니다: `APP/application-settings-plugins-tests.rs` `실제_보조_창_settings는_자기_프로젝트_창_pane에_settings_json과_프롬프트를_열고_닫히면_메인에서_새로_mount된다`(558줄~). 탭 메뉴 `tab.moveToNewWindow`로 옮김 → `ShellMutation::ActivateProject(B)` → 보조 viewport에서 프롬프트 연필·settings.json이 A의 보조 pane에 열림, B 탭 수 불변 → 창 닫기 → 탭 메인 복귀 → 새 owner의 `active == Section::Appearance`, 보조 owner 없음.
- 빠진 것: 이 검사를 shell lane이 병합된 코드에서 돌린 기록뿐입니다. lane에서는 shell lane이 없어 사양 3절 주의 5("shell-5 병합 뒤")를 못 채웠습니다(`QA-S3` 7.1). 메인에는 shell(`4fa4beff`)과 settings(`3864e1db`)가 모두 병합됐고, 병합 게이트가 앱 lib 전체를 돌렸지만 검사별 기록이 없습니다(`docs/PROCESS.md` W1 항목은 "흔들림 2건 직렬 통과"만 적음).
- 혼동 주의: shell-5의 남은 "Settings theme preview의 보조 창 적용"은 이 행이 아닙니다. 원인은 `sync_theme_preview`(`domains/settings.rs` 1218)가 `context.viewport_id()`(호출 위치 둘 다 루트 context)로 `Views::preview(viewport, ..)`를 고르기 때문이며 shell-5 소유입니다(`HO-10`).

**(d) 구현 방법**: 코드 변경 없음.
1. (e) 명령으로 검사를 한 번 실행합니다.
2. 통과하면 `MATRIX` settings-5 행을 complete로 바꾸고 6열의 "남은 것" 문장을 "병합본 `<HEAD 해시>`에서 `실제_보조_창_settings는` 통과(명령: …)"로 바꿉니다.
3. 실패하면 실패 메시지의 `phase=` 값으로 단계를 찾습니다: `settings tab moved to a new window`면 shell 탭 메뉴 경로(shell-78·shell-5), `prompt in the auxiliary pane`·`settings.json in the auxiliary pane`이면 `app_file_views::open_command`, `tabs return to the main window`면 보조 창 닫힘 복귀(`return_auxiliary_window_tabs`, shell), `settings mounted again in the main window`·`the auxiliary owner is dropped with its window`면 `Views::finish_frame`의 retain. 원인이 shell 영역 코드면 고치지 말고 shell-5 행 비고와 QA에 재현 조건을 적어 메인에 보고합니다.

**(e) 자동 검사**: `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- 실제_보조_창_settings는 --test-threads=2`. 기대: 1 passed.

**(f) 완료 판정**: 메인 HEAD에서 위 검사 통과 + 행·QA·PROCESS 갱신.

**(g) 막는 요소**: 없음. 실기 다중 창 포커스는 허락 항목(배치 31)이며 이 행의 자동 판정과 무관합니다.

---

### settings-22

**(a)** settings-22 · Notifications: 시험 알림 보내기와 suppressed 사유 toast · partial(integrations, 기능표 571줄)

**(b) 원본** `TS/widgets/settings-view/settings-notification-section.tsx`
- 23~27: 사유 → 라벨 키. `notificationsDisabled` → `settings.notificationsEnabled`, `categoryDisabled` → `settings.notificationsError`, `windowFocused` → `settings.notificationsOnlyWhenUnfocused`.
- 35: 범주 `error`.
- 48~63: `sendNativeNotification({ category: 'error', title: t('settings.notificationsSendTest'), body: t('notification.enableHint') })`를 **직접** 부릅니다(공통 진입점 `notifyNative`가 아님). 그래서 200 코드포인트 자르기·창 종류 검사·첫 전달 안내(`notification.enableHint` info)가 없습니다. 결과: suppressed면 `toast.warning(t(사유 라벨))`, delivered면 `toast.success(t('notification.enableHint'))`, 예외면 `toast.error(describeIpcError(error))`.
- 120~123: 두 번째 버튼(Bell 아이콘 + `settings.notificationsSendTest`, outline xs).
- 판정 순서는 마스터 → 범주 → 포커스(`CR/taide-notification/src/service.rs` `decide_delivery` 17줄). 기본 설정은 모두 켜짐이고 `notificationsOnlyWhenUnfocused`도 켜짐이라, 포커스가 있으면 항상 `windowFocused`로 억제됩니다(`CR/taide-model/src/settings.rs` 571~579).

**(c) native**
- 자산(모두 완료): 버튼 `UI/settings-sections-view.rs` `notifications`(891~948), `Data::send_test_notification`(664), 응답 → 알림 `Data::accept`의 `Reply::TestNotification`(1168~1176: `Delivered` → `Notice::Success("notification.enableHint")`, `Suppressed(reason)` → `Notice::Warning(suppression_label(reason))`, `Err` → `Notice::Error`), 호스트 `execute_sections`의 `Action::SendTestNotification`(`APP/domains/settings.rs` 808~829: `notification_actions::notification_notify(&worker.state, worker.platform.0.as_ref(), NotificationCategory::Error, &title, &body, || any_window_focused)`, `worker`는 `services.clone()`이라 `services.platform`을 씀), 포커스 계산 `is_any_window_focused`(`APP/domains/shell.rs` 1369: 메인 `input.raw.focused` 또는 열린 보조 viewport 하나라도 포커스).
- 이미 있는 검사: [C] `UI/settings-sections-data-tests.rs`(사유 3종 → 라벨, 전달, 오류), [A] `APP/application-settings-interface-tests.rs` `실제_알림_버튼은_가짜_설정열기와_시험알림_억제_사유를_원본_문구로_알린다`(443~518: 포커스·마스터 off·`notifyError` off 세 억제).
- 빠진 것: 실제 App에서 (1) 포커스 없음 → 플랫폼 1회 전달(제목·본문) → success `notification.enableHint`, 첫 전달 info toast 없음, (2) 플랫폼 오류 → 그 문구 error toast. lane에는 생성자 platform override가 없어 못 했습니다(`QA-S1` 4절 10). 메인에는 `Overrides { secret_store, platform }`과 `new_with_overrides`(313)가 있습니다.
- 재사용할 가짜 platform: shell 검사의 `FakePlatform`(`APP/application-notifications-tests.rs` 26~53: `sent: Mutex<Vec<(String, String)>>`, `sent()` 메서드, `send_notification`은 항상 기록 후 `Ok`). 오류를 낼 수 없으므로 거절 필드를 더해야 합니다. 이 모듈은 메인에서 보이지 않고 git lane 병합 뒤 보입니다(2.4절 가시성 표).

**(d) 구현 방법** (제품 코드 변경 없음, 검사 도우미와 검사만. 0.5절 둘째 확인이 1일 때)
1. `APP/application-notifications-tests.rs`(shell 소유 검사 파일)의 `FakePlatform`에 거절 필드를 더합니다. `Default` 파생이 그대로 동작하고 기존 검사(`notifications_tests`, git 헤더 검사)는 영향이 없습니다.

    ```rust
    #[derive(Default)]
    pub(in crate::application) struct FakePlatform {
        pub(super) sent: Mutex<Vec<(String, String)>>,
        pub(in crate::application) refusal: Mutex<Option<String>>,
    }
    ```

    `send_notification` 맨 앞에 다음을 둡니다.

    ```rust
    if let Some(reason) = self.refusal.lock().unwrap().clone() {
        return Err(AppError::Internal(reason));
    }
    ```

2. `APP/application-settings-sections-tests.rs` `Scene`:
    - 필드 `is_focused: bool`을 더하고 `assemble`에서 `true`로 초기화, `step`의 `RawInput`에 `focused: self.is_focused,`를 넣습니다. `pub(super) fn set_focused(&mut self, is_focused: bool)`을 더합니다.
    - `assemble`에 인자 `platform: Option<Arc<dyn taide_runtime::PlatformServices>>`를 더하고, `NativeApplication::new_with_secret_store(..)` 호출을 아래로 바꿉니다. 기존 `launch_with`·`launch_empty_pane`은 `None`을 넘깁니다.

    ```rust
    NativeApplication::new_with_overrides(
        &eframe::CreationContext::_new_kittest(context.clone()),
        runtime,
        state,
        tasks,
        fixture.directory.join("bin/taide"),
        Vec::new(),
        crate::application::Overrides {
            secret_store: Some(secrets.unwrap_or_else(|| {
                SecretStoreState::with_store(Arc::new(Keychain::default()))
            })),
            platform,
        },
    )
    .unwrap(),
    ```

    - 새 생성 함수(이름은 shell의 자유 함수 `launch_with_platform`과 헷갈리지 않게 다르게 둡니다):

    ```rust
    /// Launches the Settings scene whose host notifications reach `platform` instead of the OS.
    pub(super) fn launch_on_platform(
        platform: Arc<dyn taide_runtime::PlatformServices>,
        configure: impl FnOnce(&AppState),
    ) -> Self {
        Self::assemble(None, configure, true, true, Some(platform))
    }
    ```

    `Arc<FakePlatform>`을 넘기면 함수 인자 위치에서 `Arc<dyn PlatformServices>`로 자동 변환됩니다.
3. `APP/application-settings-interface-tests.rs`에 검사 1개를 더합니다(아래 (e)). 이 파일의 비공개 도우미 `warnings`(37)·`click_label`(47)과 가져온 `errors`·`successes`·`nodes`·`missing_cli_agents`, `Kind`를 그대로 씁니다.

**(e) 자동 검사** (`APP/application-settings-interface-tests.rs`)

이름 예: `실제_시험_알림은_창_포커스가_없으면_가짜_플랫폼에_한번_보내고_첫전달_안내없이_성공을_알리며_플랫폼_오류를_알린다`. `#[cfg(unix)] #[test]`를 붙이고 함수 안에서 `use crate::domains::shell::notifications_tests::FakePlatform;`.

1. `let platform = Arc::new(FakePlatform::default()); let mut scene = Scene::launch_on_platform(platform.clone(), |_| {});`, `scene.ports(Ports { agents: missing_cli_agents(), ..Default::default() });`
2. `test = scene.text("settings.notificationsSendTest")`, `hint = scene.text("notification.enableHint")`, `scene.until("test button", |_, output| !nodes(output, &test).is_empty());`
3. `scene.set_focused(false);` 뒤 `scene.step(Vec::new())`를 두 번(첫 프레임이 `raw.focused = false`를 적용하고, 둘째 프레임의 `logic`이 그 값을 `Data`에 기록). 그다음 `let context = scene.context().clone(); assert!(!scene.application().is_any_window_focused(&context));`
4. `click_label(&mut scene, &test);` → `scene.until("delivered", |application, _| successes(application).contains(&hint));`
5. 단언: `platform.sent() == vec![(test.clone(), hint.clone())]`; `warnings(scene.application())`이 비어 있음; info toast 없음 — `scene.application().toasts.inspection().into_iter().filter(|toast| toast.kind == Kind::Info && toast.title == hint).count() == 0`(shell 검사 `hint_count` 121줄과 같은 판별).
6. `*platform.refusal.lock().unwrap() = Some(REFUSED.into());`(`const REFUSED: &str = "fixture platform refused";`) 다시 `click_label` → `scene.until("refused", |application, _| errors(application).contains(&REFUSED.to_string()))`, `platform.sent().len() == 1` 유지. `describe_error`는 `AppError::Internal(value)`를 `value` 그대로 돌려줍니다(`UI/toast.rs` 1589~1608).
7. 실행: 앱 lib 명령 + 필터 `실제_시험_알림은 실제_알림_버튼은 notifications_tests 실제_app_sync의_pull과_push는 --test-threads=2`(기존 억제 검사, `FakePlatform`을 쓰는 shell·git 검사 회귀 포함). `Scene`을 바꿨으므로 settings 앱 검사 전체 회귀도 한 번: 필터 `domains::settings::`.

**(f) 완료 판정**: 위 검사 통과, 기존 억제 검사 통과, 행 complete(6열에 "전달 성공·플랫폼 오류는 `<검사 이름>`"). 실제 OS 알림 표시는 허락 항목이라 완료 판정에 넣지 않습니다(`SPEC` 4.2-1).

**(g) 막는 요소**: git lane 병합(0.5절 둘째 확인). 병합 전에 메인에서 `notifications_tests`의 공개 범위를 직접 넓히지 마십시오. `GITLANE` 작업 트리가 같은 줄을 바꾸고 있어 병합 충돌이 납니다. 병합 전이면 1.4절 1단계 행을 먼저 합니다. 클릭이 안 먹으면 `phase=` 메시지로 드러나니 그때 (e) 3·4단계의 프레임 수를 확인합니다.

---

### settings-23

**(a)** settings-23 · OS 알림 발송(agent 완료, 입력 대기, task 완료, git, 검색 치환, LSP 설치, 오류) · partial(integrations, 기능표 572줄)

**(b) 원본**
- 공통 진입점 `TS/entities/notification/notify.ts` 37~50 `notifyNative`: 주 창이고 원격 미러가 아니어야 보냄(38), 제목·본문을 200 코드포인트로 자름(42~43), delivered면 첫 전달 안내 bridge 발행(45), 실패는 삼킴(47~48).
- 생산자 `TS/app/providers/native-notification-provider.tsx`: agent 완료·입력 대기(156~157, 범주 `agentCompleted`/`agentAwaitingInput`), 명령 완료(182~183, 성공 `taskCompleted`/실패 `error`), LSP 설치(199~200, 성공 `lspInstall`/실패 `error`), 첫 전달 안내.
- git: `TS/entities/git/git.query.ts` 190~193 `GIT_REMOTE_NOTIFICATION_TITLE_KEY`, 214~237 `useGitRemoteMutation`. push·pull 성공 → 범주 `gitRemote`(223), 제목 = 프로젝트 표시명(없으면 `notification.git{Push,Pull}Succeeded`), 본문 `joinNotificationBody([성공 문구, 캐시된 branch])`; 실패 → 범주 `error`(232), 제목 같은 규칙(`...Failed`), 본문 `[실패 문구, describeIpcError(error)]`.
- 검색 치환: `TS/widgets/search-panel/search-panel-container.tsx` 124~162 `handleReplaceAll`. 성공 시 `toast.success(replaceSummary)`(133)와 `notifyNative({ category: 'searchReplace', title: 프로젝트 표시명 또는 t('notification.searchReplaceDone'), body: replaceSummary })`(134~142). `replaceSummary = t('search.replaceDone', { files, matches })`.
- 설정 스위치(`settings-notification-section.tsx` 67~114): 마스터 `notificationsEnabled`, `notificationsOnlyWhenUnfocused`, 범주 7종 `notifyAgentCompleted`·`notifyAgentAwaitingInput`·`notifyTaskCompleted`·`notifyGitRemote`·`notifySearchReplace`·`notifyLspInstall`·`notifyError`(라벨 키 `settings.notificationsAgentCompleted` 등). 범주별 게이트는 `CR/taide-notification/src/service.rs` `decide_delivery` 17·`is_category_enabled` 33, 단위 검사 72~191(모든 범주, 마스터, 포커스, 사유 순서).

**(c) native**
- 완료된 것(shell-91): `APP/domains/shell.rs` `consume_provider_events`(1112~1170), `notify_event`(1175~1198: `native_notifications::project_title`, `truncate(.., NOTIFICATION_TEXT_MAX_CODE_POINTS)`, `is_any_window_focused`, `HostCommand::Shell(Command::Notify(request))`), `notify_agent_completions`(1200), `notify_command_finished`(1227, `TASK_COMPLETION_NOTIFY_MIN_DURATION_MS` = 10,000 미만은 무시), `APP/native-notifications.rs`(`evaluate` 71: 10초 이상 working 뒤 `Idle`·`AwaitingInput`만 완료), 검사 `APP/application-notifications-tests.rs`(명령 완료·LSP·agent, 가짜 platform).
- `GITLANE` 작업 트리(미커밋, 병합 중): `domains/git.rs` `notify_git_remote`(1515~1544)를 `MutationOp::Pull`·`Push` 응답(1393·1418)에서 부름, 검사 `application-git-header-tests.rs` `실제_app_sync의_pull과_push는_성공과_실패를_프로젝트_제목의_os_알림으로_보낸다`(575~). `GITLANE` 기능표 git-7은 아직 "남은 것: pull/push OS 알림 4종"으로 적혀 있습니다(`HO-14` 3.1절이 병합 결과 확인을 맡음).
- 빠진 것:
    1. 검색 치환: `APP/domains/explorer_search.rs` `apply_search_replaced`(810~865)는 success toast(844)와 건너뜀 warning만 내고 `notify_event`를 부르지 않습니다(메인·`GITLANE` 모두). explorer-search-33의 "남은 것: OS 알림 한 줄"과 같고, 구현 계획은 `HO-12` 3.1절에 있습니다.
    2. git push/pull: 메인에는 없음. git lane 병합으로 들어옵니다.
    3. 범주 묶음 App 검사 `APP/application-settings-notifications-tests.rs`가 없습니다(lane에서는 대체 구현 금지 규칙으로 쓰지 않음, `QA-S5` 4절 1).

**(d) 구현 방법**
1. 검색 치환 알림은 explorer-search 소유입니다(`HO-12` 3.1절). 그 문서를 따라 `apply_search_replaced`의 성공 분기에서 `self.toasts.success(summary, now)` 전에 `summary`를 복제해 `self.notify_event(context, taide_model::notification::NotificationCategory::SearchReplace, Some(&project), "notification.searchReplaceDone", summary.clone())`을 부릅니다. 오류 분기(829~833)는 원본에도 알림이 없으므로 그대로 둡니다. settings 작업자가 대신 넣는 경우에도 explorer-search-33 행 비고와 그 영역 QA를 함께 고칩니다.
2. git push/pull 알림은 git lane 병합으로 들어옵니다. 병합 뒤 0.5절 첫 확인이 1인지 보고, `HO-14` 3.1절의 확인 목록으로 검사가 통과하는지 봅니다.
3. 범주 묶음 App 검사(settings 소유):
    - 새 파일 `APP/application-settings-notifications-tests.rs`를 만들고 `APP/domains/settings.rs` 끝 검사 모듈 목록에 `#[cfg(all(test, unix))] #[path = "../application-settings-notifications-tests.rs"] mod settings_notifications_tests;`를 더합니다.
    - settings-22에서 만든 `Scene::launch_on_platform`·`set_focused`와 shell의 `FakePlatform`을 씁니다.
    - 설정 변경 도우미 `update`(현재 `APP/application-settings-consumers-tests.rs` 95줄의 비공개 함수)를 두 번째로 쓰게 되므로 `APP/application-settings-sections-tests.rs`로 옮겨 `pub(super) fn update(scene: &mut Scene, change: Change)`로 두고 두 파일이 가져다 씁니다.

**(e) 자동 검사**
- 묶음 검사(이름 예: `실제_os_알림은_범주별_설정_스위치를_settings_경로로_끄면_그_범주만_보내지_않는다`). 상수 예: `const LONG_COMMAND_MS: u32 = 12_000;`(10초보다 김), `const AGENT_WORKED: Duration = Duration::from_secs(11);`, `const SETTLE_FRAMES: usize = 8;`, `const SESSION: &str = "settings-notify-session";`.
    - 공통 준비: `launch_on_platform(platform.clone(), |_| {})`, `agents: missing_cli_agents()` 포트, `set_focused(false)` 뒤 2프레임, `let events = scene.application().services.events.clone();`, `let context = scene.context().clone();`.
    - 범주마다 같은 3단계: (1) 사건을 낸 뒤 `scene.until(.., |_, _| platform.sent().len() == before + 1)`, (2) `update(&mut scene, Change::Switch(Switch::<범주 스위치>, false))` 후 `services.state.settings.read().<필드> == false`가 될 때까지 대기, (3) 같은 사건을 다시 낸 뒤 `SETTLE_FRAMES`만큼 `step` → `platform.sent().len()` 그대로.
    - task 완료: `events.publish(AppEvent::TerminalCommandFinished { session_id: SESSION.into(), cwd: Some("/synthetic/cwd".into()), exit_code: Some(0), duration_ms: LONG_COMMAND_MS })`, 스위치 `Switch::NotifyTaskCompleted`(필드 `notify_task_completed`).
    - 오류: 같은 사건 `exit_code: Some(1)`, 스위치 `Switch::NotifyError`.
    - LSP 설치: `AppEvent::LspInstallProgress { server_id: LspServerId("synthetic-server".into()), phase: LspInstallPhase::Done, received_bytes: 0.0, total_bytes: None, message: None }`, 스위치 `Switch::NotifyLspInstall`.
    - agent 완료·입력 대기: `application.shell_state.provider_events.take();` 뒤 `application.shell_state.provider_events.record(&AppEvent::AgentStateChanged { project_id, agents: vec![agent(AgentActivity::Working)] })` → `application.consume_provider_events(&context, start)` → `agents: vec![agent(AgentActivity::Idle)]`(완료) 또는 `AgentActivity::AwaitingInput`(입력 대기) → `consume_provider_events(&context, start + AGENT_WORKED)`. 다음 사이클은 `start`를 `AGENT_WORKED`의 2배씩 늘린 시각으로 다시 시작합니다(추적기가 working 시작 시각을 프로젝트·세션별로 기억). 선례 `APP/application-notifications-tests.rs` 265~316. 스위치 `Switch::NotifyAgentCompleted`·`NotifyAgentAwaitingInput`. `DetectedAgent { session_id, name, pid, activity, blocked_reason: None }`.
    - 마스터 off(`Switch::NotificationsEnabled`) 한 번 뒤 위 사건 전부 0회.
    - gitRemote·searchReplace 범주의 스위치 게이트는 각 생산자 검사(아래)와 `decide_delivery` 단위 검사(`카테고리_스위치를_끄면_그_카테고리만_억제된다` 101줄)가 근거입니다. 이 묶음 검사에 git 원격 fixture를 다시 만들지 않습니다.
- 생산자 검사(각 영역 파일):
    - 검색 치환: `HO-12` 3.1절의 검사(`APP/application-search-panel-tests.rs`). 그 파일의 `launch_project`(33줄)는 `project_rail_tests::launch`(기본 platform)를 쓰므로 `new_with_overrides`를 쓰는 실행 도우미가 필요합니다. git lane 병합 뒤라면 shell의 `launch_with_platform(fixture, state, platform)`을 재사용합니다.
    - git: `GITLANE`의 위 git 헤더 검사(병합 뒤 메인에서 실행).
- 실행: 앱 lib 명령 + 필터 `settings_notifications_tests 실제_app_sync의_pull과_push는 <검색 치환 검사 이름> --test-threads=2`.

**(f) 완료 판정**: shell-91(완료), explorer-search-33의 알림 줄, git-7의 알림 4종이 메인에 있고, 묶음 검사와 생산자 검사 2개가 통과하면 complete. 하나라도 없으면 partial 유지, 6열에 남은 범주와 소유 행을 적습니다. 실제 OS 표시는 허락 항목.

**(g) 막는 요소**: git lane 병합(0.5절 첫·둘째 확인), explorer-search-33의 알림 한 줄(0.5절 셋째 확인). 다른 영역 소유 파일 수정은 메인 통합 단계에서 하고 그 영역 행·QA를 함께 갱신합니다.

---

### settings-28

**(a)** settings-28 · 위 표시·입력 옵션(settings-27의 옵션들)의 에디터 반영 · partial(settings-code, 기능표 577줄)

**(b) 원본**
- `TS/shared/lib/code-editor-settings.ts` 43~65 `resolveCodeEditorSettingsProps`: minimap(기본 true), wordWrap(false), lineNumbers(true), tabSize, insertSpaces(true), detectIndentation(true), renderWhitespace(기본 `selection`), bracketPairColorization(true), fontLigatures(false), cursorStyle, cursorBlinking, scrollBeyondLastLine(true), stickyScroll(true), bracketPairGuides(false), smoothScrolling(false), cursorSmoothCaretAnimation(false), suggestPreview(false), rulers(빈 배열), aiAutoTabEnabled(false). formatOnType/formatOnPaste는 편집기 pane이 따로 읽습니다(37~42 주석).
- Monaco 적용: `TS/features/editor/code-editor.tsx` 284(`wordWrap: wordWrap ? 'on' : 'off'`), 275~276(`minimap: { enabled: minimap && !largeFile }`).
- semantic: `code-editor.tsx` 190~201은 Monaco의 `semanticHighlighting.enabled`만 켜고, 공급자는 `TS/shared/lib/lsp/adapters/semantic-tokens.ts`(173줄 주석: `settings.editorSemanticHighlighting`이 공급자 등록을 막음).
- diff 2종: `TS/shared/lib/diff-view-settings.ts` 12~15(기본 둘 다 false), `TS/features/git/diff-view.tsx` 80~81(`hideUnchangedRegions: { enabled }`, `experimental: { showMoves }`).
- emmet: `TS/app/providers/emmet-provider.tsx`.
- 키: `settings.editorWordWrap`, `settings.editorLineNumbers`, `settings.editorRulers`, `settings.editorRenderWhitespace`, `settings.editorSemanticHighlighting`, `settings.editorDiffHideUnchangedRegions`, `settings.editorDiffShowMoves`, `settings.editorFontLigatures`.

**(c) native**
- 소비 경로: 표시 옵션은 `UI/presentation.rs` `editor_presentation`(255~304)에서 `EditorDisplayOptions`(word_wrap, suggest_preview, render_whitespace, rulers, cursor, smooth caret, scroll beyond, smooth scrolling, bracket 2종, sticky scroll, minimap)로 바뀌고, 앱 `APP/presentation-refresh.rs` `editor_presentation`(219~227, 굵은 글꼴 패밀리 추가)을 `APP/domains/editor.rs` `show_document`(5485~, 호출 5562)가 **매 프레임** 부릅니다. 줄 번호는 `UI/presentation.rs` `editor_appearance`의 `line_numbers`(246)가 presentation 재적재 때 `application.editor.appearance`에 들어갑니다(설정 변경 → `SettingsChanged` → `presentation_refresh` revision → `Reply::Presentation` → `apply_presentation_appearances`, `domains/settings.rs` 1265).
- 이미 있는 근거:
    - 설정 → presentation 대응 UI 검사 `UI/../tests/editor-display.rs` `표시_설정은_기본값과_모든_선택지를_native_presentation에_공급한다`(306), `공백_모드는_탭과_unicode의_실제_좌표와_끝_제외_선택을_보존한다`(363), `rulers는_문자_열과_스크롤에_맞춰_1px로_그려지고_옵션_변경으로_회수된다`(430).
    - 앱 단위 `APP/presentation-refresh.rs` `native_editor_presentation은_설정의_word_wrap과_편집기_굵은_글꼴_패밀리를_전달한다`(667).
    - format on type/paste App 근거: `APP/application-startup-tests.rs`·`APP/application-json-tests.rs`의 `editor_format_on_type`·`editor_format_on_paste` 토글(각 4·3곳, `grep -n "editor_format_on_type\|editor_format_on_paste"`로 위치 확인), `APP/editor-formatting-tests.rs` `자동_포맷은_입력과_현재_위치_범위_설정을_보존하고_입력과_분리하여_undo한다`(480).
    - sticky scroll: `APP/application-sticky-definition-tests.rs`(116줄 설정), `APP/application-json-tests.rs`(1847줄 설정).
    - emmet: `APP/editor-completion-emmet-consumer-tests.rs`(69·73·106줄 `emmet_enabled` 토글).
- 빠진 것:
    1. 설정 경로 → 실제 App 표시 변화 검사: wordWrap, rulers, lineNumbers, renderWhitespace(`QA-S5` 6절 "파일 편집기 탭 fixture 필요"로 못 함. 지금은 2.4절의 파일 탭 패턴이 있습니다).
    2. 소비자 없음: semantic(editor-7 missing), diff showMoves(git-42, `GITLANE`에서도 미구현). diff hideUnchanged는 `GITLANE`에만 있습니다(`domains/git.rs` 2411·2774가 `editor_diff_hide_unchanged_regions`를 읽음, 검사 `application-git-diff-tests.rs` `실제_app_미변경_영역_숨김_설정은_열린_diff_탭에_바로_반영되고_끄면_모두_보인다` 640).
    3. 리거처는 의존이 아니라 not-applicable입니다(editor-4: epaint 0.36.2가 OpenType feature를 끄고 켤 수 없음, 결정 1 A). 6열의 "리거처(egui 한계)" 문장을 "리거처는 editor-4 not-applicable(결정 1 A)"로 바꿉니다.

**(d) 구현 방법** (`APP/application-settings-consumers-tests.rs`, 제품 코드 변경 없음)
1. 같은 파일에 파일 탭 도우미 하나를 둡니다(이 행의 4개와 settings-80 minimap 검사가 공유, 2.4절 auto-tab 검사 805~836줄과 같은 흐름).

    ```rust
    fn open_file(
        scene: &mut Scene,
        name: &str,
        text: &str,
    ) -> (taide_native_editor::view::ViewId, taide_native_editor::document::DocumentId) {
        let (project, _) = focused_pane(scene);
        let root = scene.application().services.state.projects.read()[&project].root.clone();
        let path = std::path::Path::new(&root).join(name);
        std::fs::write(&path, text).unwrap();
        let path = path.to_str().unwrap().to_owned();
        assert!(scene.application().submit(HostCommand::Editor(
            crate::domains::editor::Command::OpenFileTab {
                project,
                pane: None,
                path: path.clone(),
                preview: false,
            }
        )));
        scene.until("file view", |application, _| {
            application.files.get(&path).is_some_and(|document| {
                application.store.views().for_document(document.id).next().is_some()
            })
        });
        let application = scene.application();
        let document = application.files[&path].id;
        let view = application.store.views().for_document(document).next().unwrap().id;
        (view, document)
    }
    ```

    `focused_pane`(309줄)은 이 파일에 이미 있습니다. 화면은 `scene.resize(TOC_SCREEN)`(1400×900, 18줄 상수)으로 둡니다.
2. 도형 펼치기 도우미 하나(`painted`는 글자만 돌려주므로 사각형·원용):

    ```rust
    fn shapes(output: &egui::FullOutput) -> Vec<&egui::Shape> {
        let mut pending = output.shapes.iter().map(|clipped| &clipped.shape).collect::<Vec<_>>();
        let mut flat = Vec::new();
        while let Some(shape) = pending.pop() {
            match shape {
                egui::Shape::Vec(shapes) => pending.extend(shapes),
                shape => flat.push(shape),
            }
        }
        flat
    }
    ```

3. 검사 4개를 더합니다(아래 (e)). 설정 변경은 `update(&mut scene, change)`(95줄 또는 settings-23에서 옮긴 공용 도우미)를 씁니다.
4. git lane 병합 뒤 hideUnchanged 검사를 메인에서 실행하고 6열에 인용합니다.
5. editor-7(semantic)과 git-42(showMoves)가 끝나면 그 영역 검사 이름을 6열에 적고 complete로 올립니다. 그 전에는 partial.

**(e) 자동 검사** (모두 실제 `NativeApplication`, `Scene::launch_empty_pane(|_| {})` + `missing_cli_agents()` 포트 + `resize(TOC_SCREEN)`)
- wordWrap(이름 예: `실제_편집기_word_wrap은_settings_경로로_켜면_한_줄을_여러_화면_행으로_그리고_끄면_되돌린다`): 상수 `const WRAP_WORD: &str = "wrapword"; const WRAP_WORDS: usize = 120;`. 파일 `wrap.txt` 한 줄 = `(0..WRAP_WORDS).map(|i| format!("{WRAP_WORD}{i:03}")).collect::<Vec<_>>().join(" ")`. 편집기는 화면 행마다 `Shape::Text` 하나를 그립니다(`UI/editor_surface.rs` 3395~3402 `text_painter.layout_job` → `text_painter.galley`). `painted(output)` 중 `text.contains(WRAP_WORD)`인 수: 기본(꺼짐) 1, `Change::Code(CodeChange::Switch(CodeSwitch::EditorWordWrap, true))` 뒤 2 이상, 다시 `false` 뒤 1. 각 단계에서 `services.state.settings.read().editor_word_wrap`도 확인.
- rulers: `Change::Code(CodeChange::Rulers(vec![RULER_COLUMN]))`(`const RULER_COLUMN: u32 = 20;`) 뒤 `shapes(output)` 중 `egui::Shape::Rect(rect) if rect.fill == application.editor_display_colors.ruler && (rect.rect.width() - 1.0).abs() < f32::EPSILON`가 1개, `Rulers(Vec::new())` 뒤 0개(UI 검사 430줄과 같은 판별, 폭 1px은 `UI/editor-display.rs` `RULER_WIDTH` 25줄).
- lineNumbers: 상수 `const NUMBERED_LINES: usize = 12;`, 파일 `(1..=NUMBERED_LINES).map(|i| format!("row{i:02}")).collect::<Vec<_>>().join("\n")`. 켜짐(기본)이면 `painted` 중 `text == NUMBERED_LINES.to_string()`인 도형이 1개 이상(`UI/editor-gutter.rs` 170~177이 `(row.segment.line + 1).to_string()`을 그림), `CodeSwitch::EditorLineNumbers, false` 뒤 `application.editor.appearance.line_numbers == false`와 그 도형 0개, 다시 켜면 복귀. 상태 표시줄은 `Ln 1, Col 1` 형식이라 단독 숫자 "12"와 겹치지 않습니다.
- renderWhitespace: 파일 `"a b c d"`. 기본(`EditorRenderWhitespace::Selection`, 빈 선택)이면 `egui::Shape::Circle(circle) if circle.fill == application.editor_display_colors.whitespace`가 0개, `Change::Code(CodeChange::Selection(Selection::Whitespace(EditorRenderWhitespace::All)))` 뒤 3개(공백 3칸, `UI/editor-display.rs` `whitespace` 132~180의 `circle_filled`), `EditorRenderWhitespace::None` 뒤 0개. `use taide_model::settings::EditorRenderWhitespace;`.
- 실행: 앱 lib 명령 + 필터 `settings_consumers_tests --test-threads=2`(기존 5개 + 새 검사).

**(f) 완료 판정**: 새 App 검사 4개 통과 + 6열에 기존 소비자 근거 목록 + 리거처 not-applicable(editor-4) + semantic은 editor-7, diff 2종은 git-42의 완료 근거. editor-7과 git-42 showMoves가 complete가 되기 전에는 partial입니다(과대 판정 금지).

**(g) 막는 요소**: editor-7(editor 영역, `HO-11` 3.1절), git-42 showMoves(git 영역, monaco `computeMovedLines` 이식 결정 6 A, `HO-14` 순서 7), git lane 병합(hideUnchanged 근거). App 검사 4개는 지금 할 수 있습니다.

---

### settings-30

**(a)** settings-30 · Terminal section(글꼴 크기/family, shell, shell profile, scrollback+힌트, cursor, blink) · partial(settings-code, 기능표 579줄)

**(b) 원본**
- `TS/widgets/settings-view/settings-terminal-section.tsx`: 크기(46~47 `MIN_CODE_FONT_SIZE`~`MAX_CODE_FONT_SIZE`), family picker, shell 텍스트 필드(placeholder `/bin/zsh`, trim), shell profile 목록(조회 중 `settings.loading`), scrollback(77~81 `settings.terminalScrollback`, `MIN_SCROLLBACK`~`MAX_SCROLLBACK` = 100~100,000, 83 힌트 `settings.terminalScrollbackHint`), cursor `bar|block|underline`, blink 스위치.
- 실행 중 터미널 즉시 반영 `TS/features/terminal/terminal-view.tsx`: 글자 크기 222~224(재 fit), family 231~233, scrollback 244~245(`term.options.scrollback = scrollback`), cursorStyle 250~251, cursorBlink 256~257. shell은 새 터미널부터.

**(c) native**
- 완료된 것: cursor·blink·글자 크기·shell의 App 검사 `APP/application-settings-consumers-tests.rs` `실제_터미널_설정은_실행중_세션의_커서_깜빡임_글자크기에_반영되고_shell은_새_세션부터_쓴다`(328~432).
- 실행 중 scrollback 경로(terminal-agent-11, 메인에 병합됨): `background_tick` → `tick_terminal_agent_live_sessions`(`APP/domains/terminal_agent.rs` 553~556) → `Hub::configure_history`(`APP/terminal_host.rs` 624: 설정의 `terminal_scrollback`을 모든 세션에, 거절은 `log::warn!`) → `configure_history`(`TERM/session.rs` 270) → `TerminalCore::set_history`(`TERM/lib.rs` 465: 같은 값이면 `Ok(false)`, 예산 `admit` 초과면 오류로 그대로 둠).
- 빠진 것: Settings 경로(`Command::UpdateSettings`)로 바꾼 scrollback이 실행 중 세션 history에 반영되는 App 검사. terminal-agent의 검사(`APP/application-terminal-agent-tests.rs` `실제_터미널_실행중_설정은_scrollback_커서_글자크기_셸을_반영한다` 378~468)는 `services.state.settings.write()`로 메모리 값을 직접 바꿔 Settings 저장 경로를 지나지 않습니다.

**(d) 구현 방법**: 제품 코드 변경 없음. `APP/application-settings-consumers-tests.rs`에 검사 1개.
1. 기존 터미널 검사의 준비(`Scene::launch_empty_pane(|state| state.settings.write().shell_override = Some(CAT.into()))`, `focused_pane`, `apply_terminal_agent_intent(TerminalIntent::NewTerminal { project, pane })`, `"first terminal"` 대기)를 그대로 씁니다. 기존 검사 이름을 바꾸면 `QA-S5`와 기능표의 인용이 깨지므로 새 검사로 추가합니다.
2. 도우미(같은 파일의 `cursor_of` 316줄과 같은 모양):

    ```rust
    fn history_of(application: &NativeApplication, session: &str) -> usize {
        application
            .terminal_agent_state
            .terminals
            .hub()
            .get(session)
            .unwrap()
            .snapshot(|snapshot| snapshot.core.history())
            .unwrap()
    }
    ```

3. 상수 `const LIVE_SCROLLBACK: u32 = 500;`(100~100,000 안, 기본 10,000 `taide_model::settings::DEFAULT_TERMINAL_SCROLLBACK`과 다름).

**(e) 자동 검사** (이름 예: `실제_터미널_scrollback_설정은_settings_경로로_실행중_세션의_history_줄_수를_바꾼다`)
1. 첫 세션의 `history_of`가 `DEFAULT_TERMINAL_SCROLLBACK as usize`인지 단언.
2. `update(&mut scene, Change::Numeric(Numeric::TerminalScrollback, LIVE_SCROLLBACK))`.
3. `wait_texts(&mut scene, "running scrollback follows", &[], |application, _| application.services.state.settings.read().terminal_scrollback == LIVE_SCROLLBACK && history_of(application, &first.id) == LIVE_SCROLLBACK as usize)`.
4. `settings.json`을 읽어 `terminalScrollback == 500` 단언(`serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(application.services.state.paths.settings_file()).unwrap())`).
5. 실행: 앱 lib 명령 + 필터 `실제_터미널_scrollback_설정은 실제_터미널_설정은 --test-threads=2`.

**(f) 완료 판정**: 새 검사 + 기존 터미널 검사 통과 → complete. 6열에 두 검사 이름과 terminal-agent-11 근거.

**(g) 막는 요소**: 없음. `/bin/cat`은 읽기만 하는 셸 대용이며 사용자 데이터를 건드리지 않습니다.

---

### settings-42

**(a)** settings-42 · Plugins: VSIX 가져오기 대화상자(테마 후보 선택, 중복 덮어쓰기 확인, 대비 실패 표시, 문법 가져오기 3상태) · partial(integrations, 기능표 591줄)

**(b) 원본**: 사용자 동작은 `SPEC` 2E settings-42(370~388줄)에 전부 정리돼 있고 native에서 모두 구현·검사됐습니다(`TS/widgets/plugin-manager/vsix-import-dialog.tsx` 52~211, `TS/shared/lib/vsix-theme-import.ts` 43~119, `TS/features/plugin/vsix-import-grammars-section.tsx` 21~41). 남은 일과 관련된 원본은 색·대비 계산입니다: `TS/shared/lib/color.ts`(`isHexColor`, `compositeOverBackground`, `deltaE76`), `TS/shared/lib/theme-convert/contrast.ts`(`relativeLuminance`, `contrastRatio`, `foregroundContrastRatio`). `SPEC` 382줄: "`service.rs` 테스트 모듈의 색·대비 도우미는 `convert::color`·`convert::contrast`의 공개 함수로 옮기고 테스트가 그것을 부르게 합니다(같은 값, 중복 제거)". 사용자 문자열 변화는 없습니다.

**(c) native**
- 완료: 변환 이식 `CR/taide-theme/src/convert/{color,jsonc,tables,mapping,state,contrast,mod,tests}.rs`, 후보 생성 `CR/taide-runtime/src/vsix_actions.rs` `vsix_theme_candidates`, 대화상자 `UI/settings-dialogs.rs`·`UI/settings-sections-view.rs` `vsix_dialog`(2030), 앱 실행 `APP/domains/settings.rs` `extract_vsix`(915)·`VsixSaveThemes`. 검사: [C] `convert/tests.rs`(TS 기대값 그대로, `QA-S3` 7.2), [U] 대화상자, [A] 실제 `.vsix`(`APP/application-settings-plugins-tests.rs` `실제_vsix_가져오기는_테마_저장_중복_복사본과_문법_3상태를_원본대로_처리한다` 317).
- 빠진 것: `CR/taide-theme/src/service.rs`의 검사 모듈(`mod tests` 1019줄~)이 `convert::color`와 같은 계산을 따로 갖고 있습니다.
    - 지울 함수(모두 `mod tests` 안): `hex_to_rgb`(1555), `srgb_channel_to_linear`(1569), `relative_luminance`(1579), `contrast_ratio`(1590), `is_hex_color`(2961), `hex_channel`(2970), `rgb_to_hex`(2975), `composite_channels`(2982), `composite_over_background`(2999), `srgb_channel_to_cie_linear`(3010), `cie_lab_f`(3020), `hex_to_lab`(3031), `delta_e76`(3053), `foreground_contrast_ratio`(3063).
    - 대응 공개 함수(`CR/taide-theme/src/convert/color.rs`): `is_hex_color`(78), `hex_to_rgb`(85, `Option<Rgb { r, g, b }>`), `rgb_to_hex`(111, 인자 `Rgb`), `composite_over_background`(116), `delta_e76`(181), `relative_luminance`(197), `contrast_ratio`(207), `foreground_contrast_ratio`(214). 검사 모듈이 직접 부르는 6개(`is_hex_color`, `composite_over_background`, `contrast_ratio`, `foreground_contrast_ratio`, `relative_luminance`, `delta_e76`)는 시그니처(`&str` 인자, 같은 반환형)가 같아 호출부를 고칠 필요가 없습니다. `hex_to_rgb`·`rgb_to_hex`·`hex_to_lab`는 지울 함수 안에서만 불립니다(호출 1580, 2983, 2984, 2987, 3032, 3054~3055).
    - 의미 차이(옮긴 뒤 검사가 실패하면 여기서 원인을 찾음):
        1. `#` 없는 값: 검사 쪽 `hex_to_rgb`는 `normalize_hex_color`(1057)가 앞 `#`를 선택적으로 지워 `abc`도 받지만, convert 쪽은 `#`가 있어야 합니다. 번들 테마 47개에 `#` 없는 6자리 문자열은 없습니다(`grep -rhoE '"[0-9a-fA-F]{6}"' CR/taide-theme/resources/themes/` 결과 0).
        2. 4자리 hex: 검사 쪽은 `#rgba`도 받지만 convert 쪽은 원본 `isHexColor`처럼 3·6·8자리만 받습니다. 번들 테마에 4자리 hex 값은 없습니다(`grep -rlE '"#[0-9a-fA-F]{4}"'` 결과 0).
        3. 깨진 알파: 검사 쪽 `composite_over_background`는 알파 두 자리가 hex가 아니면 불투명 부분(`#rrggbb`)을 돌려주고, convert 쪽은 JS `parseInt`처럼 앞자리만 읽거나(`"1z"` → 1) 실패하면 NaN 알파로 `#000000`이 됩니다. 카탈로그 값은 모두 유효 hex라 차이가 드러나지 않아야 합니다.
        4. `is_hex_color`는 두 쪽 모두 `trim()` 뒤 판정해 같습니다(`QA-S3` 7.2의 "공백 처리가 다르다"는 문장은 실제 코드와 다릅니다).
    - 남길 것: `normalize_hex_color`(1057)는 다른 검사(1070·1075·1076·1413~1415·1446·1492·1887~1889·1927)가 씁니다. `HEX_WITH_ALPHA_LENGTH`·`OPAQUE_HEX_LENGTH`·`ALPHA_CHANNEL_MAX`·`HEX_RADIX`(2946~2949)는 지우지 않는 `shadow_alpha`(3110~3121)가 씁니다.

**(d) 구현 방법** (`CR/taide-theme`, edition 2021이라 let chain 금지)
1. `service.rs` `mod tests` 맨 위 `use` 묶음(1020~1026)에 `use crate::convert::color::{composite_over_background, contrast_ratio, delta_e76, foreground_contrast_ratio, is_hex_color, relative_luminance};`를 더합니다. 실제로 쓰는 이름만 넣습니다(남는 import는 경고). 부모 `service.rs`의 비검사 부분에는 같은 이름이 없어 `use super::*;`와 충돌하지 않습니다(1~1018줄 grep 결과 0).
2. 위 14개 함수와 그 문서 주석을 지웁니다.
3. 지운 함수만 쓰던 상수를 지웁니다: `RGB_CHANNEL_MAX`(1518), `SRGB_LINEAR_THRESHOLD`·`SRGB_LINEAR_DIVISOR`·`SRGB_GAMMA_OFFSET`·`SRGB_GAMMA_DIVISOR`·`SRGB_GAMMA_EXPONENT`·`LUMINANCE_WEIGHT_R`·`LUMINANCE_WEIGHT_G`·`LUMINANCE_WEIGHT_B`·`CONTRAST_RATIO_OFFSET`(1519~1527), `CIE_*` 15개(2928~2942). `MIN_CONTRAST_RATIO`(1516)와 2946~2949의 네 상수는 남깁니다. 지우기 전에 각 이름을 `grep -n "이름" CR/taide-theme/src/service.rs`로 세어, 지울 함수 밖 사용처가 없는지 확인합니다.
4. 원본 대응 문서(`/// Ports TS ...`)는 convert 쪽 공개 함수에 이미 영어 `///`가 있으므로 옮기지 않습니다.

**(e) 자동 검사**
- `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml -p taide-theme --lib --locked --offline --target-dir <T>`: 이전 기준 63 passed(`QA-S3` 7.3)와 같은 수가 모두 통과. 실패하면 위 의미 차이 1~3 중 무엇인지 실패 메시지의 토큰 이름으로 판단합니다. 결과를 맞추려고 convert 쪽을 바꾸지 마십시오(원본 정본은 convert 쪽).
- `cargo fmt --manifest-path /Users/hyunseokbyun/development/TAIDE/crates/taide-theme/Cargo.toml -- --check`.
- `cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml -p taide --locked --offline --target-dir <T>`(공유 crate를 바꾼 묶음의 src-tauri 컴파일 확인, `SPEC` 3절 주의 4).
- 완료 확인: `grep -c "fn hex_to_rgb\|fn relative_luminance\|fn contrast_ratio\|fn composite_over_background\|fn delta_e76\|fn foreground_contrast_ratio\|fn is_hex_color\|fn hex_to_lab" /Users/hyunseokbyun/development/TAIDE/crates/taide-theme/src/service.rs` 결과 0.

**(f) 완료 판정**: 중복 함수 0, `taide-theme` lib 검사 전부 통과, fmt 차이 없음 → complete. 6열에서 "남은 것" 문장을 지우고 "카탈로그 전용 TS 7사례·palette 우선 1사례는 변환 경로 밖(`QA-S3` 4절 7·8)" 문장은 유지합니다.

**(g) 막는 요소**: 없음.

---

### settings-48

**(a)** settings-48 · Sync: 설정 변경 후 동기화 데이터 반영 · partial(integrations, 기능표 597줄)

**(b) 원본** `TS/app/providers/ipc-sync-provider.tsx`
- 482~487 `syncStateChanged`: 동기화 상태 캐시 교체, 설정·테마·로케일 질의 무효화(테마 목록과 적용 테마 재조회).
- 413~428 `settingsChanged`: 테마 id·시스템 테마 추종이 바뀌면 테마 무효화, 언어가 바뀌면 로케일 무효화, `remoteAccessEnabled`가 바뀌면 원격 상태 무효화, settings.json app file 탭 내용 무효화(427).
- 다운로드 성공 문구 `settings.syncDownloadSuccess`(`TS/widgets/settings-view/settings-sync-section.tsx` 60·70), 버튼 `settings.syncDownloadNow`, settings.json 버튼 `app.openSettingsFile`.

**(c) native**
- 순서(`CR/taide-runtime/src/sync_actions.rs` `apply_sync_download` 280~339): 충돌 판정 `decide_download_apply`(83~101: gist 바뀜·다른 동기화 완료면 재시도 오류, `!force && is_remote_newer(remote, live)`면 충돌) → 설정 적용(`apply_and_broadcast` → `SettingsChanged`) → 테마·로케일 파일 쓰기(333~334, `CR/taide-sync/src/service.rs` `apply_theme_entries` 233: 각 `{ id, json }`을 `Theme`로 파싱해 `save_theme`) → `SyncStateChanged`(337).
- 완료된 것: `SyncStateChanged` → `Events::record` → `tick_settings_reads` → `Data::record_sync_status`(동기화 행 갱신, `QA-S4` [A] 1). shell-93이 메인에 병합돼 `presentation_refresh::Changes::record`(`APP/presentation-refresh.rs` 32~50)가 `SyncStateChanged`에서 `revision`과 `themes`를 올립니다 → 테마·로케일 재적재(`tick_settings_reads`의 `presentation_refresh.next`) + `poll_settings_revisions`가 `Views::observe_theme_revision`으로 Settings 카탈로그를 무효화. settings.json 탭은 `SettingsChanged` → `app_file_views.observe_settings_revision`(`APP/app-file-views.rs` 201~214)이 다음 표시 때 다시 읽게 합니다. 단위 근거 `APP/presentation-refresh.rs` `native_presentation_sync_적용은_테마_로케일을_다시_읽고_설정탭은_건드리지_않는다`(393).
- 빠진 것: 이 연결 전체를 한 번에 보이는 App 검사(`SPEC` 2F settings-48 [A]). 주의: 설정 적용 직후의 첫 재적재는 아직 테마 파일이 없어 실패할 수 있습니다(`theme_actions::theme_get_for_settings` → `load_theme`이 `NotFound`, `presentation_failures.theme = true`). 그 뒤 `SyncStateChanged`가 다시 재적재해 복구합니다. 검사는 최종 상태를 기다려야 합니다.
- 마지막 동기화 표시 갱신은 settings-46·47 검사가 이미 덮습니다(`application-settings-sync-remote-tests.rs` 첫 검사의 `"state event"` 단계, 충돌 검사의 force 다운로드).

**(d) 구현 방법**: 제품 코드 변경 없음. `APP/application-settings-sync-remote-tests.rs`에 검사 1개.
1. 상수와 payload 도우미(같은 파일의 `payload`(22) 옆). 앱 crate는 `taide-theme`를 직접 의존하지 않으므로(`APP/../Cargo.toml`에 없음) 내장 테마 id는 문자열 상수로 둡니다.

    ```rust
    const SYNCED_THEME: &str = "synced-theme";
    const SYNCED_THEME_NAME: &str = "Synced Theme";
    const SYNCED_ACCENT: &str = "#abcdef";
    const BUILTIN_DARK: &str = "taide-dark";

    fn theme_payload() -> String {
        let theme = serde_json::json!({
            "version": taide_model::theme::THEME_SCHEMA_VERSION,
            "id": SYNCED_THEME,
            "name": SYNCED_THEME_NAME,
            "type": "dark",
            "extends": BUILTIN_DARK,
            "colors": { "app.accent": SYNCED_ACCENT },
        });
        serde_json::json!({
            "schemaVersion": SETTINGS_SCHEMA_VERSION,
            "updatedAt": NEW,
            "settings": { "themeId": SYNCED_THEME },
            "themes": [{ "id": SYNCED_THEME, "json": theme.to_string() }],
            "locales": [],
        })
        .to_string()
    }
    ```

    `Theme`의 필수 필드는 `version`·`id`·`name`·`type`이고 나머지는 `#[serde(default)]`입니다(`CR/taide-model/src/theme.rs` 57~81). 읽을 때 `extends`의 내장 테마(없으면 같은 유형의 내장 테마)가 나머지 키를 채웁니다(`CR/taide-theme/src/service.rs` `load_theme` 995~1011). `themeId`는 동기화 대상 설정입니다(`CR/taide-sync/src/service.rs` 93).
2. 준비는 같은 파일 충돌 검사(230~)처럼 연결 상태를 미리 만듭니다: `keychain.values.lock().unwrap().insert("github", PAT.into());`, `sync_scene(&keychain, &gist, |state| { let mut settings = state.settings.write(); settings.sync_gist_id = Some(GIST.into()); settings.sync_last_synced_at = Some(NEW.into()); })`. 가져온 시각도 `NEW`라 `is_remote_newer`가 거짓이어서 충돌이 아니라 적용됩니다.
3. `sync_scene`은 `gist` 포트만 넣으므로 이 검사는 바로 뒤 `scene.ports(Ports { gist: Gist(gist.clone()), agents: missing_cli_agents(), ..Default::default() })`로 다시 넣습니다. `missing_cli_agents`를 `super::settings_sections_tests`에서 가져오는 import에 더합니다.

**(e) 자동 검사** (이름 예: `실제_sync_다운로드는_새_테마와_테마_설정을_적용해_테마_목록과_화면_테마와_settings_json_탭을_갱신한다`)
1. `let gist = Arc::new(FakeGist::new(NEW, FONT)); *gist.fetched.lock().unwrap() = Ok((NEW.into(), theme_payload()));` 뒤 2·3단계 준비.
2. Settings 탭 id를 기억합니다(auto-tab 검사 798~804줄처럼 layout root `PaneNode::Leaf`의 `active`).
3. settings.json 탭 열기: `app.openSettingsFile` 문구를 `nodes`로 찾고 없으면 `texts_of`로 찾아 클릭합니다(settings-5 검사의 `click_in`과 같은 방식). `TabKind::AppFile { target: AppFileTarget::Settings }` 탭이 layout에 생기고 `application.store.documents().find(&taide_native_editor::document::DocumentKey::AppFile(AppFileTarget::Settings))`가 `Some`일 때까지 대기. 그 탭 id를 기억합니다.
4. `application.controller.submit(ShellMutation::ActivateTab(settings_tab))` → 다운로드 버튼(`settings.syncDownloadNow`) 노드가 보일 때까지 대기 → `click_label` → `successes`에 `settings.syncDownloadSuccess`, `settings.read().theme_id == SYNCED_THEME`.
5. 테마 파일 `services.state.paths.themes_dir().join("synced-theme.json")` 존재.
6. 화면 테마: `application.settings_state.resolved_theme.id == SYNCED_THEME && !application.settings_state.presentation_failures.theme`가 될 때까지 대기(첫 재적재 실패를 허용).
7. 카탈로그(Settings 탭이 활성인 동안에만 owner View가 있음, 2.2절): `application.settings_state.settings_views.inspection().values().any(|view| view.catalog.as_ref().is_some_and(|catalog| catalog.themes.as_ref().is_ok_and(|themes| themes.iter().any(|theme| theme.id == SYNCED_THEME))))`가 될 때까지 대기(`inspection` feature는 앱 dev-dependency에서 켜짐, `APP/../Cargo.toml` 106줄).
8. `ShellMutation::ActivateTab(app_file_tab)` → settings.json 문서 본문(`application.store.documents().snapshot(id).unwrap().rope.to_string()`)을 `serde_json::from_str::<serde_json::Value>`로 읽어 `["themeId"] == SYNCED_THEME`가 될 때까지 대기.
9. 실행: 앱 lib 명령 + 필터 `settings_sync_remote_tests --test-threads=2`(기존 4개 회귀 포함).

**(f) 완료 판정**: 새 검사 통과 + 기존 동기화 검사 통과 → complete. 6열에 shell-93 근거와 검사 이름.

**(g) 막는 요소**: 없음. 가짜 gist·메모리 Keychain만 씁니다(실제 GitHub·Keychain은 허락 항목).

---

### settings-67

**(a)** settings-67 · 편집한 키가 실제 동작에 반영 · partial(commands-keymap, 기능표 616줄)

**(b) 원본**: `TS/shared/lib/keymap/keymap.ts` 372 `parseKeymapOverrides`, 404 `applyKeymapOverrides`(override가 있으면 그 항목의 key·mods·chord를 통째로 바꾸고 when을 다시 계산, 336줄 주석), `TS/shared/lib/keymap/command-binding-dispatch.ts`(명령 키 dispatch). 원본은 카탈로그의 모든 명령이 실행 경로를 갖고, override 키도 같은 경로로 실행됩니다. 경고 문구 `settings.keymapKeyNotBindable`은 settings-83(complete) 몫입니다.

**(c) native**
- 완료된 증명: [C] `UI/keymap-settings-tests.rs` `모든_catalog_행의_override는_새_키로_같은_id를_내고_기본_키는_그_id를_더이상_내지_않는다`(88~155: mac·그 밖 모든 행을 빈 키 `mod+alt+shift+F8`로 override, 라우팅 안 되는 행은 keymap 행이 아니고 `monaco.`이거나 `runs_via_command`가 아님을 단언하고 목록을 `settings-67 platform=… routed=… unrouted=…`로 출력). [C] `APP/application-settings-consumers-tests.rs` `override로_라우팅되는_catalog_명령의_앱_실행_경로_유무를_문맥에서_판정한다`(434~467: `command_dispatch::accepts`(`APP/command-dispatch.rs` 62)를 활성 프로젝트·포커스 슬롯·쓰기 가능 편집기·메인 창 문맥으로 불러 `settings-67 accepted=… rejected=…` 출력).
- lane 측정(`QA-S5`, shell·explorer-search 병합 전) 218행 중 수락 173, 거절 45: `window.reload`, `search`, `search-replace`, `git`, `view.welcome`, `terminal.copyImeDebug`, `app.showPerfSnapshot`, `tab.moveToNewWindow`, `tab.moveToMainWindow`, `sync.uploadNow`, `sync.downloadNow`, `cli.connectExternalEditor`, `cli.installShellCommand`, `cli.uninstallShellCommand`, `git.revertHead`, `git.createTagOnHead`, `git.toggleBlame`, `git.openFileHistory`, `terminal-jump-to-previous-command`, `terminal-jump-to-next-command`, monaco 25개.
- 메인 코드 읽기로 본 변화(측정 아님, 반드시 재측정): `UI/command-registry.rs`의 `execution`(360~)과 `keymap_run`(601~)에 `search`·`search-replace`(658~659 `Run::OpenSearch`), `view.welcome`(480 `Run::OpenWelcomeTab`), `tab.moveToNewWindow`(482 `Run::MoveFocusedTab`), `cli.*`(463~471 `Run::Cli`)가 생겼습니다. `tab.moveToMainWindow`는 `Enablement::AuxiliaryWindow`(347)라 메인 문맥에서 거절이 맞고, `terminal.copyImeDebug`·`app.showPerfSnapshot`은 `Enablement::WebviewDiagnostics`(346, 항상 거절 318)입니다. `GITLANE`에는 `git`(677 `Run::ShowGit`), `git.revertHead`·`git.createTagOnHead`·`git.openFileHistory`·`git.toggleBlame`(474~479)과 `monaco.taide.openFileHistory`·`monaco.taide.toggleBlame`(476·479)의 실행 경로가 있어 병합 뒤 거절·미라우팅 수가 줄어듭니다.
- `accepts` 거절이 곧 "실행 경로 없음"은 아닙니다: `terminal-jump-to-*`는 터미널 표면이 직접 처리합니다(`APP/terminal_surface.rs`, terminal-agent-28 complete).

**(d) 구현 방법**: settings에서 새로 만들 소비자는 없습니다(`SPEC` 2K settings-67 설계, 결정 8 A).
1. git lane 병합 뒤 메인에서 두 검사를 `--nocapture`로 다시 돌려 현재 거절 목록과 라우팅 안 되는 목록을 얻습니다.
2. 목록의 각 id를 아래로 분류해 6열과 QA 표에 적습니다: (가) 다른 경로로 실행됨(예: `terminal-jump-to-*` → terminal-agent-28), (나) 결정·설계로 비활성이 맞음(`app.showPerfSnapshot` → commands-ui-21, `docs/research/2026-10-28-lane-commands-ui-spec.md` 4.1절 결정 3 A(647줄) not-applicable; `terminal.copyImeDebug` → commands-ui-20, 같은 사양 213줄 "비활성 동등 검사". 두 행의 현재 판정은 `MATRIX`와 `HO-16`에서 확인), (다) 문맥상 거절이 맞음(`tab.moveToMainWindow`는 보조 창 문맥 전용), (라) 소유 행이 남은 것(`window.reload` → commands-ui-1, `sync.uploadNow`·`sync.downloadNow` → commands-ui-30·31, 남은 `monaco.*` → editor 영역). 분류할 때 그 명령의 기능표 행을 grep으로 찾아 행 ID를 적습니다.
3. (라)가 모두 닫히면(다른 영역 작업) 검사를 다시 돌려 거절이 (가)·(나)·(다)뿐임을 확인하고 complete로 올립니다. settings 쪽 재사용 명령(`domains::settings::Command::Sections`와 `sections::Action::{CliInstall, CliUninstall, SyncUpload, SyncDownload}`)은 이미 crate 안에서 공개입니다(`QA-S5` 4절 7). commands-ui-30·31이 이를 재사용합니다(`HO-16` 79~80줄).

**(e) 자동 검사**
- `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --lib --features native-host,inspection --locked --offline --target-dir <T> -- 모든_catalog_행의_override는 --nocapture`
- `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- override로_라우팅되는 --nocapture --test-threads=2`
- 출력 줄 `settings-67 platform=… routed=… unrouted=…`, `settings-67 accepted=… rejected=…`를 QA에 그대로 옮깁니다.

**(f) 완료 판정**: 거절 목록의 모든 id가 (가)·(나)·(다)이고 두 검사가 통과.

**(g) 막는 요소**: commands-ui(wave 3, 미착수), git lane 병합, editor 영역의 monaco 동작. settings 단독으로는 닫을 수 없습니다. 재측정·분류(1·2단계)는 병합 뒤 언제든 할 수 있고, 행은 partial로 두되 6열의 목록을 최신으로 바꿉니다.

---

### settings-78

**(a)** settings-78 · 오류 표시 경로(TS는 `toast.error`) · partial(settings-base, 기능표 627줄)

**(b) 원본**: 대부분 `toast.error(describeIpcError(error))`. 설정 저장 실패는 `toast.error(i18next.t('settings.saveFailed'), { description: describeIpcError(error) })`(`TS/entities/settings/settings.query.ts` 26), 테마·로케일 실패는 `StatusErrorBanner` + 재시도 `common.retry`(`TS/app/providers/theme-provider.tsx` 103, `locale-provider.tsx` 28), 트리 실패는 인라인 `rootUnavailable`(`TS/widgets/explorer/explorer-container.tsx` 88·295). 상태 표시줄 문자열 라벨은 원본에 없습니다.

**(c) native**
- 결정 3 A(채택, `SPEC` 4.1-3): TS 대응이 있는 지점은 원본 표면(toast·배너·인라인), 대응이 없는 native 내부 오류는 `log::warn!`만 남기고 상태바 라벨과 `status` 필드를 지웁니다.
- 라벨 구조: 필드 `status: Option<String>`(`APP/application.rs` 250), 프레임마다 `let mut status = self.status.take()`(1721)와 복원 `self.status = status`(2046), `AppSurfaces`의 `status: &mut status`(1967)·`status: &mut Option<String>`(2487), 그리기 `ui.label(status)`(2806, `status_bar` 2773 안).
- settings 파일은 0곳입니다(`QA-S5` 3.2: 앱 파일 정합 실패·테마 미리보기 실패 2곳을 `log::warn!` + `unreported_failure`(`domains/settings.rs` 113)로 옮김).
- 남은 대입(메인 `0c52b438`, `grep -c "status = Some("`, `cli_status` 1곳과 지역 변수 포함 전 수): `application.rs` 21, `domains/editor.rs` 51, `domains/explorer_search.rs` 8, `domains/commands.rs` 5, `domains/terminal_agent.rs` 5(490은 `cli_status`라 제외 → 4), `domains/shell.rs` 4, `domains/preview.rs` 2, `domains/git.rs` 1. `GITLANE` 병합이 `domains/git.rs`를 8곳으로, `git-editor.rs`에 1곳을 더합니다.
- 함수별 위치(분류 작업의 단위, awk로 둘러싼 함수를 확인함):
    - `application.rs`: `background_tick` 1090·1165·1175·1213·1242·1250·1277·1284, `show_window` 1802·1820·2003·2044·2104(1802·1820·2003·2044는 지역 `status`), `finish_frame` 2305, `paint_rail_icon` 2494(`*status`), `rail_icon` 2690, `tooltip` 2724, `prepare_status_bar` 2743·2747·2752, `apply_status_bar` 2767.
    - `domains/editor.rs`: `apply_editor_reply` 763·854·964·1007·1147, `apply_editor_intent` 1489, `request_editor_code_actions_from_source` 1979, `request_editor_rename_from_source` 2043, `request_editor_formatting` 2236, `reconcile_lsp` 2415·2426·2450·2461·2473·2500·2506, `dispatch_completion` 2716·2722, `consume_location_model` 2874·2904·2917, `poll_lsp` 3071·3150·3181·3210·3235·3334·3337·3455·3501·3538·3543, `save_untitled` 4536·4591·4597, `save_missing_draft` 4694·4700, `choose_disk` 4877, `reconcile_editor_features` 5288·5297, `problems_panel` 5327, `tab_icon` 5479, `show_document` 5624·5651·5734·5833·5851·5944(`*status`)·6010·6079·6088.
    - `domains/commands.rs`: `show_commands_overlays` 230, `show_palette` 335·351·388·450. `domains/explorer_search.rs`: `apply_explorer_search_reply` 730·747·768, `finish_explorer` 2033·2036·2055, `show_search_editor_tab` 2343·2348. `domains/terminal_agent.rs`: `tick_terminal_agent_fonts` 524, `tick_terminal_agent_cursors` 549, `finish_terminal_agent_frame` 627, `show_terminal_tab` 691. `domains/shell.rs`: `apply_shell_intent` 787·816, `show_auxiliary_toasts` 1409, `show_auxiliary_windows` 1525. `domains/preview.rs`: `reconnect_preview` 405, `show_preview_web_views` 674. `domains/git.rs`: `title_branch_icon` 131(병합 뒤 다시 셈).
- 검사가 `status`에 기대는 곳: `grep -c "application\.status\|\.status\.is_\|status = None" APP/*-tests.rs` 기준 14개 파일 33줄. 대부분 실패 메시지 출력용이지만 단언도 있습니다: `APP/application-startup-tests.rs` 744~760(`"rename-error-presented"`: 이름 바꾸기 오류가 `status`와 error toast 둘 다에 나타남, 제품 쪽은 `application.rs` 1165~1166이 `status` 대입 바로 다음 줄에서 `toasts.ipc_error`), `APP/application-explorer-search-tests.rs` 528~550(조용한 reveal 실패는 `status` 없음, `"loud reveal failure"`는 `status` 있음).
- 관련 행: commands-ui-96(비국제화 문자열)은 이 행이 0건을 만든 뒤 complete가 됩니다(`HO-16` commands-ui-96 (d)·3).

**(d) 구현 방법** (메인 통합 단계, 모든 lane 병합 뒤 한 번에)
1. 모든 lane(특히 git) 병합 뒤 위 grep으로 목록을 다시 만듭니다. `cli_status`처럼 다른 필드는 빼고 `self.status`·`*self.status`·`AppSurfaces`의 `*status`·`show_window` 지역 `status`만 셉니다.
2. 지점마다 분류해 QA 표에 적습니다(형식은 `docs/quality-assurance/2026-10-06-native-batch3-toast-feedback.md` 3절과 같게: 파일:함수, 상황, TS 대응 근거, 처리).
    - 같은 자리에서 이미 toast를 띄우는 곳: `status` 줄만 지웁니다. 예: `application.rs` 1165·1175(`self.toasts.ipc_error(..)`가 바로 다음 줄).
    - TS가 toast인 곳: `self.toasts.ipc_error(&self.locale, &error, now)`, 반복 도달 가능하면 기존 `report_once`(`application.rs` 856, 같은 QA 4절 137줄: 같은 문구의 오류 toast가 살아 있는 동안 새로 만들지 않음).
    - TS가 배너·인라인인 곳: 기존 표면(`APP/status-error-banner.rs`, 탐색기 인라인 상태)으로 보냅니다.
    - TS 대응 없음(렌더 내부 오류, 종료 절차, 터미널 입력 flush 등): `log::warn!`. 매 프레임 다시 도달하는 곳은 `domains/settings.rs`의 `ReportedFailures` + `unreported_failure`처럼 같은 문구를 한 번만 기록합니다.
3. 대입이 0이 되면 `status` 필드, `show_window`의 take/복원, `AppSurfaces`의 `status` 인자, `status_bar`의 `ui.label(status)`를 지웁니다.
4. 검사 갱신: 출력용 `application.status`는 지우고, 단언은 원본 표면으로 바꿉니다(이름 바꾸기 오류 → error toast, 시끄러운 reveal 실패 → 그 지점의 TS 표면을 확인해 toast 또는 인라인).
5. 다른 영역 파일을 고치므로 각 영역 행(예: editor·explorer-search)의 비고와 QA에도 바뀐 표면을 적고, 그 영역 검사를 함께 돌립니다.

**(e) 자동 검사**
- grep 확인: `grep -rn "self.status\b\|status: &mut Option<String>" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src --include="*.rs"` 결과 중 검사 파일이 아닌 것 0(제외 기준을 QA에 적음).
- 대표 경로 3개 App 검사: 호스트 실패(`HostReply::Failed` → toast), 설정 저장 실패(`settings.saveFailed` toast, 선례 `APP/application-search-panel-tests.rs` `실제_app_검색_이력_저장이_실패하면_설정_저장_실패_toast를_띄우고_대기_목록을_비운다` 1243), 터미널 오류(분류 결과에 따라 `log::warn!` 또는 toast).
- 앱 lib 전체를 병합 게이트와 같은 명령(`--no-fail-fast -- --test-threads=4`)으로 한 번 돌려 `status`를 기대하던 검사가 모두 새 표면으로 통과하는지 확인합니다.

**(f) 완료 판정**: `status` 필드·라벨 제거, 분류 표 QA 기록, 위 검사 통과 → complete.

**(g) 막는 요소**: 모든 lane 병합(특히 `GITLANE`). 여러 영역 파일을 동시에 고치므로 lane 병렬 중에는 하지 않습니다. 이 행을 위해 다른 영역 동작을 바꾸지 말고 표면만 원본에 맞춥니다.

---

### settings-80

**(a)** settings-80 · Interface 설정값의 소비처(탐색기 자동 reveal, 환영 화면, agent 상태 배지, IDE diff 자동 열기, searchOnType, debounce, minimap) · partial(settings-code, 기능표 629줄)

**(b) 원본 소비처**
- 자동 reveal: `TS/widgets/explorer/use-explorer-auto-reveal.ts` 64(`settings?.explorerAutoReveal ?? true`).
- 환영 화면: `TS/widgets/editor-area/pane-node-view.tsx` 130(메인 창이고 `welcomeOnEmptyEditor ?? true`일 때만).
- 상태 배지: `TS/widgets/app-sidebar/app-sidebar.tsx` 90(`agentStatusBadgeEnabled ?? true`).
- IDE diff 자동 열기: `TS/app/providers/ide-sync-provider.tsx` 54(`!current.ideAutoOpenDiff`면 탭 없이 즉시 rejected).
- searchOnType·debounce: `TS/widgets/search-panel/search-panel-container.tsx` 74~75(기본 `DEFAULT_SEARCH_ON_TYPE`, `DEFAULT_SEARCH_ON_TYPE_DEBOUNCE_MS` = 300, `CR/taide-model/src/settings.rs` 73), 192~203(trailing debounce). 꺼짐이면 안내 문구가 `search.pressEnterHint`, 켜짐이면 `search.liveSearchHint`.
- minimap: `TS/shared/lib/code-editor-settings.ts` 46(`minimap: settings?.editorMinimap ?? true`), `TS/features/editor/code-editor.tsx` 275~276(`minimap: { enabled: minimap && !largeFile }`).
- 키: `settings.explorerAutoReveal`, `settings.welcomeOnEmptyEditor`, `settings.agentStatusBadge`, `settings.ideAutoOpenDiff`, `settings.searchOnType`, `settings.searchOnTypeDebounceMs`, `settings.editorMinimap`.

**(c) native**
- 근거가 있는 소비자:
    - 환영 화면: `APP/application-settings-consumers-tests.rs` `실제_빈_편집기_환영화면은_설정을_끄면_사라지고_빈_편집기_문구를_보인다`(277, Settings 경로).
    - 배지: `APP/application-project-rail-tests.rs` `실제_app_rail은_표시_glyph와_폴링된_에이전트_상태와_배지_설정을_반영한다`(263, 메모리 설정을 직접 바꿈).
    - reveal: explorer-search-25 complete, `APP/application-explorer-search-tests.rs` `실제_app_활성_파일_자동_reveal은_설정과_사이드바_보임을_따르고_조용히_실패한다`(415, 설정 끔 포함, 메모리 설정을 직접 바꿈).
    - searchOnType·debounce 켜짐: explorer-search-34 complete, `APP/application-search-panel-tests.rs` `실제_app_입력_검색은_디바운스_뒤에_한_번_돌고_이력을_남기지_않는다`(380). 꺼짐은 단위 검사만 있음: `APP/search-panel-tests.rs` `설정이_꺼져_있으면_입력만으로는_돌지_않고_enter_에서만_돈다`(411).
- 빠진 것:
    1. minimap: 소비 경로(`UI/presentation.rs` 291 `minimap: settings.editor_minimap` → `domains/editor.rs` 5562 `editor_presentation`, 그리기 `UI/editor-minimap.rs` `measure` 42~75: 꺼짐·색 없음·`FileSizeTier::Normal`이 아니면 그리지 않음 — 원본의 `!largeFile`과 같음)는 있으나 App 검사가 없습니다.
    2. searchOnType 끔의 App 검사가 없습니다. `SPEC` 2J settings-80 검사 목록이 "searchOnType off → 입력만으로 검색 요청 없음"을 [A]로 요구하므로, 과대 판정을 피하려면 이 검사를 완료 조건에 넣습니다.
    3. IDE diff 자동 열기: 메인에는 소비자가 없습니다(`APP/ide-tools.rs` 209가 `AppEvent::IdeDiffRequested`를 발행하지만 받는 곳이 없어 시간 초과로 rejected). `GITLANE`에는 git-49가 complete이고 검사 `application-git-claude-history-tests.rs` `실제_app_claude_diff는_자동_열기_끄기_reject_탭_닫기_ide_close_tab을_원본대로_처리한다`(372)가 끔 → 즉시 Rejected·탭 없음을 확인합니다(설정 읽기는 `GITLANE` `domains/git.rs` 2273). 병합 작업 트리에서 수신 경로가 shell 수신함 `Event::IdeDiff`로 옮겨지는 중이라 병합 뒤 같은 검사가 통과하는지 다시 봐야 합니다.

**(d) 구현 방법**
1. minimap 검사(`APP/application-settings-consumers-tests.rs`). 텍스처 판별은 기존 `crate::application::error_boundary_tests::texture_ids`(`APP/application-error-boundary-tests.rs` 211, 현재 비공개)를 `pub(super)`로 바꿔 재사용합니다(두 번째 사용처이므로 복사하지 않음, shell 영역 검사 파일이라 QA에 한 줄). 판별:

    ```rust
    const MINIMAP_TEXTURE_PREFIX: &str = "native-minimap-";

    fn has_minimap(context: &egui::Context, output: &egui::FullOutput) -> bool {
        let textures = crate::application::error_boundary_tests::texture_ids(context, MINIMAP_TEXTURE_PREFIX);
        shapes(output).into_iter().any(|shape| {
            matches!(shape, egui::Shape::Mesh(mesh) if textures.contains(&mesh.texture_id))
        })
    }
    ```

    미니맵은 이름이 `native-minimap-{frame id}-{index}`인 텍스처(`UI/editor-minimap.rs` 356~360 `load_texture`)를 `painter.image`(377)로 그립니다. UI 검사도 같은 접두사로 텍스처를 찾습니다(`UI/../tests/editor-minimap.rs` 214~234). `shapes`는 settings-28 (d) 2의 도우미입니다. 이 검사는 `error_boundary_tests`가 unix 전용이므로 `#[cfg(unix)]`를 붙입니다.
2. searchOnType 끔 검사(새 검사, 같은 파일 또는 `APP/application-search-panel-tests.rs`). `crate::application::search_panel_tests::{Launched, launch_project, texts, message}`와 `project_rail_tests::{step, node_rect}`, `auxiliary_window_tests::step_until`을 씁니다. 같은 파일의 비공개 `open_search_view`(138)·`panel_generation`(154)을 `pub(super)`로 바꿔 재사용합니다(explorer-search 소유 검사 파일이라 그 영역 QA에 한 줄).
3. git lane 병합 뒤: 메인에서 위 git-49 검사를 실행하고 6열에 인용합니다.

**(e) 자동 검사**
- minimap(이름 예: `실제_편집기_미니맵은_settings_경로로_끄면_그리지_않고_켜면_다시_그린다`): `Scene::launch_empty_pane(|_| {})`, `missing_cli_agents()` 포트, `resize(TOC_SCREEN)`, 상수 `const MINIMAP_LINES: usize = 40;`의 줄로 된 파일을 settings-28의 `open_file`로 열기 → `let context = scene.context().clone();` → `scene.until("minimap", |_, output| has_minimap(&context, output))` → `update(&mut scene, Change::Switch(Switch::EditorMinimap, false))` → `!has_minimap`와 `settings.read().editor_minimap == false`와 `settings.json`의 `editorMinimap == false` → 다시 `true` → `has_minimap`.
- searchOnType 끔(이름 예: `실제_app_입력_검색은_설정을_끄면_디바운스가_지나도_돌지_않고_enter에서만_돈다`): 상수 `const LIVE_OFF_WAIT: Duration = Duration::from_millis(600);`(기본 debounce 300ms의 두 배). `launch_project("search-live-off", &[("a.txt", "needle\n")], false)` → `application.submit(HostCommand::Settings(domains::settings::Command::UpdateSettings(Change::Switch(Switch::SearchOnType, false))))` → `settings.read().search_on_type == false` 대기 → 트리 대기(`node_rect(output, "a.txt", egui::Rect::EVERYTHING)`) → `open_search_view` → `message(application, "search.pressEnterHint", &[])` 문구가 `texts(output)`에 보일 때까지 대기(소비자가 꺼짐을 읽었다는 증거) → `step(.., vec![egui::Event::Text("needle".into())])` → `LIVE_OFF_WAIT`가 지날 때까지 `step`과 짧은 `std::thread::sleep`을 반복 → `panel_generation(application, &slot) == Some(0)` → Enter(`crate::application::inlay_hints_tests::key(egui::Key::Enter, egui::Modifiers::NONE)`) → 결과 요약(`search.matchCount`) 또는 generation 1을 대기.
- git-49(병합 뒤): 앱 lib 명령 + 필터 `실제_app_claude_diff는_자동_열기_끄기 --test-threads=2`.
- 실행: 앱 lib 명령 + 필터 `settings_consumers_tests search_panel_tests --test-threads=2`.

**(f) 완료 판정**: minimap App 검사 + searchOnType 끔 App 검사 통과 + git lane 병합본에서 git-49 검사 통과 + 6열에 7개 소비자의 검사 이름이 모두 있음 → complete.

**(g) 막는 요소**: git lane 병합(IDE diff, 0.5절 넷째 확인). minimap·searchOnType 끔 검사는 지금 할 수 있습니다.

## 4. 이 영역에서 반복된 함정과 주의

1. 과대 판정: 리뷰가 4행을 되돌렸습니다. settings-5(복원 layout을 쓰고 새 mount를 확인하지 않음), settings-24(재적재 요청 미단언), settings-38(소비 경로를 인용만 함), settings-42(TS 사례 일부만 이식). complete는 사양의 [A] 단계를 **전부** 실제 App에서 단언했을 때만 씁니다(`QA-S3` 7.1·7.2, `QA-S5` 7.1, `QA-S2` 7.1). 그래서 settings-80은 searchOnType 끔 App 검사까지 요구합니다.
2. 선행 lane이 없을 때 대체 구현 금지: 알림 포트·scrollback API를 settings가 따로 만들지 않았습니다(`SPEC` 3절 주의 5). 지금은 선행이 병합됐으니 그 산출물을 그대로 씁니다. 병합 중인 `GITLANE`과 같은 줄을 메인에서 먼저 고치지 마십시오(병합 충돌).
3. 실제 사용자 환경 누수: (가) 이 기계의 실제 `/usr/local/bin/taide`가 CLI 행 문구를 바꿔 다른 검사의 문구 수를 흔들었음 → Settings를 그리는 검사는 `missing_cli_agents()` 포트(`QA-S1` 4절 11). (나) hooks 검사에서 프로젝트를 붙이면 `agent_hooks::reconcile_installed`가 실제 `HOME`을 고칠 수 있음 → `Scene::launch_with(.., false)`(root_missing). (다) Sync section이 기본 Keychain을 읽음 → `Scene` 기본을 메모리 Keychain으로 바꿈(`QA-S4` 4절 10). (라) 글꼴 목록은 포트 `font_families`로 고정(`QA-S5` 4절 8). (마) 기본 LSP 감지 포트는 실제 `PATH`를 읽기만 함(`QA-S5` 6절 끝).
4. egui 동작 함정:
    - `ScrollArea::scroll_to_rect`는 content의 `item_spacing`을 더해 32px 대신 약 34.7px에 멈춤 → 카드 루프 뒤 `scroll_with_delta`(`QA-S5` 3.1).
    - 위쪽에 경고 줄이 생기면 egui 자동 id가 바뀌어 입력 포커스를 잃음 → 입력에 `id_salt`(`QA-S4` 4절 7).
    - 이 저장소의 egui 패치는 `lost_focus`를 2프레임 보고함 → blur 패치 중복(`QA-S2` 4절 4).
    - 대화상자가 닫힌 같은 프레임의 포커스 요청은 modal 규칙으로 버려짐 → 다음 프레임에 복귀(`QA-S3` 4절 2).
    - `logic`은 직전 프레임 입력을 읽음 → 포커스 상태 검사는 한 프레임 먼저 바꾸고 2프레임 진행(0.4절, settings-22).
    - 그려지지 않은 owner View는 `finish_frame`에서 사라짐 → Settings 카탈로그·TOC 상태는 Settings 탭이 활성인 동안에만 `inspection()`으로 읽힘(settings-48).
5. 검사 쪽 원인 실패: 기본 layout에 Welcome 탭이 있음(`launch_empty_pane`이 비움), 글꼴 항목은 자기 글꼴로 그려져 글자 도형 검색이 0건(접근성 노드로 찾음), section 데이터가 도착하며 카드 높이가 바뀜(조회 완료 뒤 클릭), oMLX는 base URL이 있어야 configured(토글 비활성 원인), 요청 경로가 `/private/var/...` 정규 경로(canonicalize 비교)(`QA-S5` 6절, `QA-S2` 7.1).
6. 공유 `Data`: 모든 owner가 한 `Data`를 쓰므로 owner별 결과는 ticket으로 돌려줍니다. 새 행동을 추가할 때 이 규칙을 지키지 않으면 메인 창과 보조 창이 서로의 결과를 받습니다.
7. 응답 수명: `Reply::Sections`는 `closing`이면 버림(`QA-S5` 7.3). 제출 실패는 `submit`이 이미 알리므로 `Data::accept`에만 넣고 toast를 또 내지 않음(재현 검사 `실제_sections_요청_제출이_실패하면_오류_알림은_한_번만_뜬다`, `APP/application-settings-plugins-tests.rs` 289).
8. 매 프레임 실패 로그 폭주: `log::warn!`으로 옮길 때 같은 문구는 한 번만(`ReportedFailures` + `unreported_failure`, 단위 검사 `매_프레임_실패는_같은_문구를_한_번만_기록하고_회복_뒤_다시_기록한다`). settings-78 일괄 처리 때 그대로 씁니다.
9. 숫자 표기: JS `toFixed(1)`은 동률을 큰 쪽으로 반올림 → Rust `{:.1}`(짝수 쪽)과 다름(`to_fixed_one`, `QA-S5` 7.3). Performance JSON은 `1.0`을 `1`로(`QA-S1` 4절 8).
10. 동결 미러: `Views::show` 시그니처는 바꾸지 않습니다(`native/taide-remote-web`이 호출). UI crate나 `taide-model`을 바꾸면 Wasm check를 반드시 돌립니다. `taide-model`의 `cargo fmt --check`는 기존 `lib.rs`(`ids`/`identifier` 순서) 차이가 있으니 범위 밖으로 둡니다(`QA-S3`·`QA-S4` 5절).
11. 시험 알림은 첫 전달 안내 경로를 타면 안 됩니다. `ShellCommand::Notify`/`notify_event`로 바꾸지 마십시오(원본이 `sendNativeNotification`을 직접 부름, `SPEC` 2B settings-22 설계).
12. 설정 변경 경로: 새 소비자 App 검사는 메모리 설정을 직접 쓰지 말고 `HostCommand::Settings(Command::UpdateSettings(change))`로 바꿔 저장·사건·재조정까지 지나게 합니다(settings-30이 남았던 이유). 기존 다른 영역 검사(배지·reveal)의 직접 쓰기는 고치지 않습니다.
13. 테스트 모듈 cfg: `project_rail_tests`·`error_boundary_tests`·`notifications_tests`는 `#[cfg(all(test, unix))]`입니다. 이것을 직접 가져오는 새 검사에는 `#[cfg(unix)]`를 붙입니다(2.4절 가시성 표).
14. 기능표 상단 요약표가 행과 어긋나 있습니다(1.1절). 행을 바꿀 때마다 요약표를 다시 셉니다.
15. 병합 게이트의 흔들림: 앱 lib 병렬 실행에서 2건 정도가 흔들리고 직렬 재실행에서 통과합니다. 실패하면 단독으로 한 번 다시 돌려 흔들림인지 확인하고, 흔들림이면 QA에 이름과 두 실행 결과를 적습니다(검사 기대값을 바꾸지 않음).
16. `GITLANE` worktree는 다른 작업자가 병합 충돌을 해결하는 중입니다. 그 worktree의 파일을 고치지 말고 읽기만 합니다. 그 작업 트리의 미커밋 내용(이 문서 1.3절의 "`GITLANE`" 열)은 병합 결과가 아니라 진행 중 상태이므로, 병합 뒤 0.5절 명령으로 메인에서 다시 확인합니다.
17. 비밀 입력 메모리 위생: 비밀번호 입력의 egui undoer가 비밀 텍스트를 메모리에 남기는 문제는 손대지 않은 채 남아 있습니다(`QA-S5` 7.3 표 마지막 줄). 기능 행은 아니지만 보안 항목으로 별도 보고 대상입니다.
