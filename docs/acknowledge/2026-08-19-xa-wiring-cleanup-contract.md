# X-A 배선 + 소규모 잔여 청소 배치 계약 (2026-08-19)

> 정본: 감사 `2026-08-18-architecture-audit.md` §7 X-A(살리기/지우기 판정은 T0 계약 결정 11 로
> **기승인** — 살리기: cwd-changed·from_app·revision·layout_set_view_state / 지우기:
> focus-kind-changed·중복 커맨드 5종) + T1 3차 계약 §5.1 소규모 잔여(2·3·4·7·8).
> 사용자 승인(2026-08-19): prod 병합 완료(main=19d77e6) + 본 배치 착수("추천안대로 다 해놔").
> 착수 전 메인 실물 확인(전건 감사 상태 유효): layoutSetViewState 프론트 호출 0 / from_app
> 상수 false(watcher.rs:88) / LayoutChanged.revision 프론트 소비 0 / focus-kind-changed
> 현존(events.rs:46) / 중복 커맨드 5종 현존 / terminal:cwd-changed 이벤트·resolve_terminal_path
> 현존(호출자 0).

## 1. 범위

### 1.1 X-A 살리기 4건 (죽은 배선 완성)

- **X1#4 `layout_set_view_state`**: monaco viewState(커서·스크롤) 저장 배선 — 탭 전환/닫기 시
  `editor.saveViewState()` 를 커맨드로 영속, 탭 복귀/재시작 시 `restoreViewState()`. 문서
  (data-model·ipc-contract)가 정본으로 기재한 기능의 실현. 저장 시점·과호출 방지(디바운스/전환
  시점 한정)는 구현 판단 — hot-exit 미러·layout dirty flusher 와의 상호작용 실코드 확인 필수.
- **X1#10 `FsChange.from_app`**: 앱 자신의 쓰기(file_save·search_replace·write_atomic 계열)가
  유발한 watcher 이벤트를 echo 로 마킹 — Rust 쪽 최근 자기-쓰기 경로 기록(짧은 TTL)과 watcher
  발행 시 대조로 `from_app: true` 설정. 프론트 ipc-sync-provider 의 fsChanged 가 from_app 이벤트
  의 트리 refresh/FILE.CONTENT 무효화를 스킵(R4#13 근본 완화 — 자기 저장마다 N+1 재조회 제거).
  마킹 누락(false negative)은 현행 동작과 동일하므로 안전 방향.
- **X1#11 `LayoutChanged.revision`**: 프론트가 마지막 관측 revision 을 보관하고 낮거나 같은
  revision 의 이벤트를 무시(멀티윈도우 stale 갱신 방지 — 원래 목적). 레이아웃 캐시 갱신 지점
  (ipc-sync 계열) 실코드 확인 후 게이트 삽입.
- **X1#1 `terminal:cwd-changed`**: 셸 통합(OSC 시퀀스)에서 cwd 변화를 감지·발행하고,
  `resolve_terminal_path`(호출자 0)를 소비 경로에 배선 — 터미널 상대 경로 링크(cmd+click)의
  cwd 기준 해석 전제 완성. 셸 통합 인프라(shell_integration.rs)의 기존 시퀀스 처리 확인 후
  최소 배선(과설계 금지 — 이벤트 발행+프론트 세션별 cwd 보관+링크 해석 시 사용).

### 1.2 X-A 지우기 (dead 배선 제거)

- **X1#12 `project:focus-kind-changed`**: 이벤트·타입·발행 지점 전량 제거(소비 0·레이아웃 변이
  18종마다 무조건 발행 — 트래픽만 2배). events.rs·collect_events!·bindings·이벤트 파리티 테스트
  예외 목록 갱신.
- **X1#13 중복 커맨드 5종 제거**: `ide_start`/`ide_stop`/`remote_start`/`remote_stop`(
  settings_update 부수효과와 표면 중복)·`window_open_auxiliary`(layout_move_tab_to_window 와
  중복). collect_commands!·dispatch(ALLOWED/DENIED 테이블 포함 — T1-K 완전 분할 파리티가 제거
  누락을 자동 검출)·bindings 재생성·프론트 잔존 호출 0 확인. `git_ahead_behind`·`git_fetch`·
  `git_undo_last_commit` 은 **유지**(UI 미구현 예약 — ipc-contract 에 "예약" 표기).
