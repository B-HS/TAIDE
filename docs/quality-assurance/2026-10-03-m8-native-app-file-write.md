# M8 AppFile 저장 작업자·canonical 완료 경계

## 2026-10-05 실제 App 저장 caller 배선

최종 source 정정: 원본 `src/widgets/app-file-pane/app-file-pane.tsx`의 draftRef=null·initialDirty seed와 Settings 고정 오류/Prompt describeIpcError 계약을 직접 확인했습니다. 처음 열린 clean model은 쓰기를 보내지 않고, 실제 editor change는 해당 공유 document의 has_draft를 기록해 편집 후 undo/재저장 시의 draft를 잃지 않습니다. 단순 clean 여부만으로 저장을 막지 않습니다. Toasts.app_file_failed를 actual App 완료/준비 실패 양쪽에 연결해 Settings는 `settings.settingsJsonInvalid`의 제목만, Prompt는 기존 번역 IPC 오류를 표시합니다. 최초 전체 배선 검사의 일반 ipc_error 표현은 이 target별 원본 계약으로 정정됐습니다.

source 정정 후 `cargo test --lib app_file_ -- --nocapture`는 compile10.29초/suite0.03초·4 PASS(새 toast1·변경된 save1·Entry field 영향 read2)입니다. 이후 실제 editor change의 draft 기록 연결을 추가하고 `cargo test --lib app_file_views::tests::app_file_save_host -- --nocapture`만 compile8.61초/suite0.02초·1 PASS했습니다. 초기 clean 미입장과 명시 draft의 clean 재저장 admission도 검사합니다. 이 단계에서 toast/host/graph/Settings의 입력·환경·관련 코드가 같은 성공은 다시 실행하지 않았습니다. 두 검사 사이 실제 코드가 달라졌으며 동일 상태의 반복 계측이 아닙니다. dirty 완료 patch와 draft test patch는 formatter 문맥 불일치로 적용되지 않았고 실제 줄을 확인해 한 번씩 적용했습니다.

원본 컴포넌트의 모든 활성/비활성 remount·initialDirty/model sync와 실제 GUI/물리 입력/표시·정확한 save event 수 및 전체 source view parity는 여전히 최종 gate입니다. source 대조 없는 API 가정으로 이를 완료하지 않습니다.

최종 source 상태의 `cargo clippy --lib --bins --tests -- -D warnings`는 exit0·3.93초입니다. authored5 exact Rustfmt/check와 tracked 및 신규5 no-index whitespace 검사도 오류 출력이 없습니다. 신규 diff의 exit1은 파일 추가 차이입니다. live 검사/대기 handle은 없으며 전체 M8 완료 전 Git은 하지 않습니다. 아래 3.76초 strict와 최초13.66초 save 검사는 source 정정 전의 선행 결과로 구분합니다.

대상은 `host.rs`, `app-file-views.rs`, `app-file-views-tests.rs`, `application.rs`입니다. 실제 editor→ShellIntent::RequestSaveTab→NativeApplication::request_tab_save에 AppFile owner/document 분기를 추가했습니다. main/auxiliary의 같은 model을 target/document별 단일 pending으로 보호하며 HostCommand::WriteAppFile와 HostReply::AppFileWritten은 operation identity를 보존합니다. HostBridge의 기존 감독 작업자가 필수 reconcile을 사용해 guard 없는 apply_and_broadcast→IDE/hooks/remote await 경계를 소비하고 기존 WriteRequest/PreparedWrite API로 canonical 완료를 돌려줍니다. UI가 요청/Session identity를 확인하고 commit한 뒤 현재 dirty 값을 같은 target의 모든 탭에 pending_dirty로 발행합니다. 실패는 기존 ipc_error toast이며 제출 실패·실패 완료·owner 폐기의 pending을 회수합니다. 파일/untitled 저장·AppFile dirty-confirm 추가 금지·mirror 제외 계약은 유지합니다.

기존 무통합 HostBridge에는 명시적 None을 보관합니다. 일반 Settings controls의 이전 호환 동작은 유지하지만, 새로운 Settings AppFile 저장은 실제 reconcile이 없으면 쓰기 전에 Forbidden으로 거절합니다. no-op callback으로 full settings 파일 저장을 성공시키지 않습니다. prompt target은 원본 runtime 경로대로 Settings callback을 호출하지 않습니다. **실제 NativeApplication은 아직 ApplicationPorts owner/start를 조립하지 않으므로 현재 앱의 Settings 파일 저장은 이 명시적 연결 거절 상태입니다.** 전체 Settings/App startup/toggle 또는 GUI 완료로 세지 않습니다.

직접 검사와 결과는 다음과 같습니다.

