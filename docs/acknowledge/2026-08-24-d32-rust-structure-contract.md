# d-32 — Rust 구조 일괄(lib.rs 도메인 이관 + layout 커맨드 골격 공통화) 계약 (2026-08-24)

> 정본: `2026-08-21-batch-consolidation-decision.md` §2 d-32 / 감사 C7(:203 lib.rs)·
> R1#11(:139 layout read-clone-mutate-writeback 18) / d-28 계약 #10 deferred(T1-H 락 축 사유).
> 운영: 사용자 지시(2026-08-24 /goal)로 **커밋·푸시 없이** d-32~d-35 연속 자율 진행 —
> 결정 지점은 추천안 자체 채택 후 이 계약에 기록(멈춤 없음), bindings 표면·행동 변경만 이월.

## 0. 착수 전 메인 실사 (2026-08-24)

- `lib.rs` **1453줄**(감사 시점 778 — d-25 워처 후절화·문서주석으로 성장). 도메인 로직:
  `build_app_menu`/`handle_menu_event`(메뉴)·`flush_dirty_layouts`+`LAYOUT_FLUSH_INTERVAL_MS`
  (레이아웃 플러셔)·`handle_close_requested`/`handle_auxiliary_close_requested`/
  `restore_auxiliary_windows`(창 닫기·hot-exit·보조창 복원)·`projects_pending_watcher_restore`/
  `restore_project_watchers`(워처 재부착, d-25 산출)·`restore_state`(프로젝트 복원)·
  `poll_agents`/`queue_cold_start_external_open`(에이전트). 잔류 성격: `specta_builder`·
  `run()`·DI provider 4종(`project_capabilities`·`settings_toggle_observers`·
  `system_usage_label_providers`·`pty_spawn_env_provider`)·상수. tests(1123-1453)는 바인딩
  생성·디스패치/이벤트 패리티(잔류 성격)와 워처/복원/capability(이관 대상 동행) 혼재.
- `domain/layout/commands.rs` **370줄** — 커맨드 17(+사설 1). 골격 동형 **13개**
  (`activate_tab`·`move_tab`·`split`·`resize`·`focus_pane`·`pin_tab`·`set_preview`·
  `reopen_closed`·`set_view_state`·`set_dirty`·`set_terminal_session`·`open_untitled`·
  `set_shell_view`): `begin_mutation → layouts.read().clone() → locate(3전략: with_tab/
  with_pane/직접 project_id) → get_layout_mut → 뮤테이트 → finish_mutation → layouts.write()`.
  예외 4: `layout_get`(읽기 전용)·`open_tab`/`close_tab`(service `*_and_finish` 기 위임)·
  `convert_untitled`(**가드 하** `projects.read().clone()` 선행 — 순서 변경 금지)·
  `move_tab_to_window`(뮤테이트 구간에 `open_auxiliary_window(...).await` + `WindowStore`
  State + 빈 보조창 정리 후처리).

## 1. 범위·결정 (추천안 자체 채택 — /goal 운영)

### a. lib.rs 도메인 이관

- 이관 맵(구현이 실사로 세부 확정하되 아래 원칙 고정): 메뉴 2함수+`MENU_ID_QUIT` →
  `domain/window`(메뉴는 창 소관 — 신설 모듈 금지) / 플러셔 → `domain/layout` / 닫기·보조창
  복원 3함수 → `domain/window` / 워처 재부착 2함수·`restore_state` → `domain/project` /
  에이전트 2함수 → `domain/agent`. 각 함수 본문·doc 주석 **원문 그대로 이동**(pub(crate)
  승격만), `run()` 은 새 경로 호출로 배선 교체 — **호출 시점·순서·스폰 구조 불변**.
- DI provider 4종·`specta_builder`·`run()`·`BINDINGS_PATH`·바인딩/패리티 테스트는 **잔류**
  (부트스트랩 조립 — 추천안 채택). 이관 함수의 전용 테스트(워처 3·복원 1·capability 2 등
  실사 확정)는 대상 모듈로 동행 이동.
