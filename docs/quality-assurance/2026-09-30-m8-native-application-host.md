# M8 native executable·실제 서비스 조립 첫 구현

## 대상과 상태

`native/taide-native-app/{Cargo.toml,Cargo.lock,src/*,tests/host.rs}`를 추가했습니다. AppServices·TaskSupervisor·ShellController·canonical native editor를 조립하는 첫 실행 경로입니다. 합성 shell spike의 fixture를 앱으로 이름만 바꾸지 않았으며 기존 서비스와 실제 파일 경계를 사용합니다. 아직 다수 host action/surface가 미연결이므로 N2/M8 완료나 기존 앱을 대체할 수 있는 상태가 아닙니다.

## 실제 조립

- 실행 시 `--data-dir <absolute directory>`를 반드시 요구합니다. 상대 경로·루트·부모 경로 성분·누락/추가 인수는 거절하고 기존 앱 data-dir을 자동 선택하지 않습니다. 지정된 경로가 사용자 데이터인지 자동 판별하는 sandbox 정책은 아니므로 사용자/검증 harness가 격리 경로를 제공해야 합니다.
- 단일 multi-thread Tokio Runtime의 Handle을 기존 TaskSupervisor에 전달합니다. 등록된 worker에서 기존 session/layout/settings 복원을 실행하고 같은 상태·감독자·store들을 AppServices로 조립합니다. Keyring은 격리 service 이름으로 생성만 하며 검사에서 secret 조회/쓰기를 호출하지 않습니다. remote 동시성 128은 기존 Tauri assembly 상수와 같은 값이며 공유 상수 위치 통합과 실제 remote host 활성화는 남습니다.
- eframe 0.36.2의 native entry와 기존 egui shell/controller를 연결합니다. OS theme 관측값과 기존 locale resolver/테마 로딩을 사용합니다. OS theme 관측값이 없는 경우 dark, locale가 없는 경우 기존 resolver fallback을 사용하며 이 fallback을 실제 OS 값이라고 주장하지 않습니다.
- 최초 shell/editor 색과 editor font size/line number/indent를 기존 ResolvedTheme/Settings에서 가져옵니다. 현재 색은 6/8자리 hex만 처리하므로 CSS·짧은 hex 등 모든 user theme 동등성은 미완료입니다. 실제 font-family/CJK fallback 및 per-file editorconfig 매핑도 남습니다.
- 64개 command/reply의 native HostBridge가 기존 AppServices의 guarded file open/save·plugin language overlay·tree row/toggle·layout dirty를 실행합니다. 실제 runtime TaskSupervisor가 worker를 소유하며 별도 감독자를 만들지 않습니다. 큐는 count 제한이며 payload/RSS quota·per-window 공정성·고빈도 dirty coalescing은 남습니다.
- GUI owner가 PreparedDocument를 commit해 실제 EditorStore에 입장시키고 main window/pane/tab별 ViewId를 연결합니다. file-tab 본문은 NativeEditor를 렌더하고 save 요청은 immutable snapshot을 기존 파일 쓰기로 전달합니다. 완료 때 저장 snapshot만 baseline으로 채택해 저장 중 새 편집의 dirty를 유지합니다.
- 첫 host 당시에는 dirty mirror가 있는 path의 열기를 거절했습니다. 후속 `2026-10-01-m8-native-tab-close-and-draft-restore.md`에서 기존 파일의 canonical draft 복원·live shared body 보호를 구현해 이 거절은 제거했습니다. missing source/전체 conflict UX·untitled/app-owned mirror는 아직 남습니다.
- close는 입력을 멈추고 pending command/reply/token을 폐기한 뒤 worker 완료를 기다립니다. dirty 파일의 기존 mirror와 layout 저장이 성공한 뒤 기존 ExitDrain으로 watcher/LSP/PTY/AI/작업을 회수합니다. layout은 저장 실패를 propagate하고 성공한 항목만 dirty에서 제거합니다. 실패를 로그만 남기고 완료 처리하지 않습니다. 외부 CLI 파일의 dirty close는 먼저 저장해야 하며 확인/Save As UX는 남습니다.

## 검증 근거

