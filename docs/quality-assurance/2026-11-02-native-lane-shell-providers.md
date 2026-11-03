# lane shell 묶음 1: provider 소비자

> 작성일: 2026-11-02 · lane: `shell`(worktree `TAIDE-lanes/shell`, 브랜치 `lane/shell`) · 사양: `docs/research/2026-10-28-lane-shell-spec.md` 2.0~2.6, 3.1 · 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안, 결정 1 A)

## 대상

| ID | 기능 | 결과 |
| --- | --- | --- |
| shell-93 | IPC 동기화 | 수신함·Sync 테마 재로드·앱 밖 닫힘 정리 완료. Git 이벤트(git-64)는 git lane이라 partial 유지 |
| shell-37 | 프로젝트 닫기 | complete(테스트 부채 4만 남김) |
| shell-90 | 외부 열기(agent CLI, `--wait`) | complete(실제 `taide` CLI 연결은 허락 항목) |
| shell-91 | OS 알림 | complete(실제 OS 알림 표시는 실기) |
| shell-92 | IDE 동기화 | save·진단 완료. diff 요청(git-49)은 git lane이라 partial 유지 |
| shell-96 | Emmet provider | complete(코드 변경 없음, 보조 창 검사 추가) |

같은 근거로 갱신한 대응표 행(결정 1 A): terminal-agent-57, 59, 60, 61, 62, 63, 70, 73, settings-23(agent·task·LSP·오류 범주).

## 원본 동작

원본 근거는 사양 2.1~2.6에 줄 번호까지 정리돼 있어 그대로 따랐습니다. 구현 중 원본에서 다시 확인한 것만 적습니다.

- 알림: `src/app/providers/native-notification-provider.tsx` 57-204, `src/shared/lib/native-notification-gate.ts` 68-123, `src/shared/lib/notification-text.ts` 19-79, `src/shared/lib/format-duration.ts` 21-31, `src/entities/notification/notify.ts` 37-50. LSP 설치 알림의 제목은 프로젝트 제목이 아니라 문구 키 그대로입니다. 명령 완료의 대상은 `owner?.tabTitle ?? cwd ?? terminal.title`이라 소유 탭이 있으면 빈 제목이어도 그 값을 쓰고, 빈 조각은 `joinNotificationBody`가 뺍니다.
- 외부 열기: `src/app/providers/agent-external-open-provider.tsx` 25-118, `src/entities/agent/external-open-target.ts`, `src/shared/lib/path-root.ts` 15-40, `crates/taide-agent/src/service.rs` 256-270(`parse_cli_payload`), `src-tauri/src/domain/agent/commands.rs` 326-345. 대상 비교의 root 길이는 JS 문자열 길이(UTF-16 단위)입니다.
- IDE 진단 push: `src/app/providers/ide-sync-provider.tsx` 137-158. 표식의 `path`는 `marker.resource.fsPath`(파일 경로)입니다.

## 구현

### 공통(사양 2.0)

- `provider-events.rs`(`crate::provider_events`): `Inbox`가 `PaintSink::publish`에서 `AgentStateChanged`, `TerminalCommandFinished`, `LspInstallProgress`(Done/Failed만), `IdeSaveRequested`, `ProjectClosed`(중복 제거)를 순서 대기열로, `AgentExternalOpen`은 플래그로 기록합니다. 상한 `MAX_PROVIDER_EVENTS`(256)를 넘으면 닫힘이 아닌 가장 오래된 것을 버리고 `log::warn`합니다. 단일 인스턴스 중계용 "메인 창 앞으로" 플래그도 둡니다.
- `domains/shell.rs` `consume_provider_events(context, now)`: `background_tick`의 `self.poll(context)` 바로 뒤 한 줄에서 부릅니다(사양 2.0.1). 종료 중에는 비우기만 합니다.
- `application.rs`(공용, 추가만): `PaintSink.providers`, `Overrides { secret_store, platform }`와 `new_with_overrides`(`new_with_secret_store`는 위임), shell `State` 리터럴의 새 필드 3개, `background_tick` 호출 한 줄.

### shell-93

- `presentation_refresh::Changes::record`: `SyncStateChanged`에서 `revision`과 `themes`를 올리고 `settings`는 두지 않습니다(사양 2.1).
- 앱 밖 닫힘: 수신함의 `ProjectClosed`가 `services.state.projects`에 없는 프로젝트면 `forget_closed_project`를 부릅니다. `Reply::ProjectClosed`도 같은 함수를 부르며 반복 호출해도 결과가 같습니다.