- 커맨드 등록·이벤트·bindings 표면 무변경(cargo test 바인딩 생성 결과 diff 0).

### b. layout 커맨드 골격 공통화

- 동형 13개를 헬퍼 1개로 공통화: locate 전략(enum 또는 클로저 — `WithTab(TabId)`/
  `WithPane(PaneId)`/`Direct(ProjectId)`)과 뮤테이트 클로저(`FnOnce(&mut ProjectLayout) ->
  AppResult<()>` 계열)를 받아 골격 실행. **락 의미 절대 불변**: guard 획득→해제 구간,
  `read().clone()` 시점, `finish_mutation` 후 `write()` 순서가 현행과 동일해야 하며 헬퍼
  도입으로 await 지점이 늘어나서는 안 된다(뮤테이터는 동기 클로저 한정).
- 예외 4는 **수제 유지**(추천안 채택): `convert_untitled` 의 가드-하 projects 선행 읽기를
  헬퍼에 욱여넣지 않는다(pre-hook 일반화는 과추상화·락 판정 위험) / `move_tab_to_window` 의
  비동기 뮤테이터도 동일 / `open_tab`·`close_tab` 은 service 위임 현행 유지.
- 위치: 헬퍼는 `commands.rs` 사설(소비처가 이 파일뿐 — shared 승격 금지).

### 범위 외

- T1-H 본체(락 입도 변경·spawn_blocking IO 분리) — d-35. 커맨드 추가/제거·시그니처 변경.
- lib.rs `run()` 내부 로직 재작성(배선 교체만). dispatch.rs·capability 정책 변경.

## 2. 실행·검증

- 구현: Rust 한 시점 한 에이전트 — R1(lib.rs 이관) → R2(layout 골격) **순차**(sonnet+xhigh),
  각자 계약 §3 기록. cargo fmt 필수.
- 검토 3렌즈(opus+xhigh): ① 동시성·락(guard 수명·락 구간 바이트 의미 대조·await 지점 증감
  0·이관 함수 호출 시점/스폰 구조 불변 — **최우선**) ② 동등성(멀티셋 대조·본문 무변경·테스트
  동행·bindings diff 0) ③ 컨벤션·경계(모듈 배치·pub 표면 최소·doc 주석 이동 정합). major
  적대적(opus+high) → 수정 → 메인 2차(cargo test 전체+verify+vite). **커밋 없음**(/goal).

---

## 3. 구현 완료 기록 (2026-08-24)

### 3-R1. lib.rs 도메인 이관

### R1 — lib.rs 도메인 이관 결과 (d-32, 2026-08-24)

### 이관 맵

| 함수/상수 | 이전 위치(lib.rs) | 새 위치 | 가시성 |
|---|---|---|---|
| `MENU_ID_QUIT` | 상수(구 :44) | `domain/window/commands.rs` | `pub(crate)` |
| `build_app_menu` | 함수(구 :46-83) | `domain/window/commands.rs` | `pub(crate)` |
| `handle_auxiliary_close_requested` | 함수(구 :462-470) | `domain/window/commands.rs` | `fn`(모듈 내부 전용 — lib.rs 가 직접 호출하지 않음) |
| `handle_close_requested` | 함수(구 :482-515) | `domain/window/commands.rs` | `pub(crate)` |
| `handle_menu_event` | 함수(구 :525-532) | `domain/window/commands.rs` | `pub(crate)` |
| `restore_auxiliary_windows` | 함수(구 :541-576) | `domain/window/commands.rs` | `pub(crate)` |
| `LAYOUT_FLUSH_INTERVAL_MS` | 상수(구 :421) | `domain/layout/service.rs`(`save_layout` 직후) | `pub(crate)` |
| `flush_dirty_layouts` | 함수(구 :425-450) | `domain/layout/service.rs` | `pub(crate)` |
| `restore_state` | 함수(구 :827-848) | `domain/project/commands.rs` | `pub(crate)` |
| `projects_pending_watcher_restore` | 함수(구 :595-622) | `domain/project/commands.rs` | `pub(crate)` |
| `restore_project_watchers` | 함수(구 :711-796) | `domain/project/commands.rs` | `pub(crate)` |
| `poll_agents` | async 함수(구 :798-825) | `domain/agent/commands.rs` | `pub(crate) async` |
| `queue_cold_start_external_open` | 함수(구 :853-864) | `domain/agent/commands.rs` | `pub(crate)` |
| `stub_project`/`stub_session` + 워처복원 테스트 4건 | lib.rs `mod tests`(구 :1341-1452) | `domain/project/commands.rs` `mod restore_tests`(동행) | 테스트 전용(비공개) |

