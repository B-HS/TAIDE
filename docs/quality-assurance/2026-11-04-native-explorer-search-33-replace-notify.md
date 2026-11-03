# explorer-search-33: 모두 바꾸기 OS 알림 연결

> 작성일 2026-11-04 · 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`(HEAD `062d7560`, 브랜치 `to_rust_native`)
> 근거 문서 `docs/handoff/native-completion/12-remaining-explorer-search.md` 3.1절
> 같은 날 묶음 `explorer-search-20·21`(`2026-11-04-native-explorer-search-20-21-row-menu.md`)과 함께 처리했습니다.

## 대상

| ID | 기능 | 결과 |
| --- | --- | --- |
| explorer-search-33 | 검색 교체의 모든 하위 동작(토글·입력·파일별 체크·확인 대화상자·질의 불일치 안내·요약 toast·건너뜀 보고·네이티브 알림·교체 후 재검색) | complete |
| settings-23 | OS 알림 발송 범주 | partial 유지. 검색 치환 생산자 연결은 이 묶음으로 완료. 남은 것은 범주 7종 묶음 App 검사(메인 통합) |

## 원본 동작

- `src/widgets/search-panel/search-panel-container.tsx` 124-162행 `handleReplaceAll`의 `onSuccess` 순서: 1) `toast.success(search.replaceDone{files,matches})`, 2) `notifyNative({category: 'searchReplace', title: resolveProjectNotificationTitle(...fallbackTitle: notification.searchReplaceDone), body: replaceSummary})`, 3) 건너뜀이 있으면 `toast.warning(search.replaceSkipped)`, 4) 같은 질의로 재검색(`recordHistory: false`).
- `onError`는 `toast.error`만 하고 알림을 보내지 않습니다.
- 알림 게이트는 `crates/taide-notification/src/service.rs` `decide_delivery`가 판단합니다(사용 안 함·범주 꺼짐·focus 조건).

## 구현

### explorer-search-33

`native/taide-native-app/src/domains/explorer_search.rs` `apply_search_replaced` 성공 분기에서 요약 toast 다음에 shell 소유 공통 진입점을 부릅니다.

```rust
self.toasts.success(summary.clone(), now);
self.notify_event(
    context,
    taide_model::notification::NotificationCategory::SearchReplace,
    Some(&project),
    "notification.searchReplaceDone",
    summary,
);
```

- 순서는 원본과 같습니다(success toast → 알림 → 건너뜀 warning → 재검색).
- 실패 분기(`Err(error)`)와 패널이 없는 조기 반환에는 넣지 않았습니다(원본 `onError`·unmount와 동일).
- 제목 규칙·200 코드 포인트 자르기·focus 입력은 `notify_event`(`domains/shell.rs` 1176행)가 처리하므로 이 파일은 진입점만 부릅니다(결정 1 A, 중복 소유 없음).
- 새 locale 키는 없습니다. `notification.searchReplaceDone`이 en·ko·ja 카탈로그 476행에 이미 있습니다.

### 검사 준비

- `application-notifications-tests.rs`의 `FakePlatform`·`sent`·`launch_with_platform` 가시성은 이미 `pub(in crate::application)`이라 변경하지 않았습니다(handoff 12 문서가 적은 `pub(super)`보다 실제 코드가 넓습니다).
- `application-search-panel-tests.rs`의 `launch_project`에서 상태 구성 부분을 `project_state(name, files, with_auxiliary) -> (Fixture, AppState, ProjectId, ShellSlotId, PathBuf)`로 분리하고 `launch_project`는 그 결과로 `launch`를 부르게 했습니다(동작 불변). 새 검사가 같은 상태를 `launch_with_platform`으로 띄우기 위해서입니다.

### 사양과 실제 코드가 달라 실제 코드를 따른 곳

- 가시성: 문서는 `pub(super) struct FakePlatform`을 올리라고 했지만 실제 코드는 `pub(in crate::application)`이었습니다. 변경 없이 호출할 수 있어 그대로 두었습니다.
- 행 우클릭 좌표: handoff 12 문서 3.3절 예시는 행 rect `center()`를 쓰지만, 이 fixture의 행 rect에는 펼침 chevron이 포함되어 중심이 이름 영역을 벗어날 수 있습니다. `ROW_CLICK_INSET = 8.0` 만큼 왼쪽 안쪽을 눌러 대상 행 선택을 확정했습니다(20·21 묶음 QA에 상세).

## 검증

공통 cargo 옵션: `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`.

| 명령 | 결과 |
| --- | --- |
| `cargo build --example native-lsp-mock <앱 Cargo.toml>` | exit 0, `target/debug/examples/native-lsp-mock` 생성 확인 |
| `cargo check --all-targets <앱 Cargo.toml>` | exit 0. 경고는 기존 것만: `editor-code-actions.rs` `is_applying`, `zen.rs` `mut`, `application-inlay-hints-tests.rs` `fixture_with_emmet_peek`, vendor `wry-preview` |
| `cargo test --lib -- --test-threads=2 search_panel_tests explorer_search_tests explorer::tests notifications_tests` | 24 통과, 0 실패(1209 filtered) |
| `cargo test --lib -- --exact application::search_panel_tests::실제_app_모두_바꾸기_성공은_프로젝트_제목과_요약으로_os_알림을_한_번_보낸다` | 1 통과 |
| `cargo test --lib -- --exact application::explorer_search_tests::실제_app_터미널에서_열기는_...` | 1 통과 |
| `cargo test --test explorer` | 22 통과, 0 실패 |
| `cargo fmt -- --check` | exit 0 |

### 새 검사

`application-search-panel-tests.rs` `실제_app_모두_바꾸기_성공은_프로젝트_제목과_요약으로_os_알림을_한_번_보낸다`(`#[cfg(unix)]`).