- 신규 `cargo test --lib app_file_views::tests::app_file_save_host -- --nocapture`: compile13.66초/suite0.03초·고유1 PASS입니다. 실제 공유 문서/host/blocking write에서 owner 선택·공유 pending·다른 Session의 cancel 거절·reconcile 동안 완료 없음·live/disk 적용·revision 갱신에도 중복 방지·저장 중 추가 편집 dirty 보존·중복 완료 무시·invalid JSON 실패/디스크 보존·callback1회·재시도/제출 실패 회수 경계·무통합 Settings Forbidden·owner 폐기/다른 owner 계속 사용·disconnect/task0을 확인했습니다. OS 화면·실제 CmdS 물리 입력·toast 픽셀·포화된 실제 큐·enabled 서버를 이 검사로 주장하지 않습니다.
- 변경된 host/Views/constructor의 기존 read/shared model 및 file-save 배정 영향4개만 직접 libtest binary의 exact 이름으로 실행해4 PASS·suite0.05초입니다. production_graph의 동일 Tabs/Hub와 actual Ports reconcile도 포함합니다. Application reply의 dirty 처리만 추가한 뒤 해당 pure worker/Views의 신규 성공은 반복하지 않았습니다.
- 변경된 Option/reconcile 경로의 `cargo test --test settings-controls native_settings_controls_host -- --nocapture`: compile12.12초/suite0.17초·고유1 PASS입니다. 이전 상태의 성공과 다른 코드 상태이므로 관련 검사만 한 번 확인했습니다.
- 초기 lib `cargo check --lib`는 exit0·13.78초입니다. final `cargo clippy --lib --bins --tests -- -D warnings`는 exit0·3.76초이며 새로운 enum variant의 기존 consumers까지 타입 계약을 확인합니다. authored4 Rustfmt/check와 tracked diff/check는 exit0입니다. Wry17 dependency warnings는 authored 진단과 구분합니다.

모든 Cargo는 기존 manifest·CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo·locked/offline·target=/private/tmp/taide-m8-menu-build.j6Efnw에서 직렬입니다. 초기 dirty 처리 patch는 formatter와 문맥이 달라 적용되지 않았고 실제 줄을 확인해 적용했습니다. 테스트 함수의 잘못된 일본어 이름은 실행 전에 한국어로 정정했습니다. 기존 API/core 저장 성공은 재사용하며 새로운 dependency/manifest/lock/MSRV/제품TS/보호 bundle/사용자 OS·clipboard·Keychain/Git 변경은 없습니다.

후속47은1/4(25%)·전체362/433(83.60%, 비가중/공수비 아님)·최종 N1~N8 0/8입니다. 현재 code wiring만 완료했고 실제 App 필수 geometry/palette·원격 응답 소유권·Rust assets/owner/start/Exit 및 아래 남은 실기/전체 quota 게이트는 미완료입니다. 전체 ETA는 그 구현 공수가 확정되지 않아 산정 보류입니다.

## 대상 파일

- `crates/taide-runtime/src/app_actions.rs`
- `native/taide-native-app/src/{app-file,app-file-write,lib}.rs`
- `native/taide-native-app/tests/app-file.rs`

## 선행 작업자 구현 리포트

Session.write_request/owns_write·WriteRequest.execute·PreparedWrite.commit을 구현했습니다. 실제 target 검증·Settings JSON parse와 필수 apply callback, prompt3 검증/atomic persistence를 기존 runtime 경로로 재사용합니다. 저장 후 canonical read와 문서 완료·추가 편집 보존, 잠금 대기 뒤 취소·lease 수명을 확인했습니다. 아직 HostBridge write/화면 CmdS에 연결하지 않았으며 실제 IDE/hooks/remote 설정 포트도 조립 전입니다. 전체 Settings/M8 완료가 아닙니다.

## 상세