**잔류(변경 없음)**: `specta_builder`·`run()`·`BINDINGS_PATH`·`RAW_CHANNEL_COMMANDS`·DI provider 4종(`project_capabilities`·`settings_toggle_observers`·`system_usage_label_providers`·`pty_spawn_env_provider`)·바인딩 생성/패리티 테스트 7건(`typescript_바인딩을_생성한다` 외 6건). lib.rs 1453→850줄.

### 판단 근거

1. **window 함수 → `commands.rs`(service.rs 아님)**: window 도메인은 `commands.rs → service.rs` 단방향 참조만 갖고 있었다(모든 도메인 공통 관행 — `service.rs` 가 `commands.rs` 를 import 하는 선례는 코드베이스 전체에 0건). 이관 대상 6개 함수가 `WindowStore`(commands.rs 정의)·`open_auxiliary_window`(commands.rs 정의)에 의존하므로, `service.rs` 에 두면 `service → commands` 역방향 의존을 신설하게 된다. 기존 단방향 관행을 지키기 위해 `commands.rs` 로 배치했다.
2. **layout 함수 → `service.rs`(commands.rs 아님)**: layout 도메인은 이미 `finish_mutation`·`open_tab_and_finish`·`close_tab_and_finish` 등 `AppState`/`AppHandle` 을 직접 받는 오케스트레이션을 `service.rs` 에 두는 도메인 고유 관행을 갖고 있었다(**§4 L3-7 정정**: window/service.rs 도 AppHandle 오케스트레이션 패턴을 갖는다 — layout service 배치의 실근거는 flush 가 같은 파일의 save_layout 만 호출하는 순수 AppState 함수라는 점이며, window 이관분은 WindowStore·open_auxiliary_window 가 commands.rs 정의라 service 배치 시 역방향 의존이 신설되는 차이). `flush_dirty_layouts` 는 `save_layout`(같은 파일)만 호출하는 순수 AppState 기반 함수라 이 관행에 정확히 부합하며 신규 의존도 없다.
3. **agent 함수 → `commands.rs`(service.rs 아님)**: agent 도메인의 `service.rs` 는 `AppHandle`/tauri 의존이 전무한 순수 로직 전용 파일임을 확인했다(grep 0건). 반면 `commands.rs` 에는 이미 `AgentStore`/`AgentHooksStore`(정의처)·`detect_agents_for_pids_blocking`·`build_detected_agents`·`cleanup_all_wait_markers` 등 Store 의존 비-IPC 헬퍼가 존재해, `poll_agents`/`queue_cold_start_external_open` 이 그 관행에 정확히 부합한다.
4. **project 함수 → `commands.rs`(유일한 후보)**: `project::service.rs` 는 `paths`/`session`/`projects` 파라미터만 받는 완전 순수 함수 전용(AppHandle/AppState 참조 0건)이라 `restore_project_watchers`(AppHandle 스폰·이벤트 emit)를 넣을 수 없다. `commands.rs` 는 이미 `AppHandle`·`tauri_specta::Event`·이벤트 emit 을 쓰는 유일한 후보였다.
5. **신규 교차도메인 의존 4건과 화이트리스트 등재(핵심 판단)**: project 함수 3개를 `project::commands.rs` 로 옮기자 `tests/domain_boundaries.rs` 의 `도메인_간_참조는_types와_capability_확장점과_화이트리스트만_허용된다` 테스트가 실패했다 — `project/commands.rs → file::capability`·`git::watch`·`layout::service`·`settings::service` 4개 신규 실행경로 참조가 미등재 상태였기 때문(이 4개 호출은 이전에는 lib.rs 조립 계층에 있어 도메인 경계 스캔 대상 밖이었다). 이관 원칙상 project 도메인 배치가 계약에 고정되어 있고, 동일 함수 집합을 다른 파일로 쪼개는 것은 §1-a 원칙(`restore_state` 와 나머지 2함수를 분리하지 말라는 취지의 상호 doc 참조 다수)에 반하므로, 코드 자체를 쪼개는 대신 `tests/domain_boundaries.rs` 가 이미 제공하는 공식 예외 경로(`ALLOWED_CROSS_DOMAIN_EDGES` — 불가피한 교차 참조를 사유와 함께 등재)를 사용했다. 정확히 동일한 성격의 선례가 이미 존재한다: `sync/*` 는 "aggregation domain"으로서 settings·theme·locale 을 직접 호출하도록 승인되어 있다(파일 상단 doc 주석). project 의 부팅 복원 3함수도 매 도메인의 영속 상태·워처를 앱 시작 시 일괄 재구성하는 동일한 "1회성 집계" 성격이므로 같은 근거로 4건을 등재하고, 알파벳 순서 위치(**§4 L3-7 정정**: 실제 삽입 위치는 remote/login_page.rs 항목 앞 — sync 앞 아님)에 정확히 그 근거를 서술한 doc 주석을 추가했다. **신규 의존 방향**: `project::commands.rs → file::capability`, `project::commands.rs → git::watch`, `project::commands.rs → layout::service`, `project::commands.rs → settings::service`(모두 project 가 소비하는 단방향, project 를 역참조하는 기존 edge — `layout/git/file::capability → project::types`·`project::capability` — 와는 다른 하위 모듈이라 순환이 아님). 이 4개는 이전부터 lib.rs 조립 계층이 이미 갖고 있던 의존(정확히 같은 함수 호출)이 그대로 project 도메인으로 이동한 것일 뿐 논리적으로 새로 생긴 결합은 아니다.
6. **본문/doc 이동 방식**: 함수 본문·doc 주석은 원문 그대로 유지하되, ①이동 후 같은 파일에 남게 된 자기참조(`domain::window::service::X`, `domain::window::commands::X`, `domain::project::service::X`, `domain::agent::commands::X`, `domain::agent::service::X`, `domain::layout::service::X`)는 해당 함수가 실제로 같은 파일에 있으면 bare 이름으로, 같은 도메인의 다른 파일에 있으면 기존 `use super::service;`/`use super::commands;` 관행을 재사용해 축약했다. ②타 도메인 참조(`domain::file::capability::*`, `domain::git::watch::*`, `domain::layout::service::*`, `domain::settings::service::*`)는 `crate::` 접두사만 추가해 그대로 두었다(신규 `use` 도입 없이 최소 변경). doc 주석 본문(자연어 산문 속 `domain::x::y` 언급)은 코드가 아니므로 전혀 손대지 않았다.