### shell-37

- `forget_closed_project`: `release_project_documents` 뒤에 `explorer_search_state`의 `trees`, `explorers`, `tree_loading`, 그 프로젝트를 가리키는 `pending_entry_delete`(삭제 진행 중이 아닐 때)를 지우고, `peek_models.reassign_project`로 닫힌 프로젝트 소유 peek 문서를 `root_guard::resolve_owning_project`로 다시 정합니다(없으면 `None`).
- 전수 확인(`HashMap<ProjectId,_>`/`HashSet<ProjectId>` 필드): 위 3개 외에 프로젝트 키 필드는 없습니다. `symbol_sidebars`는 slot 키이고 `background_tick`의 `reconcile`이 slot tree에서 사라진 항목을 이미 지우므로 건드리지 않았습니다.
- `persistence::closing_mirror_job`: in-flight mirror가 있어도 현재 revision이 더 크면 job을 줍니다. `close_project`가 `mirror_job` 대신 씁니다. 늦게 온 mirror 응답은 `ProjectClosed`의 `settled`로 entry가 사라져 `mirror_finished`가 무시합니다(App 검사에서 오류 없음).
- 공용 상태 쓰기: explorer-search 소유 필드를 shell 영역에서 직접 지웁니다(사양 1.3 표). explorer-search D6 병합 때 "그 프로젝트의 mount 전부 제거"로 맞춰야 합니다.

### shell-90

- `external-open.rs`(`crate::external_open`): `request_from`(원본 `parse_cli_payload`), `queue`(marker 등록, `authorize_cli_opened_path`, 대기열), `is_within_root`/`resolve_target`(원본 정규화·가장 긴 root·같은 길이 첫 항목·활성 → 첫 프로젝트), `WaitMarkers`, `project_has_file_tab`, `release_closed_tab`, host drain `process`(프로젝트 열기 → 대상 선택 → 활성화 → `open_file_tab(preview = marker 없음)` → 성공 + marker면 host 안에서 등록), `Outcome`.
- 실행 인자: `bootstrap::Launch::parse`가 `--data-dir <dir>` 뒤의 인자를 CLI payload로 읽습니다. `LaunchConfig::parse`는 이것으로 위임합니다. 기존 오류(인자 없음, data dir 없음, root, 상대 경로, `..`, `--data-dir`가 첫 인자가 아님)는 그대로이고, "unexpected native launch argument" 오류는 사양대로 없어졌습니다(`tests/host.rs`의 해당 사례를 "`--data-dir`가 첫 인자가 아님"으로 바꿈).
- `single-instance.rs`(`#[cfg(unix)]`): 소켓 `temp_dir()/taide-native-<FNV-1a 16자리>.sock`. `acquire`가 연결 성공 시 인자를 JSON 한 줄로 보내고 ack(`RELAY_ACK_TIMEOUT` 5초)를 받으면 `Relayed`, 거부된 연결이면 남은 소켓 파일(소켓일 때만)을 지우고 bind + 0600, bind 실패면 `Unavailable`(단독 실행 + `log::warn`). `serve`는 transient task의 tokio `UnixListener`로 연결마다 `MAX_RELAY_BYTES`(64 KiB)·`RELAY_READ_TIMEOUT`(2초) 한 줄을 읽고, 요청이 있으면 `queue` + `AgentExternalOpen` 발행, 요청 유무와 관계없이 메인 창 앞으로 플래그와 ack를 보냅니다. 소켓 파일은 `SocketFile` drop(첫 인스턴스 `main` 종료)에서 지웁니다.
- `main.rs`: parse 직후·restore 전에 claim, `Relayed`면 창 없이 `Ok(())`. 앱 생성 뒤 `with_relay(listener)`·`with_external_open(request)`(첫 tick drain 플래그는 항상 세움, 원본 mount drain).
- 소비: 플래그가 있으면 `Command::ProcessExternalOpens` 제출(실패면 플래그 유지), 메인 창 앞으로는 ROOT viewport에 `Visible(true)`·`Focus`. 응답은 `OpenProjectFirst` → `app.openProjectFirst` info, `Failed` → IPC 오류 toast, `WaitRegistered` → `app.externalEditorTabHint` info.
- marker 해제 지점: host `CloseTab`(editor 영역 dispatch에 호출 한 줄), IDE `LayoutActions::close_tab`(닫은 뒤 layout으로 판정). 원본의 IDE 닫기 경로 미해제는 결함이라 재현하지 않았습니다(사양 2.4).
- `WaitMarkers` 공유: `LayoutActions::new`가 만들고 `application_ports::Ports.wait_markers`가 같은 `Arc`를 `HostIntegrations.wait_markers`로 넘깁니다. 다른 `HostIntegrations` 생성자(검사·원격)는 빈 `Arc::default()`입니다.

