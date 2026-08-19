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