### 검증

1. `cargo fmt --all --check` — 통과(diff 0).
2. `cargo test --workspace` — 전체 통과. lib.rs 내 잔류 바인딩/패리티 테스트 7건 포함 1071+6+17+3건 모두 green. 바인딩 생성 테스트 실행 후 `git status --short -- src/shared/api/bindings.ts` 무출력으로 표면 무변경 확인.
3. `tests/domain_boundaries.rs` 3건(도메인 경계·infra 역참조·import 형태 금지) — 화이트리스트 4건 등재 후 전부 통과. 등재 전에는 `도메인_간_참조는_types와_capability_확장점과_화이트리스트만_허용된다` 1건이 정확히 예상된 4개 edge 로 실패했음을 먼저 확인한 뒤 등재했다(우회 아님 — 공식 예외 메커니즘 사용).
4. `cargo clippy --workspace --all-targets -- -D warnings` — 통과(경고 0, `#[allow]` 미사용).

### 범위 준수

- `layout/commands.rs` 무수정(후속 에이전트 몫), `src/`(TS) 무수정, 신규 크레이트 없음, `#[allow]` 없음, git add/commit/push 없음(워킹트리 미커밋 유지).
- 매직넘버 신설 없음(이동한 상수 `LAYOUT_FLUSH_INTERVAL_MS`=2_000 은 원래 리터럴 그대로 이동, 새 리터럴 도입 없음).