### shell-91

- `native-notifications.rs`(`crate::native_notifications`): 상수 3개, `Tracker`(프로젝트별 working 지도, 첫 전달 안내 플래그), `evaluate`, `agent_category`, `body_fragment`, `join_body`, `format_duration`, `agent_body`, `task_body`, `truncate`(Unicode scalar 기준), `project_title`(`resolve_project_display` label → 이름 → 대체), `terminal_tab_title`(메인 + 보조 창 트리), `session_owner`(세션 프로젝트 순서), `Request`.
- 다른 lane용 진입점 `NativeApplication::notify_event(context, category, project, fallback_title_key, body)`: 제목 규칙, 200 코드 포인트 자르기, `is_any_window_focused`, `Command::Notify` 제출을 한 곳에서 합니다. git(git lane)·검색 치환(explorer-search lane)·settings가 이것을 부릅니다.
- host `Notify`: `run_blocking_result`에서 `notification_actions::notification_notify(state, services.platform, ..)`. 응답 `NotificationSent(Ok(Delivered))`이고 아직 안내하지 않았으면 `notification.enableHint` info 1회, 오류는 `log::warn`만 합니다.

### shell-92

- `ide-sync.rs`(`crate::ide_sync`): 대기 저장 요청, 진단 debounce(`IDE_DIAGNOSTICS_PUSH_DEBOUNCE` 300 ms, 서명 = 진단 store `revision` Arc 동일성 + 포커스 slot 프로젝트 + `ide.is_running()`), `has_file_tab`, `diagnostics`(store → `IdeDiagnostic`).
- save 요청: 그 프로젝트 layout 모든 트리에 같은 경로 문자열의 파일 탭이 없으면 `true`, 문서 없음 `false`, clean `true`, read-only `false`, 그 밖은 `store.save_snapshot` → `HostCommand::Editor(Save)`, 제출 성공이면 `Reply::Saved` 끝(editor 영역에 호출 한 줄 `finish_ide_saves`)에서 결과로 응답합니다. 보조 창 탭도 같은 store라 저장합니다.
- 진단 push: 마감이 지나고 포커스 프로젝트와 실행 상태가 있으면 `PublishIdeDiagnostics`. 마감 대기 중에는 `request_repaint_after`.

### shell-96

- 코드 변경 없음. 보조 창 편집기에서도 완성 공급자가 Emmet을 공급하고 Tab으로 수락됩니다.

### 사양과 실제 코드가 달라 실제 코드를 따른 곳

1. 진단 `path`: 사양은 `diagnostics::resource`(URI)였지만 원본은 `marker.resource.fsPath`, native IDE 도구 `getDiagnostics`도 URI를 경로로 바꿔 `IdeDiagnostic.path`와 비교합니다(`ide-tools.rs` `diagnostics`). 파일 문서의 경로 문자열을 씁니다. untitled·AppFile 문서는 파일 경로가 없어 넣지 않습니다.
2. `Reply::Saved` 처리는 wave 0 이동으로 `application.rs`가 아니라 `domains/editor.rs`에 있습니다. 끝에 `finish_ide_saves` 호출 한 줄만 추가했습니다.
3. App 검사 모듈은 `application.rs` 목록이 아니라 `domains/shell.rs` 끝에 `#[path = "../application-*-tests.rs"]`로 선언했습니다(공용 파일 충돌 회피). shell-37 검사도 `application-project-rail-tests.rs` 대신 `application-ipc-sync-tests.rs`에 두었습니다.
4. 탐색기 삭제 추종: native 삭제는 탭을 닫지 않고 missing draft로 유지합니다(`explorer_source_missing.rs`). 닫는 탭이 없어 marker 해제 훅을 둘 곳이 없고, 남은 marker는 종료 때 `cleanup_all_wait_markers`가 지웁니다.
5. `new_with_secret_store` 사용처가 한 곳뿐이라 위임 함수로 남겼습니다.

### 원본과 다르게 둔 점(근거)