- **X1#2 잔여 소비 판정**: `terminal:exited` 구독 0(죽은 pty alive 취급) — 세션 정리 트리거로
  소비 신설. `remote:state-changed`·`app:ready` — 실코드 확인 후 소비 신설이 자연스러우면 배선,
  아니면 제거(app:ready 는 T2-F dead 후보 — 제거 우선 검토). 판정·근거 보고.

### 1.3 §5.1 소규모 잔여

- **(2) open-with-registry 생명주기**: LRU 상한(임시)을 유지하되 탭 닫기(useCloseTab)·프로젝트
  종료 시 해당 경로 오버라이드 정리 훅 배선(editor-pane 배치에서 소유 경계로 못 했던 것 —
  이번엔 entities/editor + 소비 훅 함께 소유).
- **(3) ws.rs writer 무한 대기**: 무트래픽 세션에서 도메인 스토어가 tx clone 을 보유하면 writer
  태스크가 종료되지 않아 프루닝·클라이언트 카운트 지연 — 세션 만료 Close 프레임 flush 를 보장
  하면서 유한 대기(타임아웃 상수) 후 정리하는 안전한 종료 시퀀스로. T1-K 검토가 확인한 진입
  구조 불변.
- **(4) 재핸드셰이크 실패-확인**: 렌더러 재시도 소진 시 호출할 실패 보고 경로 신설(신규 커맨드
  `lsp_report_reinitialize_failure` 또는 기존 상태 갱신 확장 — 구현 판단, T1-K 기본 거부 구조에
  등재 필수). last_error 를 "자동 재시작 대기" 문구에서 실패 사실로 갱신(상태바 폴링이 정직해짐).
- **(7) `TREE_ROWS_UNBOUNDED_LIMIT` 센티널 제거**: `tree_rows` 의 `limit: Option<u32>` 전환
  (None=전량) — Rust 시그니처+bindings+프론트 소비 동반.
- **(8) PROJECT_SCOPED_KEYS 경로 키 커버**: FILE.CONTENT/RAW 등 경로 키가 projectClosed 정리에서
  빠지는 공백 — 프로젝트 root prefix 기반 predicate 제거 추가(실피해 낮음 평가였으나 완결).

## 2. 실행 구조

- **Phase R(Rust 단독, sonnet+xhigh)**: §1.1 Rust 측(viewState 커맨드는 기존재 — 저장 필드·발행
  경로 확인, from_app 에코 마킹, revision 은 기발행, cwd-changed 발행, exited/상태 이벤트) +
  §1.2 전량(이벤트·커맨드 제거, 파리티·테이블 갱신) + §1.3 (3)(4)(7) Rust 측. cargo 그린 +
  bindings 재생성.
- **Phase F 병렬 2(sonnet+xhigh, R 후)**:
  - F1 = viewState 프론트 배선(layout 계열·editor-pane 훅 접점)+revision 게이트+from_app 소비
    (ipc-sync-provider)+PROJECT_SCOPED_KEYS 경로 predicate+TREE_ROWS 소비.
  - F2 = cwd-changed 구독·터미널 링크 해석+terminal:exited 소비+open-with 생명주기 정리+
    재핸드셰이크 실패 보고 소비(lsp-session-registry 재시도 소진 경로).
- **메인 통합**: 접합부 확인+verify·vite build 직접.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 정확성: viewState 저장/복원 시점과
  hot-exit·레이아웃 flusher 경합, from_app 오마킹(진짜 외부 변경을 스킵) 위험, revision 게이트의
  창간 순서, ws 종료 시퀀스 / 계약: 제거 5종+1이벤트의 잔존 참조 0·파리티 전체 그린 / 보안:
  신규 커맨드의 T1-K 등재 적정) → 적대적 → 수정 → 메인 2차 → 커밋(dev).

## 3. 완료 조건

- verify+vite build 그린. 이벤트/커맨드 파리티(제거 반영)·T1-K 완전 분할·arm 파리티 전부 그린.
  신설 배선 회귀 테스트(from_app 에코·revision 게이트·viewState 왕복·cwd 해석).
- 실기 이월(qa6): 탭 전환·재시작 후 커서/스크롤 복원·자기 저장 시 트리 무점멸·터미널 상대 경로
  링크·원격 세션 만료 시 리다이렉트 무회귀.