### 3-R2. layout 커맨드 골격 공통화

### d-32 R2 — layout 커맨드 골격 공통화 (계약 §1-b) 실행 기록

### 실사 갱신

착수 시점 재확인: `domain/layout/commands.rs` 370줄, `flush_dirty_layouts`는 이미 R1에서
`domain/layout/service.rs`로 이관되어 있었다(현재 파일에 잔존 확인 완료). 골격 동형 13개
커맨드(`activate_tab`·`move_tab`·`split`·`resize`·`focus_pane`·`pin_tab`·`set_preview`·
`reopen_closed`·`set_view_state`·`set_dirty`·`set_terminal_session`·`open_untitled`·
`set_shell_view`)와 예외 4(`layout_get`·`open_tab`/`close_tab`·`convert_untitled`·
`move_tab_to_window`)의 실사 내용은 계약 §0과 완전히 일치했다.

### 공통화 설계

**locate 전략은 enum**(`LayoutLocate { WithTab(TabId), WithPane(PaneId), Direct(ProjectId) }`)으로
선택했다 — 클로저 대신 enum 을 쓴 이유는 (1) 3개 전략이 모두 `&HashMap<ProjectId, ProjectLayout>`
을 받아 `AppResult<ProjectId>`를 내는 동일한 모양이 아니라, `Direct` 만 조회 자체가 없는 이질적
케이스라 클로저 트레잇 하나로 통일하면 오히려 `Result<ProjectId, AppError>` 래핑을 위한 보일러
플레이트 클로저(`|_| Ok(project_id)`)가 필요해져 가독성이 떨어지고, (2) enum 쪽이 호출부에서
"이 커맨드가 무엇으로 프로젝트를 찾는가"를 한눈에 드러내 계약이 요구하는 "판단 근거 기록"에 더
적합하다고 판단했다.

**헬퍼 시그니처**(`commands.rs` 사설, `pub` 없음):

```rust
async fn run_layout_mutation<F>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    locate: LayoutLocate,
    mutate: F,
) -> AppResult<ProjectLayout>
where
    F: FnOnce(&mut ProjectLayout) -> AppResult<()>,
{
    let _guard = state.begin_mutation().await;                              // await 1회 — 유일
    let mut layouts = state.layouts.read().clone();
    let project_id = match locate {
        LayoutLocate::WithTab(tab_id) => service::locate_project_with_tab(&layouts, &tab_id)?,
        LayoutLocate::WithPane(pane_id) => service::locate_project_with_pane(&layouts, &pane_id)?,
        LayoutLocate::Direct(project_id) => project_id,
    };
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    mutate(layout)?;                                                        // 동기 뮤테이트

    let updated = service::finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}
```

이 순서(`begin_mutation → read().clone() → locate → get_layout_mut → 동기 mutate → finish_mutation
→ write()`)는 원본 13개 커맨드가 각각 인라인으로 갖고 있던 순서와 바이트 단위로 동일하다.
`mutate: F: FnOnce(&mut ProjectLayout) -> AppResult<()>` 하나로 고정해, 반환값이 다른 서비스
함수(`reopen_closed`→`Option<TabId>`, `apply_shell_view_patch`→`()`)는 클로저 안에서
`Ok(())`로 통일해 흡수했다(반환값은 원본에서도 커맨드가 사용하지 않고 버렸으므로 동작 동일).