- IDE save는 일반 저장 명령을 그대로 써서 format-on-save가 켜져 있으면 포맷합니다(기본 꺼짐). 원본 `useSaveFile` 직접 저장과 다릅니다. 포맷을 건너뛰려면 editor 영역 내부 상태(`resumed_save_epochs`)를 shell에서 써야 해서 하지 않았습니다.
- IDE save의 read-only 분기: 원본 순서(dirty 확인 → read-only)대로 두었습니다. native는 read-only 문서를 편집할 수 없어 실제로는 clean `true`가 먼저 걸립니다.
- 원본 보조 창 탭 save `false`(realm 제약)와 IDE 닫기 경로 marker 미해제는 결함으로 보고 재현하지 않았습니다(사양 2.4, 2.6).

## 검증

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/shell`, manifest는 `native/taide-native-app/Cargo.toml`입니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets` | 통과. 남은 경고 3건은 이 묶음 밖 기존 경고(`editor-code-actions.rs` `is_applying`, `zen.rs` `mut`, `application-inlay-hints-tests.rs` `fixture_with_emmet_peek`) |
| `cargo fmt -- --check` | 통과 |
| `cargo test --lib -- provider_events native_notifications ide_sync external_open single_instance presentation_refresh peek_models domains::shell:: 보조_창_html_편집기도_emmet project_rail_tests exit_tests ide_tools ide_server --test-threads=2` | 54 passed |
| `cargo test --lib -- ide_sync::tests --test-threads=2` | 2 passed(보조 창 트리 탭 찾기 추가 뒤) |
| `cargo test --test host --test persistence -- 실행_설정은 프로젝트_닫기_mirror는 --test-threads=2` | 각 1 passed |
| `cargo test --lib -- project_rail_tests exit_tests ide_tools ide_server ide_selection_tests file_drop_tests application_ports bootstrap --test-threads=2`(회귀) | 26 passed |

새 검사

- crate 단위: `provider_events` 2(기록·비우기, 상한), `native_notifications` 7(평가 경계, 프로젝트별 지도, 본문 4종, 경과 4경계, 코드 포인트 자르기, 명령 본문, 제목·보조 창 탭 소유), `ide_sync` 2(debounce, 보조 창 트리 탭), `presentation_refresh` 1(Sync), `peek_models` 1(소유자 재해석), `external_open` 4(대상, root 밖 대체, marker, 실행 인자), `single_instance` 2(중계·창 앞으로·파일 삭제·권한, 남은 소켓 교체·상한 거부), `tests/persistence.rs` 1(closing mirror)
- 실제 App(`main.rs` → `NativeApplication` → `background_tick` → `consume_provider_events`/host 경로)
    - `ipc_sync_tests` 4: Sync 뒤 같은 테마 파일 재로드(이벤트 전에는 옛 색 유지), 앱 밖 `project_close` 뒤 문서·drafts·tree·explorer 정리, rail 닫기 뒤 재열기에서 새 tree와 빈 선택, in-flight mirror보다 새 본문이 닫기 전에 mirror됨
    - `notifications_tests` 2: 명령 완료(포커스 게이트·10초 임계·범주 스위치·제목·본문), LSP Done(Downloading 무시), 첫 전달 안내 1회 / agent working → idle 11초 1건(제목·탭 꼬리)
    - `ide_sync_tests` 2: dirty 저장(디스크·clean), clean(수정 시각 불변), 인코딩 손실(read-only, clean이라 `true`, 디스크 불변), 열린 탭 없음 / 진단은 IDE 실행 전 push 없음, 실행 뒤 300 ms 이후 store와 같은 항목
    - `external_open_tests` 1: 첫 실행 요청(프로젝트 없음 → 안내, marker 삭제), 폴더 → 프로젝트 열림·marker 삭제, 없는 파일 → 오류·marker 삭제, 프로젝트 파일 + marker → preview 아닌 탭·안내·marker 유지, 같은 경로 탭 두 개 중 하나 닫기 → 유지, 둘 다 → 삭제, root 밖 파일 → 활성 프로젝트 preview 탭, 메인 창 앞으로 viewport 명령
    - `auxiliary_window_tests` 1: 보조 창 html 편집기 Emmet 공급·Tab 수락·undo 1회
