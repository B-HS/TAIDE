# M8 AppFile 가상 문서·읽기 경계

## 대상 파일

- `crates/taide-model/src/app.rs`
- `native/taide-native-editor/src/{document,store}.rs`, `tests/app-file.rs`
- `native/taide-native-app/src/{app-file,persistence,lib}.rs`, `tests/app-file.rs`

## 리포트

이 문서는 읽기 코어 완료 당시 기록입니다. 이후 실제 header/HostBridge/AppSurfaces/NativeEditor 읽기 연결·SettingsChanged·owner/model 수명은 [AppFile surface 후속](2026-10-03-m8-native-app-file-surface.md), 실제 쓰기 API는 [write 후속](2026-10-03-m8-native-app-file-write.md)에서 구현·검증했습니다. 아래 당시 미연결 목록을 현재 코드의 전체 상태로 승계하지 않습니다. 원본에 없는 AppFile dirty 확인은 추가하지 않으며 실제 저장 키/설정 포트·전체 Exit는 계속 미완료입니다.

AppFile 전용 문서 identity와 canonical text 반영 코어, 감독된 읽기·문서 admission을 구현했습니다. 실제 Settings header/HostBridge/AppSurfaces/NativeEditor 배선과 설정·프롬프트 쓰기·dirty 닫기·앱 종료는 아직 구현하지 않았습니다. 저장 완료 코어 검사를 실제 파일 저장 성공으로 확대하지 않습니다. M8 N1~N8은 0/8이며 목표는 active입니다.

## 확인한 원본·계약

`src/widgets/app-file-pane/app-file-pane.tsx`, `src/entities/app-file/{app-file-model-path,app-file.query}.ts`, `src/app/providers/ipc-sync-provider.tsx`, `crates/taide-runtime/src/{app_actions,settings_actions}.rs`, `crates/taide-app/src/service.rs`를 확인했습니다.

- 원본 Monaco 가상 model path는 Settings와 닫힌 prompt id 3종별로 공유됩니다. native에서는 path-shaped 문자열 대신 `DocumentKey::AppFile(AppFileTarget)`을 씁니다. 기존 wire 형태는 유지하고 모델에 Hash derive만 추가했습니다. 실제 파일·Untitled와 identity가 충돌하지 않습니다.
- JSON 언어·normal/editable metadata이며 실제 경로·수정 시각·외부 파일 관측이 없습니다. AppFile은 일반 `mark_saved`에서 거절하고 미러 worker에서도 거절합니다. LSP의 기존 File-only admission과 App의 기존 File-only autosave/watcher 분기는 유지했습니다. 실제 앱에서 이 분기들이 함께 실행되는 검사는 아직 없습니다.
- dirty 상태는 공유 문서가 소유합니다. view detach/remount와 다른 창의 view도 같은 document를 사용하며 dirty release는 거절합니다. 실제 탭 close/save UI는 별도 미완료입니다.
- `mark_app_file_saved`는 정확한 document/target·저장 revision·본문 크기를 검사합니다. 저장 뒤 추가 편집은 유지하고 dirty를 남깁니다. 요청 본문과 현재 본문이 같을 때만 canonical text를 반영하며 변경 시 revision 갱신·selection clamp·IME/fold/syntax/undo 재설정을 수행합니다. 일반 파일 disk-conflict 관측으로 바꾸지 않습니다.
- SettingsChanged는 원본 APP_FILE(Settings) query도 무효화합니다. `refresh_app_file`은 clean만 새 본문을 채택하고 dirty를 유지합니다. native SettingsChanged→재읽기 배선은 아직 없습니다.
- `Session`의 폐기 ticket과 요청별 operation identity, owner의 project/pane/tab/target 검증을 구현했습니다. inactive tab은 여전히 존재하면 읽기 결과를 보존할 수 있지만 닫힘·target 교체·세션 폐기는 거절합니다. renderer가 pending request와 `same_request`를 비교하는 연결은 다음 구현입니다.
- 실제 읽기는 기존 runtime app_file_read의 파일 override/현재 Settings·bundled prompt fallback을 재사용합니다. owned mutation guard와 TaskOperationLease를 worker/admission까지 보유하고 blocking worker에서 실행합니다. commit 직전 owner/ticket을 다시 검사하고 guard/lease를 해제합니다. Tokio 1.53.1 설치 소스 `runtime/handle.rs`의 blocking thread Handle::block_on 계약과 기존 TaskSupervisor run_blocking_result 구현을 확인했습니다.

## 실행한 최소 검증