### 맵 표 — 13개 커맨드 × locate 전략 × mutate 클로저

| 커맨드 | locate 전략 | mutate 클로저 본문 |
|---|---|---|
| `layout_activate_tab` | `WithTab(tab_id.clone())` | `service::activate_tab(layout, &tab_id)` |
| `layout_move_tab` | `WithTab(tab_id.clone())` | `service::move_tab(layout, &tab_id, &pane_id, index as usize)` |
| `layout_split` | `WithPane(pane_id.clone())` | `service::split(layout, &pane_id, edge, &tab_id)` |
| `layout_resize` | `WithPane(pane_id.clone())` | `service::resize(layout, &pane_id, sizes)` |
| `layout_focus_pane` | `WithPane(pane_id.clone())` | `service::focus_pane(layout, &pane_id)` |
| `layout_pin_tab` | `WithTab(tab_id.clone())` | `service::pin_tab(layout, &tab_id, pinned)` |
| `layout_set_preview` | `WithTab(tab_id.clone())` | `service::set_preview(layout, &tab_id, preview)` |
| `layout_reopen_closed` | `Direct(project_id)` | `service::reopen_closed(layout); Ok(())` |
| `layout_set_view_state` | `WithTab(tab_id.clone())` | `service::set_view_state(layout, &tab_id, view_state)` |
| `layout_set_dirty` | `WithTab(tab_id.clone())` | `service::set_dirty(layout, &tab_id, dirty)` |
| `layout_set_terminal_session` | `WithTab(tab_id.clone())` | `service::set_terminal_session(layout, &tab_id, session_id)` |
| `layout_open_untitled` | `Direct(project_id)` | `focused_pane`/`next_untitled_index`/`Tab` 생성 후 `service::open_tab(layout, &pane_id, tab, false)?; Ok(())` |
| `layout_set_shell_view` | `Direct(project_id)` | `service::apply_shell_view_patch(layout, &patch); Ok(())` |

각 커맨드는 원본 `tab_id`/`pane_id`를 `locate`에는 `.clone()`으로 넘기고 원본 값은 `move` 클로저에
캡처해 `mutate` 안에서 사용한다(둘 다 `Clone`인 `string_id!` 매크로 뉴타입 — `src/ids.rs`).
`open_untitled`의 사전 읽기(`focused_pane`·`next_untitled_index`)는 계약이 예상한 대로 클로저
내부(`&mut ProjectLayout` 수신 이후)에서 그대로 처리해 골격과 충돌하지 않았다.

### 예외 4 — 수제 유지(변경 없음)

- `layout_get`: 읽기 전용, `begin_mutation` 자체를 타지 않음 — 헬퍼 대상 아님.
- `layout_open_tab`/`layout_close_tab`: `service::open_tab_and_finish`/`close_tab_and_finish`에
  위임(그 함수들이 guard 를 직접 잡음) — 헬퍼로 감쌀 대상이 없음.
- `layout_convert_untitled`: guard 획득 **직후** `state.projects.read().clone()` 선행 읽기가
  `layouts.read().clone()`보다 앞선다 — 헬퍼의 고정 순서(`layouts.read().clone()`이 첫 스텝)와
  충돌하므로 계약대로 수제 유지, 순서 변경 없음.
- `layout_move_tab_to_window`: `NewAuxiliary` 분기의 `open_auxiliary_window(...).await`(비동기
  뮤테이터) + `WindowStore` State + 빈 보조창 정리(`cleanup_emptied_auxiliary_windows`) 후처리를
  가진다 — 헬퍼의 "동기 mutate 전제"(await 지점 증가 금지)와 정면 충돌하므로 계약대로 수제 유지.