1. snapshot의 DocumentKey와 Session owner target이 정확히 일치해야 요청을 만듭니다. 새 요청은 operation identity를 가지며 Session ticket의 폐기와 project/pane/tab/target/shutdown 경계를 읽기와 공유합니다. 아직 renderer의 pending/중복 완료 admission은 연결 전입니다.
2. native worker는 operation lease를 만들고 owned mutation을 기다린 뒤 authority를 다시 검사합니다. 기존 app_file_write가 내부에서 같은 mutex를 잡으므로 이를 그대로 중첩 호출하지 않습니다. runtime에 admitted 본문을 분리했고 기존 entry는 여전히 자체 guard를 획득한 뒤 같은 본문을 호출합니다. admitted API의 호출자는 정확한 state mutation guard를 보유해야 합니다.
3. blocking worker의 Handle::block_on으로 기존 parse/write와 callback을 실행합니다. Settings apply callback은 필수 인자이며 선택적/no-op 기본 구현이 없습니다. 실제 조립은 내부 guard 없는 apply_and_broadcast와 IDE→hooks→remote awaited 순서를 사용해야 합니다. 테스트 callback은 실제 settings_actions parse/sanitize/persist/live state/canonical 경로를 사용하지만 domain observer는 synthetic assertion입니다. 실제 서버 시작/종료 성공을 주장하지 않습니다.
4. prompt는 기존 닫힌 id3종과 해당 JSON schema를 검증하고 write_atomic을 사용하며 Settings callback을 호출하지 않습니다. 저장 성공 뒤 기존 read_app_file로 canonical content를 얻고 guard/lease를 UI commit 또는 drop까지 보유합니다. commit은 authority와 document/target/revision을 재검사합니다. 저장 중 추가 편집은 기존 mark_app_file_saved가 보존하며 dirty를 남깁니다.
5. 디스크 쓰기가 이미 시작·완료된 뒤 취소하거나 canonical read/문서 commit이 실패하면 쓰기를 rollback하지 않습니다. 비중단 worker의 실제 파일 성공과 UI clean 완료는 별개의 경계입니다. 화면 조립은 이 provenance와 dirty/실패를 구분해야 합니다. 큰 문자열/read allocation과 전체 메모리 상한도 미완료입니다.

## 실제 최소 검증

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`으로 직렬 실행했습니다.

- app manifest `cargo test --test app-file native_app_file_write`: 신규2건 중 prompt3 schema/실제 atomic override/invalid 기존 파일 보존·잠금 대기 뒤 Session 취소·잘못된 target·file grant 미생성·lease 회수1 PASS, 전체 suite0.06초·컴파일13.41초입니다. Settings fixture는 version 등 필수 스키마가 없는 부분 JSON을 썼기 때문에 error.settings.schemaInvalid로 FAIL했습니다. 실제 전체 Settings를 serialize하도록 정정했으며 제품 파서를 완화하지 않았습니다.
- `cargo test --test app-file native_app_file_write는_실제_settings_apply`: 관련 실패만1회 재실행해1 PASS, suite0.04초·컴파일2.45초입니다. invalid JSON이 callback/파일/live state에 도달하지 않음·operation identity/다른 Session 거절·9999→48 보정/실제 저장/canonical 채택/clean·추가 편집 dirty/기존 디스크 보존을 확인했습니다. legacy app_file_write가 여전히 자체 mutex를 기다림도 같은 검사에서 확인했습니다.
- runtime manifest `cargo clippy --lib -- -D warnings`: exit0,7.96초입니다. root MSRV1.89/edition2021과 기존 환경을 유지했습니다.
- native app manifest `cargo clippy --lib --bins --tests -- -D warnings`: 쓰기 추가 뒤 exit0,17.21초입니다. 이후 닫기 계약 수정 영향의 최종 검사 exit0,13.10초를 재사용합니다. Wry17개 기존 dependency warning과 authored strict 검사를 구분하며 suppression은 없습니다.
- authored root1파일 Rustfmt edition2021 및 native4파일 edition2024 exact check exit0, tracked diff check exit0입니다. 신규 파일 no-index check의 빈 출력 exit1은 파일 추가 차이가 있음을 뜻하며 whitespace 오류가 아닙니다.

서로 다른 쓰기 검사2건 PASS이며 앞선 읽기/core/ThemeEditor·이번 surface/header/icon 성공은 다시 실행하지 않았습니다. whole suite의 keybinding Tab RED는 별도로 남습니다.

## 다음 구현

- [x] 실제 runtime 쓰기 재사용·필수 Settings apply·canonical 완료·추가 편집/대기 취소 API
- [x] 서로 다른 쓰기2건/관련 static 검사·성공 재사용
- [ ] HostBridge typed write·target/doc별 pending·같은 request admission·submit failure·화면 CmdS/오류 toast와 dirty 발행: 2026-10-05 code caller/직접 검사는 구현했으나 실제 App integrations/물리 입력·포화·GUI는 미완료입니다.
- [ ] 실제 Settings apply/IDE/hooks/remote 포트 조립·원본 순서/시작/종료/실패·SettingsChanged 이전 완료
- [ ] 닫힌/new AppFile 탭의 initialDirty/model sync/view state·전체 GUI/AX/aux/project/Exit/메모리/성능

원본 AppFile의 닫기에는 새 dirty 확인을 추가하지 않습니다. load_layout은 AppFile persisted dirty를 지우고 hot-exit mirror는 없습니다. 다음 연결에서 이 기존 계약을 새로운 복구 기능으로 바꾸지 않습니다. OS/보호 bundle·제품TS·dependency/lock/MSRV·Git을 변경하지 않았으며 전체 M8 완료 뒤만 commit/push합니다.