- 실패 선행 확인: 검사는 구현과 함께 썼고, 구현 전 상태에서 실패하는지는 따로 실행하지 않았습니다. 관찰한 실패는 검사 구성 문제 2건(비활성 탭 문서 미로드, 프레임 밖 포커스 입력)과 보조 창 html이 미리보기로 열리는 것 1건이었고 모두 검사 구성을 고쳤습니다. agent 검사는 프레임 밖에서 시각을 넘겨 부르므로 egui 입력의 포커스 값이 프레임 밖 기본값이 되어, 그 검사에서만 "포커스 없을 때만" 설정을 끄고 포커스 게이트는 명령 완료 검사가 확인합니다.
- 실행하지 않은 것: 필터 없는 앱 전체 테스트(메인 병합 때), `--features inspection` 검사(UI crate와 Emmet 전용 모듈을 바꾸지 않음), 동결 remote-web Wasm check(UI crate·vendored eframe 변경 없음, 사양 3.1).

## 남은 범위

- shell-93: Git 이벤트 소비(git-64, git lane) 뒤 묶음 3에서 최종 판정
- shell-92: IDE diff 요청·close 요청(git-49, git lane). `ide_sync`에 붙이면 됩니다.
- shell-37: 테스트 부채 4(다른 프로젝트 탭이 보는 문서의 소유자 재해석)는 프로젝트 사이 탭 이동·중첩 프로젝트 열기 경로가 생길 때. explorer-search D6 병합 때 `forget_closed_project`의 explorer 키를 mount 키로 맞춤
- shell-90: 실제 `taide` CLI → native 연결은 shell-15 실제 사용자 데이터 경로 허락 뒤 실기
- shell-91: 실제 OS 알림 표시·osascript 경로·macOS 권한은 실기(배치 31), Windows/Linux 전송은 릴리스 대상 밖. git remote·검색 치환 범주는 각 lane이 `notify_event`를 부른 뒤 갱신
- 다른 lane 알림: settings 사양 2.5의 `Overrides`는 필드만 추가하면 됩니다(`application.rs` `Overrides`). settings `Changes`는 알림 이벤트를 다시 기록하지 않아야 합니다(결정 1 A)

## 수정(shell lane 리뷰 반영)

차단 항목은 없었습니다. 과대 판정 3건(이 묶음)과 비차단 항목 일부를 실제 코드로 확인해 반영했습니다. shell-3 과대 판정은 묶음 2 문서(`2026-11-02-native-lane-shell-windows-layers.md`) 수정 절에 적었습니다.

### 과대 판정 정정

1. shell-92 / terminal-agent-70 대응표 문구: 리뷰가 맞습니다. `resolve_ide_save`는 탭 없음 `true` → 문서 없음 `false` → clean `true` → dirty이면서 read-only `false` → 저장 순서라, 인코딩 손실 read-only 문서는 clean이므로 `true`입니다(App 검사가 그렇게 단언). 두 행을 이 순서대로 고쳤고, 문서 없음 `false`와 dirty read-only `false`(native 도달 불가)는 코드 확인뿐이라고 적었습니다. `resolve_ide_save`의 영어 문서 주석도 같은 내용으로 고쳤습니다. 동작 변경은 없습니다.
2. shell-90 문구: IDE `close_tab` 경로의 marker 해제 App 검사를 추가했고(아래 검증), 행 비고에 탐색기 삭제 추종 경로는 native가 탭을 닫지 않아 훅이 없고 남은 marker는 종료 때 지워진다는 점(위 "사양과 실제 코드가 달라" 4)을 넣었습니다.
3. shell-92 보조 창 저장 근거: 보조 창에만 열린 dirty 탭을 IDE save 요청으로 저장하는 실제 App 검사를 추가했습니다. 행 비고는 이 검사를 근거로 고쳤습니다.

### 비차단 반영