### 검증 결과

1. `cargo fmt --all --check` — 최초 실행에서 헬퍼 시그니처 줄바꿈 1건 diff 발견 → `cargo fmt --all`
   적용 후 재실행, **diff 0** 확인.
2. `cargo test --workspace` — **1071 + 3(domain_boundaries) + 6(session_restore) + 17(cli) = 1097
   개 전부 통과**, 0 실패. `tests::typescript_바인딩을_생성한다`(바인딩 생성)·
   `tests::collect_commands_매크로_출력과_dispatch_테이블은_커맨드_이름_집합이_일치한다`(디스패치
   패리티) 포함. 이후 `git diff --stat -- src/shared/api/bindings.ts` **출력 없음(diff 0)** 확인 —
   커맨드 이름·인자·반환 타입 표면 무변경.
3. `cargo clippy --workspace --all-targets -- -D warnings` — **경고 0, 에러 0**.

### 결과 규모

`domain/layout/commands.rs` **370줄 → 334줄**(-36줄, -9.7%). `#[allow]`·`@ts-ignore` 류 우회 없음,
신규 크레이트 없음. TS(`src/`)·`lib.rs`는 이 작업에서 손대지 않았다(`git status` 확인 — 이번
커밋 대상 diff는 `domain/layout/commands.rs` 단독, R1이 만든 lib.rs/agent/project/window 쪽
선행 diff는 그대로 유지).


### 3-오픈이슈