고정 공식 eframe `epi::App`의 ui/on_exit/close intercept와 sys-locale 0.3.2 API, 기존 runtime 파일/tree/종료/감독자 source를 확인했습니다. GUI 후보는 기존 버전의 격리 edge만 재사용하고 root 제품 manifest/lock/MSRV는 이번 변경으로 수정하지 않습니다. 독립 native-app lock에는 GUI graph가 추가됐으며 최종 제품 의존성 확정으로 계산하지 않습니다. eframe upstream을 사용해 아직 실험 vendor의 child AX 수정은 제품 조립에 채택하지 않습니다.

모든 Cargo 검사는 `--offline --target-dir experiments/native-shell-spike/target`을 사용했습니다. 처음 lock을 만든 뒤에는 `--locked`를 사용했습니다.

- [x] `cargo check --manifest-path native/taide-native-app/Cargo.toml`: 첫 compile은 entry의 TaskSupervisor::new runtime Handle 인수 누락으로 exit 101이었습니다. 실제 API에 맞춘 뒤 check exit 0, 0.38초입니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test host`: 2 passed/0 failed, 0.03초입니다. CLI 경계와 합성 temp 파일의 host 열기→admission commit→canonical 편집→실제 파일 저장→baseline 완료→dirty mirror 거절을 확인했습니다. link build 16.89초이며 사용자 shell/앱·OS port·secret store는 호출하지 않았습니다.
- [x] 저장 실패를 엄격하게 반환하는 새 `--test host 종료_layout`: 1 passed/0 failed, 0.01초입니다. 합성 data 경로가 파일인 실패 조건에서 dirty layout을 유지하고 operation 추적을 회수합니다.
- [x] 미소비 token 검사에서 repaint 알림만 기다리는 대신 reply queue가 실제 비어 있지 않음을 확인하도록 강화한 뒤 `--test host 실제_native_host`만 실행했습니다. 1 passed/0 failed, 0.02초이며 queued PreparedDocument가 소비되지 않은 상태의 disconnect→worker 완료→mutation lock 회수·tracked 0입니다. unchanged CLI 검사는 반복하지 않았습니다.
- [x] strict clippy의 초기 collapsible-if/chunks-exact 경고를 검사기 억제 없이 수정했습니다. 마지막 `cargo clippy --lib --bin taide-native-app --test host -- -D warnings`는 exit 0, 0.44초입니다.
- [x] `experiments/native-shell-spike/target/debug/taide-native-app`의 실행 가능 파일 존재를 확인했습니다. 바이너리를 실행해 창/OS port를 검증한 것은 아닙니다.

기존 shell/controller/document/editing/surface의 unchanged 성공 증거는 재사용했습니다. root M8 상위 N1~N8은 여전히 0/8이며 commit/push 완료 조건을 충족하지 않았습니다.

## 남은 실제 연결

- [ ] 전체 project lifecycle·IDE/agent/remote reactions, OS menu·dirty tab close·project close draft handshake·branch/status. 첫 attach/restore watcher·폴더/파일 dialog 호출·탐색기 탭/갱신의 코드 연결은 아래 후속 기록이며 전체 lifecycle·실제 OS dialog/픽셀 통과는 아닙니다.
- [ ] untitled/app-file/diff/terminal/settings와 나머지 213 TS view, 전체 hot-exit 복원/conflict/rekey/close/discard, view eviction/세션 view 상태·autosave/periodic persistence. 기존 file draft와 단일 tab close의 첫 후속 구현은 `2026-10-01-m8-native-tab-close-and-draft-restore.md`에서 구분합니다.
- [ ] 전체 theme/locale interpolation/font-family/CJK·shaping·AX·keymap·다중 창/child AX·창 geometry/drop/context menu, shell title/OS chrome 동등성
- [ ] UI/OS 오류의 3-locale 표시·queue retry/worker failure·exit 실패 재시도/직접 강제 종료 오류 보고, canonical mirror alias·외부 FS race·save metadata/전체 payload/RSS quota
- [ ] 실제 GUI·CJK IME/VoiceOver·GPU 복구/software·성능/beta/rollback·서명/공증/install/upgrade와 최종 TS 제거/Rust99%/완료 Git

현재 미구현 host action/tab surface는 명시적으로 미연결 오류를 표시합니다. 이를 동작하는 원본 view의 대체나 완료로 계산하지 않으며 해당 코드를 채워 전체 동등성을 달성해야 합니다. 사용자의 기존 앱/데이터/실기 bundle과 OS 입력기·VoiceOver 설정은 변경하지 않았습니다.

## 후속: 프로젝트·dialog·탐색기 실연결

대상은 같은 패키지의 `src/{projects,events,host,application,lib}.rs`, `tests/projects.rs`, 독립 manifest/lock입니다. 기존 root 제품 manifest·MSRV·Tauri adapter는 변경하지 않았습니다. `rfd 0.17.2`는 이미 shell spike에서 사용한 고정 버전을 이 격리 실행 패키지에도 연결했고 `taide-ide`·`log`는 기존 graph를 재사용했습니다. 최종 제품 의존성 확정이 아닙니다.

- 폴더 열기는 기존 `project_actions::project_open`의 기록·attach·실패 rollback·event 정책을 그대로 호출합니다. 레이아웃→파일 watcher→Git watcher→terminal/cache→IDE lockfile 순서를 유지하고, 비용 있는 build는 기존 `run_project_build`의 등록 blocking worker/operation으로 소유합니다. mutation guard를 잡기 전 build하고 guard 안에서 live root·shutdown을 재검사해 등록합니다. 복원은 기존 active/session 순서·missing-root 제외 함수를 사용하고 이미 있는 watcher/layout을 덮지 않습니다.
- 파일 watcher의 한 debounce tick 전체를 기존 `resolve_from_app`에 넘깁니다. Git watcher는 기존 `.git` directory 판정과 index/HEAD/refs 분류·cache invalidate-before-event 정책을 사용합니다. 두 watcher 모두 기존 stop scheduler와 ExitDrain으로 join합니다. 이 native의 Git 분류는 현재 Tauri 분류와 별도 source에 같은 계약을 구현했으며 정책 단일화와 `.git` file worktree 지원은 후속입니다.
- host의 `OpenProject`/`RestoreWatchers`/`OpenFileTab`을 실제 앱에 연결했습니다. main-thread OS dialog는 고정 rfd source의 `FileDialog::set_parent(Frame)`/`pick_folder`/`pick_file`를 사용하고 file dialog는 해당 프로젝트 root를 초기 위치로 잡습니다. 취소는 mutation을 제출하지 않고 비 UTF-8 경로는 명시적으로 거절합니다. 프로젝트 밖 선택 파일은 기존 root guard를 그대로 적용해 거절하며 CLI 승인 집합을 넓히지 않습니다. 실제 dialog UI·재진입·취소·키보드 검사는 아직 하지 않았습니다.
- 탐색기 단일 클릭은 preview, double click은 permanent file-tab 요청으로 같은 layout service에 전달합니다. 기존 tab dedupe·dirty preview promotion을 재사용합니다. 전체 tree를 반환하는 원본 TS query와 같이 `limit: None`을 사용해 최초/토글 256행 잘림을 제거했습니다. 가상 행 렌더는 유지합니다.
- 실제 `FsChanged`/`FsRescanRequired`는 프로젝트·parent directory별로 합칩니다. self-write modified만 tree 갱신에서 제외하고 create/rename/remove는 제외하지 않습니다. 대기 프로젝트 64개·directory 128개 초과는 조용한 유실 대신 rescan으로 승격합니다. rescan은 원본 `rescanTreeRefreshDirs`처럼 root·현재 전체 page의 expanded directory를 실제 `tree_refresh`로 읽고, 다중 경로 중 실패가 있어도 나머지를 처리한 뒤 최종 rows와 오류를 반환합니다. 숫자 경계는 로컬 명명 상수이며 전체 memory/per-project fairness gate는 미완료입니다.
- command queue 포화 때 파일/tree loading flag를 회수하고 repaint로 재시도하며 감시 invalidation을 다시 합칩니다. GUI dirty 상태는 tab별 최신값으로 보존해 다른 파일 탭 열기 전에 먼저 제출합니다. dirty 제출이 포화되면 lower-priority action은 실행하지 않고 오류를 표시합니다. 이 순서의 실제 GUI 입력/포화 검사는 아직 없으며 최종 queue UX 게이트로 남습니다.
- `agent_hooks_enabled`가 켜진 상태는 미연결 reconciliation을 성공으로 속이지 않고 프로젝트 mutation 전에 명시적으로 거절합니다. 기본 off의 watcher 연결은 동작하며 enabled 경로는 후속 구현입니다. IDE server가 없으면 기존과 같이 lockfile 갱신은 할 일이 없고, server context가 있으면 실제 기존 atomic writer를 호출합니다. 실제 IDE startup/server·remote 연결은 남습니다.
- native watcher/IDE build 실패는 현재 명시적 attach 오류로 반환합니다. 원본 Tauri의 watcher 실패 warning 후 degraded attach보다 엄격한 첫 host 정책이며 최종 degraded UX를 동일하게 맞추는 작업이 남습니다. 기존 `ProjectLifecyclePort::await_project_flush`는 오류 반환형이 아니므로 rollback만 기존 warning 정책으로 처리합니다. 정상 프로젝트 닫기 UI는 draft flush/오류 확인 경계를 구현하기 전 노출하지 않았습니다. 앱 정상 종료의 별도 엄격 layout flush는 유지했습니다.

### 이번 검증

모든 검사는 같은 `--offline --target-dir experiments/native-shell-spike/target`을 사용하고 lock 갱신 이후에는 `--locked`를 사용했습니다. 사용자의 실기 앱/bundle·입력기·VoiceOver는 변경하지 않고 GUI 실행·dialog 호출·IDE socket·secret 조회를 하지 않았습니다.

- [x] 대상 `cargo check`: 고정 rfd graph 연결 후 exit 0, 5.48초. 이후 event consumer 연결 상태 check exit 0, 0.41초입니다.
- [x] 새 `cargo test --test projects`: 최초 test compile에서 잘못 쓴 supervisor 조회 메서드명 E0599와 미사용 import를 고쳤습니다. 실제 source의 `tracked_count`로 수정한 뒤 3 passed/0 failed, 0.17초입니다. 합성 project open·attach/restore idempotence·Git/Terminal capability 순서·300개 이상의 모든 tree 행·탭 dedupe/dirty preview 보존·rescan 실제 새 파일 반영·root 밖 파일 거절·종료 join을 확인했습니다. enabled hook 사전 거절과 합성 IDE lockfile 경로 실패로 실제 watcher build 후 attach 실패/rollback/stop 회수도 확인했습니다.
- [x] 추가 `cargo test --test projects 실제_os_watcher`: sandbox 안에서 5초 deadline이 만료해 1 failed였습니다. 진단 관찰값과 실패 때도 join하는 cleanup을 추가하고, 합성 임시 프로젝트만 사용하는 정확한 단일 검사에 좁은 실행 권한을 적용한 뒤 1 passed/0 failed, 0.57초입니다. 실제 OS의 외부 파일 생성→FsChanged·Git index 변경→GitStatusChanged와 ExitDrain join/tracked 0을 확인했습니다. 제한 영향과 일치하는 관찰이며 FSEvents의 내부 실패 코드를 직접 측정한 것은 아닙니다. deadline 증가·반복 계측·fixture assertion 완화는 하지 않았습니다.
- [x] 당시 대상 `cargo clippy --lib --bin taide-native-app --test projects --test host -- -D warnings`: exit 0, 0.53초입니다. 최종 진단 코드까지 포함한 같은 대상 clippy는 exit 0, 0.36초이며 대상 rustfmt check·`git diff --check`도 exit 0입니다.

이미 통과한 이전 host/file/save/editor/controller 검사는 영향이 없는 동일 입력을 반복하지 않았습니다. 이 후속은 전체 N2/M8·213 view parity·픽셀/실제 OS dialog 완료가 아닙니다. 외부 파일 변경의 editor conflict/reload·dirty mirror 복원·정상 project close·더블클릭 focus/키보드·전체 explorer toolbar/context menu/DnD·enabled hooks/IDE·theme/locale/AX 등은 계속 구현해야 합니다.