- 설정을 `notifications_enabled = true`, `notify_search_replace = true`, `notifications_only_when_unfocused = false`, `search_on_type = false`로 두고 `launch_with_platform`으로 실제 앱을 띄웁니다. 검사 프레임의 `focused` 기본값이 true이므로 focus 조건을 끕니다.
- 트리 대기 → `open_search_view` → `search(NEEDLE)` → `search.matchCount{3,2}` 대기 → 바꾸기 전환 클릭(`settle_pointer` 먼저) → 교체 입력 클릭·텍스트 입력 → `search.replaceAll` 클릭 → `common.confirm` 버튼 클릭.
- `platform.sent()`가 `[("alpha", search.replaceDone{files:2,matches:3})]` 하나인지 단언합니다. `a.txt` 1건·`b.txt` 2건이 모두 체크된 상태로 바뀌므로 files 2·matches 3입니다.
- 실패 선행 확인은 따로 하지 않았습니다(구현 전에 이 검사만 실행해 실패를 확인하지 않음).
- 실행하지 않은 것: 실제 OS 알림 표시(허락 P2, 배치 31 실기), 병합 게이트(메인이 묶음 뒤 실행).

## 실제 앱 경로

- 알림: `main.rs` → `eframe::create_native(.. NativeApplication::new(..))` → `background_tick` → `poll`(`application.rs` 775행) → `apply_explorer_search_reply`(`domains/explorer_search.rs` 650행) → `apply_search_replaced`(810행) → `notify_event`(`domains/shell.rs` 1176행) → `HostCommand::Shell(Command::Notify)` → `dispatch` → `notification_actions::notification_notify` → `PlatformServices::send_notification`(검사에서는 `FakePlatform`).
- 교체 요청: 바꾸기 확인 → `finish_explorer` … → host `ReplaceSearch` → `search_actions::search_replace` → `Reply::SearchReplaced`.

## 남은 범위

- 실제 OS 알림 센터 표시는 자동 검사 범위가 아닙니다(허락 P2, 배치 31 실기).
- settings-23의 범주 7종 묶음 App 검사는 메인 통합 몫으로 남습니다.