- docs/ipc-contract.md·docs/data-model.md·docs/features/terminal.md·docs/PROCESS.md·docs/bug/*·docs/acknowledge/2026-08-08-agent-badge-latency-decisions.md·docs/quality-assurance/2026-08-11-qa6-checklist.md 등에 'lib.rs::restore_project_watchers'·'lib.rs::poll_agents'·'lib.rs::restore_auxiliary_windows' 형태의 구 위치 표기가 다수 남아 있음 — R1 범위(코드 이관)에는 포함되지 않아 손대지 않았으나, 새 위치(domain/project|window|agent::commands.rs)로 갱신하는 문서 동기화가 후속으로 필요함.
- R2(layout 커맨드 골격 공통화)는 이 작업에 포함되지 않음 — 계약 §1-b 그대로 후속 에이전트(sonnet+xhigh 순차) 몫.

---

## 4. 검토 반영 (2026-08-24)

> 검토 wf_df2dd6c7-a50(3렌즈): 동시성·락 **전부 통과**(13/13 역재구성 일치·run() 완전 일치·d-25 계약 보존·데드락 신규 0)·동등성 통과(멀티셋·유실 0·bindings diff 0). major 는 L3-1(화이트리스트 등재 근거) 1건 → 적대적 wf_eaa96bb6-33a **downgraded**(4개 하위 주장 중 사실 오인 2·근거 불충분 1·nit 1 — 처분: 문서 정련 한정, capability 재설계는 d-35 후보로 워처 2엣지 한정·트레잇 재설계 선행 조건 기록). minor 계 12(중복 제거) → 수정 wf_b6a32dd6-caa 전건 반영.

d-32 minor 6항목 전부 코드 동작 무변경(주석/식별자 가시성/테스트 모듈명만 수정) 원칙 준수해 처리함.

1. d32-01(layout/commands.rs run_layout_mutation doc): 동시성 렌즈가 지적한 '레이스' 논거를 삭제하고, 실제 제약 근거(계약상 await 지점 증가 금지 + FnOnce 시그니처 유지)로 교체. guard가 await를 가로질러 유지된다는 사실(layout_move_tab_to_window 실례)을 doc 안에 반증 근거로 명시해 향후 오독을 막음.

2. d32-02/L2-01/L3-4(stale lib.rs:: 포인터 8곳): state.rs 2곳, file/capability.rs 2곳, git/watch.rs 1곳, layout/service.rs 1곳을 'domain::project::commands::restore_project_watchers' 산문 표기로 통일(crate:: 접두 없음 — 경계 스캔은 //로 시작하는 줄을 제외하지만 안전하게 접두사 생략 유지). constants.rs:22는 기계적 단순 치환이 아니라, menu Quit(domain::window::commands로 이관됨)와 single-instance focus-forwarding(lib.rs 잔류) 두 흐름이 실제로는 서로 다른 위치에 있음을 실물 확인 후 각각 정확히 병기해 정정. window/service.rs:52는 지시대로 '실물 확인 후 정확히' 갱신 — plan_return_of_auxiliary_window_tabs의 실제 호출처 2곳(lib.rs on_window_event Destroyed arm 직접 호출, domain::window::commands::handle_auxiliary_close_requested를 통한 간접 호출)을 모두 반영.

3. d32-03/L3-2(MENU_ID_QUIT): pub(crate) -> private const. grep으로 정의 1건 + 사용 2건(둘 다 동일 파일) 재확인 후 사설 환원. lib.rs 참조 0건 확인.

4. d32-04/L3-3(낡은 상대 지시어 4곳): project/commands.rs :223 'in setup() below'->'in lib.rs's setup()', :226 'the old ... loop here did'->'...loop in lib.rs's setup() did'. window/commands.rs :186 'the Destroyed handler below'->'lib.rs's WindowEvent::Destroyed handler'(lib.rs:605 Destroyed arm 실물 확인). window/commands.rs :70 'boot-time restoration in lib.rs'->같은 파일 restore_auxiliary_windows(:268) 참조로 정정 — restore_auxiliary_windows가 open_auxiliary_window를 호출하는 실제 boot-time restoration 함수임을 확인.

5. L3-6(mod restore_tests): 코드베이스 전체 'mod tests' 76건 vs 'mod restore_tests' 1건(유일 예외) 확인 후 mod tests로 통일. cargo test -p taide --lib에서 해당 4개 테스트가 domain::project::commands::tests::* 이름으로 정상 통과.

6. L3-1 downgraded 처분(a) — 적대적 검증(wkxnbuq9c.output)의 verdict=downgraded, fixGuidance (a) 문서 정련 한정 지시를 그대로 따름. tests/domain_boundaries.rs:50-57의 project 화이트리스트 4건 doc을 fixGuidance 원문 그대로 재작성: ① sync/* 유비 삭제, 'the assembly (lib.rs setup) still owns the boot call order; only the step bodies live here' 실사유로 교체 ② file::capability/git::watch(감소 가능)와 layout::service/settings::service(capability 성격 아님)를 구분한 d-35 이월 deferral 문구 추가 ③ 'moved verbatim from lib.rs's former setup() body'가 부정확함(실제로 baseline에서 이 3함수는 lib.rs 최상위 fn이었고 setup()은 호출만 했음)을 반영해 'moved verbatim from lib.rs's former top-level boot-restore helpers, called from setup()'로 정정. fixGuidance (b)(capability 재설계)는 명시적으로 '지금 채택 불가, d-35 후보로만 이월'이라 코드 변경 대상에서 제외했고, 이번 작업에서도 손대지 않음. architecture.md 갱신(fixGuidance 2번)은 docs/ 수정 금지 제약으로 이번 범위에서 제외 — open issue로 남김.

전 항목 cargo fmt --all --check clean, cargo test(lib 1071/1071 · domain_boundaries 3/3 · session_restore 6/6) 전부 green, cargo clippy -D warnings 경고 0, #[allow] 추가 0건으로 기계 검증 완료.

**메인 보완**: fixer 범위 제약(docs/ 수정 금지)으로 남긴 2건은 메인이 직접 반영 완료 — ① docs/architecture.md §2 에 tier-2 경로(부팅 1회성 복원 — 조립부가 호출 순서 소유 조건) 명문화 ② 라이브 문서 4건(ipc-contract.md:141·:240, data-model.md:260, features/terminal.md:170) 경로 정정.