---

## 4. 구현 완료 기록 (2026-08-19, Phase E 검토 전)

> 구현 wf_48420169-27f(R→F1·F2 병렬, sonnet+xhigh) + 메인 접합부 2건. 메인 2차: verify·vite
> build 직접 재실행(결과는 §5 검토 후 기록과 함께).

- **R(살리기)**: from_app — `infra/self_write.rs` SelfWriteTracker(TTL 2s·경로별 1회 소비) 신설,
  파일 쓰기 6경로 마킹·watcher 그룹 전 경로 일치 시만 true(부분 일치는 보수적 false). cwd —
  셸 통합 훅에 OSC7 순수 경로 시퀀스 추가(`extract_latest_cwd`, 유일 소비자 자체 파서라 percent-
  encoding 생략 — 외부 터미널 미러링 시 재검토 조건 기록). viewState·revision 은 Rust 기구현
  확인(무변경, 발행 규약 문서화만).
- **R(지우기)**: focus-kind-changed 이벤트·FocusKind 타입·발행 전량 제거 + **app:ready 도 제거**
  (발행 지점 0 확증 — T2-F 선반영). 중복 커맨드 5종 제거(settings_update 의
  apply_integration_toggles 가 유일 실도달 경로임을 확증 후 — ide_start/remote_start 는 내부
  함수로 존속·래퍼 3종 삭제). git 3종 유지+"예약" 표기. 커맨드 180→176(+raw 3=179)·이벤트
  25→23·ALLOWED 163→160·DENIED 20→19, 전 파리티 그린.
- **R(잔여)**: ws writer 유한 대기(3s 타임아웃 후 abort, Close 프레임 플러시 보존·leaked-sender
  회귀 테스트) / 신규 `lsp_report_reinitialize_failure`(세대 가드 공유·Crashed 확정·T1-K 허용
  등재) / tree_rows `Option<u32>`.
- **F1**: viewState 배선 신규 `use-editor-view-state.ts` — 저장은 effect cleanup 한정(React
  커밋 순서를 이용해 모델 스왑 전 저장·값 dedup 으로 IPC 절약), 복원은 인스턴스당 1회·
  consumePendingReveal 이 항상 우선. revision 게이트(프로젝트별 lastSeen·`isStaleLayoutRevision`).
  from_app 소비 — **계약 이탈 판단**: 트리 refresh 만 스킵하고 FILE.CONTENT invalidation 은
  유지(search_replace 가 자체 onSuccess invalidation 이 없어 열린 탭 갱신의 유일 경로가 watcher
  — 문자 그대로 스킵하면 회귀. 근거 실코드 확증, Phase E 재확인 대상). TREE_ROWS 센티널 제거·
  경로 키 predicate(PROJECT_SCOPED_PATH_KEY_PREFIXES — root 를 스윕 전에 읽는 순서 처리).
- **F2**: cwd 구독+터미널 파일 링크 실배선(미배선이던 findTerminalLinkMatches 를 ILinkProvider
  로·resolveTerminalPath 첫 소비자·requestOpenFileFromEditor 재사용, 물리 행 단위 한정 —
  최소 배선 명시). terminal:exited 소비(exited 표시+Restart+SESSIONS 무효화·session.running
  첫 소비). open-with pruneOpenWithOverrides+useCloseTab 배선(다른 창 잔존 확인 후 정리).
  재핸드셰이크 실패 보고 소비(재시도 소진 시 호출). lsp.ipc.ts 1함수 추가는 소유 경계 최소
  이탈(패턴 정합 — 승인).
