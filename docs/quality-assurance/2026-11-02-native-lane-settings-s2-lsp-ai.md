# native lane settings 묶음 S2: LSP·AI section (s2-lsp-ai)

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/settings`(브랜치 `lane/settings`) · 사양 `docs/research/2026-10-28-lane-settings-spec.md` 2.1~2.8, 2C, 2D, 3절 S2
> 결정: 사양 4.1 전부 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`). 이 묶음은 lane의 첫 묶음이라 S1 공통 골격 가운데 S2에 필요한 부분을 먼저 만들었습니다.

## 1. 대상 ID와 판정

| ID | 내용 | 판정 |
| --- | --- | --- |
| settings-32 | LSP section 서버 목록·설치 여부·경로·버전·experimental 배지·힌트 3종 | complete |
| settings-33 | LSP 설치·취소·진행률(바이트)·실패 문구·설치 이벤트 동기화 | complete(실제 다운로드는 4.2 허락 항목) |
| settings-34 | LSP 설치 명령 복사와 툴체인/SDK/체크섬 힌트 | complete(실제 클립보드는 4.2 허락 항목) |
| settings-35 | AI provider 토큰 행(ollamaCloud, codex 경고) 저장·해제·상태 | complete(Keychain·Codex 검증은 4.2 허락 항목) |
| settings-36 | AI oMLX base URL·API key | complete |
| settings-37 | AI provider 선택, 모델 목록 조회 중/오류/빈 상태, 모델 선택 | complete |
| settings-38 | AI auto-tab 토글(선택 provider 미설정이면 비활성) | complete(7절: 토글 → 편집기 요청 연결 App 검사 추가) |

## 2. 원본 동작 근거

- 사양 2C(32~34), 2D(35~38)의 원본 줄 인용을 그대로 따릅니다. 이번에 다시 확인한 원본: `src/features/settings/lsp-server-status-list.tsx`(행·버튼·진행·힌트·복사), `src/widgets/settings-view/settings-lsp-section.tsx`(조회 중 문구, 취소 오류 키 무시), `src/entities/lsp/lsp.query.ts`(staleTime Infinity, 설치 성공 재감지, 실패 시 끝나지 않은 진행 제거), `src/widgets/settings-view/settings-ai-section.tsx`, `src/features/settings/{ai-provider-token-row,ai-omlx-row,ai-auto-tab-toggle,text-field,option-picker,settings-section}.tsx`, `src/entities/ai/ai.query.ts`(토큰 저장·해제 성공 시 상태와 그 provider 모델 무효화).
- section 순서와 제목 키: `src/widgets/settings-view/settings-view.tsx:61-92`(Keymap 다음 LSP, AI).

## 3. 구현

### 3.1 공통 골격(S1에서 S2가 쓰는 부분)

- `Section::{Lsp, Ai}`, `Section::HOSTED`, `Section::description()`(Keymap·LSP 설명, 기존 Keymap 분기를 일반 규칙으로 교체): `native/taide-native-ui/src/settings-controls.rs`.
- `Change::Hosted(HostedChange)`(`AiProvider`, `AiModel`, `AiAutoTab`, `AiOmlxBaseUrl`)가 기존 `UpdateSettings` → `update_control_settings` → `Integrations::reconcile` 경로를 그대로 탑니다. `Switch::ALL`은 바꾸지 않았습니다.
- 데이터 캐시 `native/taide-native-ui/src/settings-sections-data.rs`(`settings_sections_data::Data`): `Query::{LspServers, AiTokens, AiModels(provider)}`의 신선도(LspServers·AiModels 무한, AiTokens 60초), 실패 값은 다음 mount에 재조회, 600초 미관찰 뒤 mount하면 버림, 세대 번호로 늦은 응답 무시. 행동 `Action`과 `Request::failed`, 응답 `Reply`, 사용자 알림 `Notice`, LSP 진행 맵, AI 토큰 저장 pending(마지막으로 보낸 provider만), `Secret`(Debug 가림).
- 화면 `native/taide-native-ui/src/settings-sections-view.rs`(모듈 경로 `settings_view::sections_view`): `Host`, owner별 `OwnerState`, LSP·AI section 렌더.
- 진입: `Views::show_with_host(..., Host)`. 기존 `Views::show`는 시그니처를 유지하고 호스트 없음으로 위임하므로 동결 미러(`taide-remote-web`)는 8 section 그대로입니다(결정 1 A).
- 앱 영역 `native/taide-native-app/src/domains/settings.rs`: `Command::Sections { request, ports }`, `Reply::Sections`, worker는 `spawn_transient`로 실행해 설치가 다른 host 명령(취소 포함)을 막지 않게 했습니다. 응답 → `Data::accept` → toast. 진행 이벤트 기록기 `Events`(서버별 마지막 값)를 `PaintSink`가 기록하고 `tick_settings_reads`가 `Data`로 옮깁니다. 요청 배출은 기존 `load_settings_requests` 안에서 합니다.
- 주입 포트 `native/taide-native-app/src/settings-ports.rs`: `detect_lsp_servers`, `lsp_install`. 클립보드는 기존 호스트 `ClipboardWriter` 주입을 씁니다.
- 아이콘: `Icon::CircleCheck`, `Icon::CircleX`(기존 `resources/problems/circle-{check,x}.svg`).
- 공용 파일(추가만): `application.rs`의 `PaintSink.settings` 필드와 `record` 1줄, 생성자의 `settings_events` 1줄과 `settings_state` 리터럴 끝 2필드. `lib.rs`(앱·UI) 모듈 목록 정렬 위치 1줄씩.