- 중복 helper: `external_open::project_has_file_tab`과 `ide_sync::has_file_tab`(같은 로직)을 사양 2.4대로 `tabs::project_has_file_tab` 하나로 합쳤습니다. `external_open::release_closed_tab`·`resolve_ide_save`가 이것을 부르고, 보조 창 트리·정확한 경로 문자열 단위 검사는 `ide_sync::tests`에 그대로 두고 호출만 바꿨습니다.
- 테스트 공백 (a): 진단 App 검사가 구현 함수 결과와 같은지만 보던 것에 더해, 진단 store의 원본 LSP 진단과 항목별로 경로(파일 경로 문자열)·범위(LSP 0 기준 그대로, 한 줄 JSON이라 0행)·message·source를 직접 단언합니다. 포커스 프로젝트가 바뀌면 debounce를 다시 세고 새 프로젝트로 push하는 단위 검사(`ide_진단_push는_포커스_프로젝트가_바뀌면_새_프로젝트로_다시_센다`)를 추가했습니다. 실제 App에서 포커스 slot을 바꿔 재 push하는 검사는 없습니다(두 프로젝트 slot 전환 fixture가 필요해 남김).
- 테스트 공백 (b): `실제_app은_보조_창에만_열린_dirty_탭도_ide_저장요청으로_저장한다`(보조 창 fixture `auxiliary_window_tests::single_window` 재사용, 메인 트리에 탭 없음 확인 → 편집 → IDE save `true` → 디스크 = 편집 본문 → clean). fixture 재사용을 위해 `SingleWindow`·`single_window`와 필드 4개의 가시성만 `pub(super)`로 바꿨습니다.
- 테스트 공백 (c): `실제_app은_외부_열기_요청을_프로젝트_탭_wait_marker와_안내로_처리한다`에 IDE 경로를 추가했습니다. 같은 파일을 `--wait`로 다시 열어 marker를 만든 뒤 `ide_tools::dispatch("close_tab", tab_name)`(실제 App의 `application_ports.ide.actions`)로 닫으면 탭이 사라지고 marker가 해제됩니다.

### 반영하지 않은 비차단 항목(사유)

- 단일 인스턴스 competing start(첫 인스턴스 부팅이 `RELAY_ACK_TIMEOUT`보다 길면 두 번째가 단독 실행): 이전 동작 수준이고, 고치려면 accept loop를 앱 생성 전에 시작하도록 `main.rs` 순서를 바꿔야 해 후속으로 둡니다.
- `Command::Notify`의 osascript 무기한 대기: 사양이 지시한 host 직렬 경로입니다. timeout 또는 별도 task 분리를 후속으로 권합니다(`bootstrap.rs` `send_notification`은 `status()`만 호출).
- `resolve_ide_save`의 `submit` 실패(같은 문서 저장·형식화 저장 진행 중) 때 `saved:false`, `Reply::Saved`가 오지 않는 실패 경로에 남는 `ide_sync.saves` 항목: 정리하려면 editor 영역(`domains/editor.rs`)의 형식화 저장 실패 경로에 훅이 필요해 lane 경계를 넘습니다. 영향은 드문 경로의 `saved:false`와 다음 저장 때 만료 request id에 대한 `log::warn` 1줄입니다. 후속(editor lane 합의)으로 둡니다.
- (d) shell-98 검사가 보조 창 탭 메뉴 대신 `application.report()`를 직접 부르는 점: 작업자 보고대로 유지합니다.
- shell-37의 explorer-search 소유 필드 직접 정리, `ToastLayers` 역참조의 다른 lane 기존 toast 검사 영향: 병합 때 메인 확인 항목입니다(위 남은 범위·묶음 2 문서).
- stderr 로거의 경로 포함 메시지, `FALLBACK_LOCALE_ID = "en"` 중복: 앱 crate가 `taide-locale`에 직접 의존하지 않아 Cargo 변경 없이 `BUILTIN_EN_ID`를 쓸 수 없습니다. 배포 전 로그 문구 확인을 권합니다.
- Linux 공유 `/tmp` 소켓 이름, agent 완료 시각의 틱 기준 오차: 릴리스 대상 밖·한 프레임 수준이라 기록만 합니다.

### 검증(수정)

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/shell`, manifest는 `native/taide-native-app/Cargo.toml`입니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets` | 통과. 경고는 기존 3건(`is_applying`, `zen.rs` `mut`, `fixture_with_emmet_peek`)과 vendored wry 경고 |
| `cargo fmt -- --check` | 통과 |
| `cargo test --lib -- ide_sync external_open --test-threads=2` | 11 passed |
| 변이 확인 1: `project_has_file_tab`이 메인 트리만 보게 임시 변경 뒤 `cargo test --lib -- 보조_창에만_열린_dirty_탭도 --test-threads=2` | 1 failed(디스크 단언 `"synthetic\n"` ≠ `"edited synthetic\n"`), 원복 |
| 변이 확인 2: IDE `close_tab`의 `release_closed_tab` 호출을 임시로 뺀 뒤 `cargo test --lib -- external_open_tests --test-threads=2` | 1 failed(`ide close released marker` 단계 10초 시한), 원복 |
| 원복 뒤 `cargo test --lib -- ide_sync external_open auxiliary_window_tests ide_tools ide_server view_state_tests --test-threads=2` | 47 passed |

- UI crate·vendored eframe은 바꾸지 않아 UI `--features native-host,inspection` 검사와 remote-web Wasm check는 다시 돌리지 않았습니다.