- **메인 접합부 2건**: `app.type.ts` AppReady re-export 제거(app:ready 삭제 여파 — R 이 지목한
  유일 tsc 에러). remote:state-changed 프론트 소비 배선(sync:state-changed 선례 미러 —
  setQueryData(REMOTE.STATUS), X1#2 판정 완결).

## 5. Phase E 검토 반영 (F2, 2026-08-19)

> 정본: 전문 QA 검토(4렌즈) confirmed major 4건(F2 소유분) — from_app 스킵 회귀 2건(#4·#8, LSP
> WorkspaceEdit·원격 dispatch 의 파일 생성/이름변경/삭제가 자체 트리 refresh 없이 from_app 마킹
> 되어 탐색기 미갱신) + pruneOpenWithOverrides dead export 3건(#5·#10·#15, 계약 §1.3(2) 의 프로젝트
> 종료 시 정리 미배선).

- **from_app 스킵 범위 축소**: `ipc-sync-provider.tsx` 의 스킵 조건을 `change.fromApp` 단독에서
  `isSelfEchoWithoutTreeImpact`(`fromApp && kind === 'modified'`)로 좁혔다. §4 가 전제했던 "from_app
  마킹 오퍼레이션은 전부 호출부에서 refresh 하거나 트리를 안 건드린다"는 `workspace-edit-applier.ts`
  의 리소스 오퍼레이션(LSP WorkspaceEdit 의 파일 생성/이름변경/삭제)과 원격 dispatch 의
  `file_create`/`file_rename`/`file_delete` arm 에서 성립하지 않았다(둘 다 트리 갱신 호출부가 없음).
  `created`/`renamed`/`removed` 종류는 이제 `fromApp` 값과 무관하게 항상 refresh 하므로, R 이
  진행 중인 self-write 마킹 정확도 개선(원자적 쓰기 temp sibling 관련 findings #3·#9·#14, R 소유)
  과 독립적으로 이 회귀가 닫힌다. `modified` 만 스킵을 유지해 R4#13 의 자기 저장 N+1 완화 목적은
  보존.
- **open-with 프로젝트 종료 정리 배선**: §4 가 "pruneOpenWithOverrides+useCloseTab 배선"이라 기록한
  것은 부정확했다 — 실제로 탭 닫기는 `setOpenWithOverride(path, null)` 뿐이었고 `pruneOpenWithOverrides`
  는 테스트 외 호출자가 0 이었다. `ipc-sync-provider.tsx` 의 `projectClosed` 핸들러에
  `pruneOpenWithOverrides(collectOpenFilePathsOutsideProject(...))` 를 신규 배선했다 — 캐시에 남은
  다른 프로젝트들의 레이아웃(메인+보조 창, `collectAllPaneTabs`)에서 아직 열려 있는 파일 경로를
  모아 keepPaths 로 넘기므로, 방금 닫힌 프로젝트에만 속했던 오버라이드만 정리되고 다른 프로젝트가
  여전히 열어 둔 경로의 오버라이드는 보존된다. `open-with-registry.ts` JSDoc + `docs/architecture.md`
  의 레지스트리 수명表 동반 갱신(“명시적 해제 없음” 서술이 이제 사실과 다름).
- **범위 밖(R 소유, 확인만)**: findings #3·#9·#14(file_save/search_replace 의 temp-sibling 이 watcher
  그룹을 오염시켜 macOS 에서 from_app 마킹이 사실상 항상 false)는 Rust 측 self-write 마킹/watcher
  로직의 결함으로 이 배치의 소유 경계 밖이다. 위 스킵 범위 축소는 그 결함과 독립적으로 안전하다 —
  마킹이 틀려도(false 로 과소평가되어도) 트리가 과다 refresh 될 뿐 탐색기가 stale 해지는 방향의
  결함은 만들지 않는다.

## 6. Phase E 검토 반영 (R, 2026-08-19)

> 정본: 전문 QA 검토(4렌즈) confirmed major — from_app temp-sibling 미마킹 3건(#3·#9·#14, R 소유분,
> §5 의 "범위 밖" 항목과 동일 findings) + OSC7 종결자/스킴 버그(#6) + resolve_terminal_path 무가드
> (#7·#11, 보안) + OSC7 cwd 무검증(#12, 검토 후 minor 로 재분류 — 하류 root guard 로 실피해 없음) +
> lsp_report_reinitialize_failure 세대 가드 문서 과장(#13, 검토 후 correctness/major 로 재분류).

- **from_app temp-sibling 근본 수정**: `infra::persist::is_temp_sibling` 신설 — `write_atomic` 이
  만드는 `.{name}.{uuid}.tmp` 형제 경로를 watcher 단계(`group_relevant_changes`)에서 그룹 진입 전에
  제외한다. 동시에 `infra::self_write::resolve_from_app` 를 그룹 1개가 아니라 **디바운스 배치
  전체**(`Vec<FsChange>`)를 받도록 재설계 — 같은 최종 경로가 `write_atomic`+notify 특성상 여러
  `FsChangeKind` 그룹(Created/Modified/Renamed)에 나뉘어 나타나도 마킹 판정을 배치당 한 번만
  소비하고 모든 그룹에 동일하게 적용한다(temp 필터만으로는 경로당 1회 소비 마킹이 여러 그룹으로
  쪼개져 최대 한 그룹만 true 가 되는 문제가 남는다는 걸 검토 과정에서 확인 — 두 수정을 함께
  적용). `infra::watcher::start_watch` 의 콜백 시그니처가 `Fn(FsChange)` → `Fn(Vec<FsChange>)` 로
  바뀌어 `project::commands::attach_watcher`/`attach_git_watcher` 양쪽 호출부를 함께 갱신했다.
  `FsChange.kind` 필드(기존 타입, 신설 아님)가 프론트의 `kind === 'modified'` 스킵 축소(§5)와
  나란히 쓰일 수 있음을 `docs/ipc-contract.md` 에 명시.
- **OSC7 종결자·스킴 버그 수정**: `shell_integration::extract_latest_cwd` 가 ST 를 전역 우선
  탐색하던 것을 ST/BEL 중 더 앞선 종결자를 채택(`earliest_terminator`)하도록 변경 — BEL 로 끝난
  시퀀스가 뒤쪽의 무관한 ST(TAIDE 자신의 OSC 133 마커 등)까지 삼키던 버그를 닫는다. `file://` 로
  시작하는 payload(표준 OSC 7 스킴, 사용자 rc 가 별도로 뱉을 수 있음)는 채택하지 않고 무시해,
  TAIDE 자신의 순수 경로 보고를 덮어쓰지 못하게 했다(host 제거+percent-decode 대신 무시 쪽 선택 —
  이 파서의 유일 소비자가 자체 파서라는 기존 "최소 배선" 설계와 일관).
- **resolve_terminal_path 루트 가드 추가**: `AppState` 를 받아 `root_guard::resolve_owning_project`
  로 결과(canonicalize 된 절대경로)를 검증하고, 열린 프로젝트 루트 밖이면 `Forbidden` 이 아니라
  `AppError::NotFound` 로 거부한다(존재 여부 오라클 차단 — 루트 밖 실재 파일과 존재하지 않는 파일이
  같은 에러로 응답됨). 가드 로직은 `guard_terminal_path(projects, path, cwd)` 로 분리해 `State`
  없이 직접 테스트 가능하게 했다. `cwd` 자체는 별도로 선제 검증하지 않는다 — 절대경로 `path` 는
  `cwd` 를 아예 쓰지 않으므로(`service::resolve_terminal_path`), 선제 검증을 하면 세션 cwd 가
  정상적으로 프로젝트 밖을 가리키는 흔한 경우(사용자가 터미널에서 `cd ~` 등)에 절대경로 링크까지
  잘못 거부하게 된다 — 최종 resolved 경로 하나만 검증하는 쪽이 더 정확하고 더 안전하다. 원격
  허용(`REMOTE_ALLOWED_COMMANDS`)은 유지 — `dispatch.rs` 의 arm 이 `app.state()` 를 추가로 전달하도록
  갱신, `path`/`cwd` 두 인자는 원격 와이어 프로토콜 변화 없음(프론트 `resolveTerminalPath(path,
  cwd)` 시그니처도 무변화 — `State` 추출자는 TS 바인딩에 노출되지 않음).
- **lsp_report_reinitialize_failure 세대 가드 문서 정정 + 실결함 수정**: 검토가 "generation-
  mismatch-is-ignored guard already defends against a stale or spoofed report" 문구가 위조 방어를
  주장하는 게 부정확함을 확인(인증된 원격 피어는 `lsp_sessions` 로 현재 generation 을 그대로 읽어
  일치시킬 수 있다 — 경합 방어일 뿐). 문서를 "race guard" 로 정정했다. 동시에 검토가 발견한 비적대적
  실결함(`lsp_restart` 가 `generation` 을 올리지 않아, 수동 재시작 이후 옛 재시도 루프가 뒤늦게
  실패를 신고하면 방금 복구된 `Running` 세션이 다시 `Crashed` 로 잘못 표시됨)은 `status == Crashed`
  전제(`should_apply_reinitialize_failure`)를 추가해 닫았다. `lsp_confirm_reinitialize` 에는 대칭
  전제를 추가하지 않았다 — 그 커맨드의 목표 상태(`Running`)는 이미 건강한 세션에 적용해도 무해해
  상태 전제가 불필요하기 때문이며, 이는 T1-K 의 "원격=인증 동급 신뢰, 호출자 identity 는 인가 축이
  아니다" 선례와 어긋나지 않는다(owner/identity 검사를 추가한 게 아니라 상태 전제를 추가한 것).
- **OSC7 cwd 무검증 채택(#12)은 별도 수정 없음**: 검토가 major → minor 로 재분류했다 — 오염된 cwd
  로 얻은 경로도 `resolve_terminal_path` 의 신규 루트 가드에서 결국 거부되므로(내용 유출 불가,
  존재 오라클도 이미 위 가드가 닫음), 그리고 오염 없이도 절대경로 링크로 동일 도달 범위가 이미
  존재하므로(신규 능력 부여 아님) 이 배치에서는 하드닝하지 않았다.

---

## 6. Phase E 종합·메인 2차 (2026-08-19)

> 검토 wf_90e96573-b9f(4렌즈, 발견 45: major 16·minor 29) → 적대적 16건 **confirmed 16·refuted 0**
> (이 배치 검토가 신설 배선의 실결함을 전량 적중 — 검토 단계 값어치 재입증) → 수정
> wf_553b8d99-ecd(R·F1·F2 병렬). 메인 2차: 수정 7축 스팟 전건 실물 확인 + verify·vite build.

- **R**: temp sibling 을 watcher 그룹 진입 전 제외 + resolve_from_app 을 디바운스 배치 전체
  판정으로 재설계(그룹 분할 잔여 결함까지 — from_app 실효화 완성) / OSC7 earliest_terminator
  (min(ST,BEL))·file:// 무시 / **guard_terminal_path 신설**(root_guard 경유·루트 밖 NotFound
  통일로 존재 오라클 차단·cwd 는 최종 경로 단일 검증 — 절대경로 오탈락 방지 근거 기록) /
  세대 가드 doc 을 race guard 로 정직화 + **비적대 실결함 추가 수정**(status==Crashed 전제 —
  수동 재시작 후 늦은 실패 신고가 Running 을 뒤집던 경로. confirm 은 목표상태 무해로 비대칭
  아님 판단).
- **F1**: 저장 effect 를 useLayoutEffect 로(React 소스 대조 — layout cleanup 이 passive dispose
  보다 항상 선행, editor-pane·code-editor 무변경으로 해소) / hasRestorableModel 가드(실적용
  시에만 복원 소모) / registerViewStateFlush 별도 맵(기존 mirror flush 와 키 충돌 시 미저장
  복구가 깨지는 것을 발견해 분리) / minor 4 수정(shape 검증·select 구독 축소 등).
- **F2**: isSelfEchoWithoutTreeImpact(fromApp && kind==='modified' — 트리 구조 변경 종류는 항상
  refresh, WorkspaceEdit·원격 조작 회귀 해소·N+1 목적 보존. FsChangeKind 기존재라 R 대기 불요)
  / collectOpenFilePathsOutsideProject + projectClosed prune 배선(타 프로젝트 열린 경로 보존).
- **운영 교훈(2회 재발)**: 병렬 에이전트 공유 워킹트리에서 `git stash` 는 타 에이전트 산출물을
  통째로 스태시해 pop 충돌을 유발 — 이번 배치에서 2회 발생, 전량 수동 복구·diff 대조·전체 그린
  으로 무손실 확증. **이후 모든 구현 Workflow 프롬프트에 "git stash 금지" 명시**(메인 규칙화).
- **이월 잔여(신규)**: ① 탭 닫기 시 viewState 영속 실패(closed_tabs 미검색 — 재열기 복원 공백,
  백엔드+프론트 걸침) ② useReplaceSearch onSuccess invalidation 부재(F1 계약 이탈의 근본 —
  entities/search 정비 시) ③ REMOTE.STATUS 5초 폴링과 이벤트 push 중복(무해 — 폴링 완화 검토)
  ④ isQueryKeyUnderProjectRoot JSDoc 오참조·best-effort 한계. PROCESS 잔여 총괄에 등재.