### 3.2 LSP(32~34)

- 조회 중 muted `settings.loading`, 실패면 빈 목록. 행: CircleCheck/CircleX 14px(agent-running/badge), 이름, `expert` 배지(`settings.lspExperimental`, warning 글자·warning 40% 테두리), 상태 문구(`settings.lspInstalled (v버전)`/`settings.lspNotInstalled`), 버튼(활성 진행이면 ghost 취소, download면 outline 설치(`downloadAvailable != true`면 비활성), toolchain 가능하면 `"{tool} {설치}"`).
- 경로 줄(들여씀 22px, muted monospace), 진행 줄(실패 `message ?? settings.lspInstallFailed` error, 아니면 회전 Loader + `settings.lspInstalling` + download이고 total이 있으면 `받은 / 전체`와 4px 막대), 상태 힌트 3종(warning), sdk-detect 평문 힌트, 그 밖 힌트 + 복사 버튼(아래쪽 tooltip, 접근 이름 `settings.lspCopyCommand`).
- 설치 성공 → `LspServers` 무효화(관찰 중이면 즉시 재감지). 실패 → 진행이 끝나지 않았으면 그 항목 제거, `error.lsp.installCancelled`면 조용히, 아니면 `describe_error` toast. 취소 실패 toast. 복사 성공 `settings.lspCommandCopied`, 실패 `settings.lspCommandCopyFailed`.

### 3.3 AI(35~38)

- 토큰 행 2개(ollamaCloud, codex+경고)와 oMLX 행. 저장은 trim이 비거나 그 provider 저장 중이면 비활성, 저장 시 입력을 비우고 `Secret`으로 보냄. 해제는 설정됨일 때(oMLX는 항상). 오류 문구는 `describe || settings.aiTokenSaveFailed|ClearFailed`, 성공 toast 없음, 성공 시 상태와 그 provider 모델 무효화.
- oMLX base URL: 라벨·placeholder·폭 224px의 기존 text field 도우미(`code_view::labeled_text`, 기존 `text_field`를 나눈 것)로 그리고 blur에서 trim 값을 패치합니다. base URL 변경을 감지하면(`Data::observe_settings`) `AiTokens`·`AiModels(Omlx)`를 무효화합니다(원본 미무효화 결함 보정, 사양 36).
- provider 선택(기존 `Picker`, 폭 128px) → `{aiProvider, aiModel: ""}`. 선택 provider가 설정됐을 때만 모델 조회. 오류(oMLX `settings.aiOmlxConnectFailed`, 그 외 `settings.aiModelLoadFailed`), 빈 목록 muted placeholder, 목록이면 모델 선택(값이 목록에 없으면 muted placeholder) → `{aiProvider: 선택값, aiModel: id}`.
- auto-tab: 선택 provider 미설정이면 비활성 + `settings.aiAutoTabProviderRequired`, 아니면 `settings.aiAutoTabToggleHint`. 변경 → `{aiProvider: 선택값, aiAutoTabEnabled}`.

## 4. 사양·원본과 다른 점(근거)