모든 Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`으로 한 번에 하나만 실행했습니다.

| 검사                                                                        | 실제 결과                          | 직접 확인한 위험                                                                                                                                          |
| --------------------------------------------------------------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| editor manifest `cargo test --test app-file`                                | 2 PASS, suite 0.00초·컴파일 0.34초 | 닫힌 target/공유 view·dirty remount·일반 저장/관측 거절·용량/identity 실패 원자성, 저장 중 추가 편집·canonical/selection/IME/undo·stale/released document |
| app manifest `cargo test --test app-file` (refresh 추가 전)                 | 1 PASS, suite 0.00초·컴파일 1.17초 | 실제 current Settings/override·prompt3 fallback, wrong owner/target·폐기/잠금 대기 뒤 취소·lease 회수, 미러 거절·project file grant 미생성                |
| app manifest `cargo test --test app-file native_app_file_refresh`           | 1 PASS, suite 0.01초·컴파일 7.08초 | clean override 채택·revision, 이후 dirty 재읽기 보호·disk-conflict 미생성                                                                                 |
| app manifest `cargo clippy --lib --bins --tests -- -D warnings`             | exit 0, 20.95초                    | 새 DocumentKey에 따른 App 전체 match/계약과 authored 코드·검사                                                                                            |
| editor manifest `cargo clippy --lib --test app-file -- -D warnings`         | exit 0, 1.61초                     | editor 코어와 직접 검사 코드                                                                                                                              |
| authored 8파일 `rustfmt --check --edition 2024 --config skip_children=true` | exit 0                             | 이번 변경 파일 포맷                                                                                                                                       |

서로 다른 관련 검사 4건 PASS입니다. refresh 변경은 새 영향 검사로 덮고 앞선 요청 인증·core 저장 성공은 재사용했습니다. 전체 suite는 실행하지 않았으며 기존 keybinding Tab RED가 남아 있어 전체 green으로 보고하지 않습니다. 기존 Wry 17 warnings는 별개이며 suppression을 추가하지 않았습니다.

첫 editor test compile의 Selection::caret/secondary/primary 잘못된 fixture API(E0599/E0560/E0610)를 실제 view.rs 계약으로 수정했습니다. app fixture의 AppPaths.data_dir를 메서드로 호출한 E0599도 실제 public field로 수정했습니다. 제품 런타임 재현 오류가 아니라 테스트 코드 컴파일 오류이며 통과한 검사 반복으로 집계하지 않습니다.

## 남은 구현·검증

- [x] 가상 문서 identity/metadata·dirty 수명·canonical 저장 완료 코어
- [x] 감독된 읽기·owner/ticket/admission·clean 갱신/dirty 보존
- [x] 위 최소 검사와 정적 검사, 성공 재사용
- [ ] Settings header·typed OpenAppFile/ReadAppFile HostBridge·실제 NativeEditor/키맵/SettingsChanged 재읽기와 pending/failure UI
- [ ] 실제 설정·프롬프트 쓰기·atomic persistence/parse/sanitize·오류 toast·canonical 재읽기·저장 경합과 취소/worker lifetime
- [ ] dirty close/CloseAll/project/window/exit·공유 target의 survivor 처리와 실제 GUI/AX/픽셀

Settings 전체 JSON 쓰기는 기존 presentation-only no-op reconcile을 그대로 사용할 수 없습니다. 원본은 IDE server→agent hooks→remote access 순서로 실제 시작/종료를 기다린 뒤 SettingsChanged를 보냅니다. native AppServices에는 store가 있지만 서버 조립과 포트가 아직 연결되지 않았습니다. 실제 포트까지 구현하기 전 Settings JSON 저장 완료를 주장하지 않습니다. 원본 settings 실패의 고정 settingsJsonInvalid 메시지와 Prompt의 describeIpcError도 보존해야 합니다.

현재 원본 read_app_file은 read_to_string을 사용해 admission 이전 문자열 할당이 파일 크기에 비례합니다. 기존 Core의 max_document_bytes는 admission/편집/canonical 반영 상한이며 읽기 할당의 상한이 아닙니다. 전체 메모리/큰 설정·프롬프트 정책은 구현·성능/보안 게이트에서 별도로 확인하며 이미 제한됐다고 주장하지 않습니다.

실제 OS 설정·IME·VoiceOver·보호 bundle·제품 TS·dependency/lock/MSRV·Git을 변경하지 않았습니다. 테스트 디스크는 고유 synthetic 임시 디렉토리뿐입니다. 전체 M8 완료 뒤만 선별 commit·일반 push합니다.