1. 데이터 캐시 위치: 사양 2.4는 앱이 `Data`를 소유하고 `Host`로 넘기는 설계지만, 앱의 모든 Settings owner가 이미 공유하는 UI `Views` 안에 `Data`를 두었습니다(`Views::sections_data_mut`). 공용 `AppSurfaces` 필드를 늘리지 않으려는 것이고 공유 범위는 같습니다. 그래서 `Host`는 S2에서 필드가 없는 표시용 값입니다(S1 행이 필요한 필드를 붙입니다).
2. 모듈 경로: 화면 모듈은 `settings_view`의 비공개 상수·`Appearance` 필드를 쓰려고 `settings_view::sections_view`(파일 `settings-sections-view.rs`)로 두었습니다. 데이터 모듈은 사양대로 `settings_sections_data`입니다.
3. 포트: 사양 2.7의 `path_var` 대신 `detect_lsp_servers(lsp_dir)`를 두었습니다. 기본값은 같은 `detect_servers(lsp_dir, PATH)`이고, 앱 검사가 감지 결과를 고정할 수 있습니다. `new_with_overrides`는 만들지 않고 검사가 생성 직후 `settings_state.sections_ports`를 바꿉니다(생성자가 포트를 읽지 않음).
4. base URL blur: 이 저장소의 egui 패치는 `lost_focus`를 2프레임 동안 보고합니다(`vendor/egui-input/src/memory/mod.rs` `lost_focus`). 기존 `text_field`(rulers·shell)는 그래서 blur 한 번에 같은 패치를 두 번 보냅니다(기존 동작, 이번 범위 밖이라 그대로 둠). AI base URL은 원본처럼 blur당 한 번만 보내도록 owner 상태로 중복을 막았습니다.
5. auto-tab 비활성: 원본은 스위치만 흐려지지만 native는 기존 `switch_control`을 `add_enabled_ui(false)`로 감싸 라벨·힌트도 함께 흐려집니다. 동작(클릭 무시, 힌트 문구)은 같습니다.
6. 스피너는 원본 `animate-spin`처럼 감소 모션 설정과 무관하게 돕니다(원본 CSS에 `prefers-reduced-motion` 규칙 없음, 팔레트 스피너 선례와 같음).
7. Picker의 placeholder 색: 원본 `OptionPicker`는 값이 목록에 없으면 placeholder를 muted로 그립니다. 기존 `Picker`에 `is_placeholder` 표시를 추가해 모델 선택에서만 켭니다(기존 호출부 동작 불변).

## 5. 검증

실행 명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`입니다.

| 범위 | 명령 | 결과 |
| --- | --- | --- |
| [C] 데이터 캐시 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection -- settings_sections` | 6 passed |
| [U] 하네스 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection -- sections_view` | 11 passed |
| [A] 실제 App | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_sections_tests --test-threads=2` | 4 passed |
| UI lib 전체(아이콘 색인 이동·code view 도우미 분리 회귀) | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection` | 184 passed |
| UI 통합 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test settings-code-controls --test snippet-editor` | 1 + 18 passed |
| 앱 settings 영역 회귀(포맷 뒤) | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_ theme_ snippet_host_tests app_file_views remote_preferences presentation --test-threads=2` | 39 passed(위 [A] 4개 포함) |
| 앱 통합 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test settings-controls` | 2 passed |
| 컴파일 | `cargo check --all-targets`(UI는 `--features native-host,inspection`, 앱은 기본) | 둘 다 통과, 경고는 기존 것뿐(`tests/workbench.rs` unused mut, vendor wry, `is_applying`, `zen.rs`, `fixture_with_emmet_peek`) |
| 동결 미러 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과(경고는 기존 `browser-editor.rs`·`browser-application.rs` 미사용) |
| 형식 | `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`(UI·앱) | 차이 없음 |
| lockfile | `git status --short` | `Cargo.toml`·`Cargo.lock` 변경 없음 |

- [C] `settings-sections-data-tests.rs`: mount·신선도·무효화 재조회, 늦은 세대 무시와 실패 재조회·GC, 설치 취소 키 무음과 끝나지 않은 진행만 제거, 복사·취소 알림, 토큰 저장 pending은 마지막 요청만·성공 무효화·`Secret` 가림, base URL 변경 무효화, 제출 실패 응답 식별자.
- [U] `settings-sections-view-tests.rs`: 바이트 형식 경계(1023/1024/1048575/1048576), 미러는 8 section·요청 없음과 호스트는 LSP·AI를 Keymap 뒤에, LSP 행 구성(버전 문구·경로·배지·힌트 3종 색·sdk 평문·`go 설치`·설치 버튼 활성 3종·복사 1개), 진행 렌더(막대 폭 비율 0.25, 검증 단계 바이트 없음, 실패 메시지와 대체 문구, 완료 뒤 진행 없음)와 취소·재설치 요청, 복사 버튼 노출 조건·아래쪽 tooltip·복사 요청, 토큰 행 상태 색·경고·저장 비활성/저장 중 비활성·입력 비움·해제, oMLX URL blur trim 패치(1회)와 키 저장, 모델 영역 상태(미설정·조회 중 없음, 오류 2종, 빈 목록 muted), 키보드로 모델·provider 선택 패치와 muted placeholder, auto-tab 비활성·힌트·패치, owner section 상태 초기화.
- [A] `application-settings-sections-tests.rs`(실제 `NativeApplication`, `eframe::App::logic`/`ui`, 경로 main.rs → `NativeApplication` → `show_settings_tab` → `Views::show_with_host` → `domains::settings::dispatch`):
    1. 임시 데이터 디렉터리와 빈 PATH로 실제 `detect_servers` → 행 수가 manifest 서버 수와 같고 모든 서버 이름이 1번씩, download 서버 수 이상이 미설치.
    2. 가짜 설치 포트가 진행 이벤트를 발행 → `PaintSink` → `Events` → `Data` → "512B / 2.0KB"와 취소 버튼 → 성공 → 재감지 1회(감지 호출 2회)·설치됨; 취소 키 실패 → toast 없음·진행 제거; 기타 실패 → 오류 toast·진행 제거; sdk-detect 서버(`vtsls`)에 실제 `lsp_install` → `error.lsp.sdkDetectOnly` toast(네트워크 없음); 가짜 클립보드 bridge로 복사 → 기록된 텍스트와 성공 toast, 거절 → 실패 toast.
    3. 가짜 `SecretStore`로 ollamaCloud 저장(입력 trim) → Saved, 해제 → NotSet, store 거절 → 원본 문구 toast·저장 안 됨.
    4. loopback oMLX 모의 서버: base URL 입력·blur → `settings.json`에 trim 값 → configured → 모델 조회 1회 → 키보드로 `beta` 선택 → `settings.json` provider `omlx`·model `beta`, auto-tab 클릭 → `aiAutoTabEnabled: true`, provider를 ollamaCloud로 바꾸면 model이 비워짐(`None`), 모델 조회는 계속 1회.

## 6. 남은 범위

- 실제 LSP 다운로드, 실제 클립보드, Keychain, Codex 토큰 검증·ollamaCloud 모델 목록(네트워크)은 4.2 허락 항목이라 자동 검사에서 부르지 않았습니다.
- (7절에서 닫힘) 처음 settings-38 App 검사는 토글이 세 값을 `settings.json`에 쓰는 것까지만 확인하고 편집기 소비자는 `application-ai-completion-tests.rs`를 인용했습니다.
- settings-39(프롬프트 파일 열기)는 S3 묶음입니다. AI section의 프롬프트 3행은 아직 없습니다.
- S1의 나머지 골격(Interface 행, Notifications 버튼, 대화상자 모듈, Performance, `new_with_overrides`)은 S1 묶음 몫입니다.

## 7. 수정(리뷰 반영, 2026-11-03)

### 7.1 settings-38 — 과대 판정 반영

- 리뷰 지적: 사양 [A]는 "클릭 → 설정 반영 → 편집기 자동완성 요청 경로가 켜짐"인데 새 App 검사는 `settings.json` 세 값까지만 확인하고 소비 경로는 인용 근거였음.
- 확인: 지적이 맞습니다.
- 수정: `application-settings-sections-tests.rs`에 `실제_auto_tab_토글은_편집기_자동완성_요청_경로를_켠다`를 추가했습니다(실제 `NativeApplication`, 주입 completion 함수 `editor_ai_completion::State::with_complete`로 요청만 기록하고 네트워크 없음).
    1. 설정: provider oMLX, base URL `http://127.0.0.1:9`(연결 거부 loopback; base URL이 있어야 oMLX가 configured라 토글이 켜짐, 모델 목록 조회는 실패 상태로 남음), model `fixture`, auto-tab 꺼짐.
    2. `HostCommand::Editor(OpenFileTab)`로 프로젝트 파일 탭을 열고 본문 끝에 `;` 입력 → 문서가 `let value = ;`로 바뀌었지만 요청 0건(꺼짐).
    3. `ShellMutation::ActivateTab`으로 Settings 탭 → `settings.aiAutoTabToggle` 클릭 → 메모리 `ai_auto_tab_enabled`.
    4. 파일 탭으로 돌아가 `;` 입력 → 요청 1건: prefix `let value = ;;`, provider oMLX, model `fixture`, 같은 파일(경로는 canonicalize 비교).
- 1차 실행 실패 원인: (a) base URL 없는 oMLX는 configured가 아니라 토글이 비활성이었음 → 설정에 base URL 추가, (b) 요청 `file_path`가 `/private/var/...` 정규 경로라 문자열 비교가 어긋남 → canonicalize 비교. 둘 다 검사 준비의 문제였고 제품 코드는 바꾸지 않았습니다.

### 7.2 검증

| 범위 | 명령(공통 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`) | 결과 |
| --- | --- | --- |
| [A] settings-38 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- 실제_auto_tab_토글은 --test-threads=2` | 1 passed(2회 실패 뒤 위 두 준비 수정) |
