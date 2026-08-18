# 아키텍처 감사 T0(즉시 수정) 24항목 수정 계약 (2026-08-18)

> 감사 보고서 정본: `docs/quality-assurance/2026-08-18-architecture-audit.md`(§6.1 T0 24항목·§9 결정).
> 사용자 결정(2026-08-18, 4문항 전부 추천안): ① T0 지금 착수 ② 세부 8건 전부 추천안 ③ 대형
> 3건(T1-H·T2-E·C13)은 T1 편성에 포함 ④ e2e 파일럿 지금 준비·실행(독립).
> 메인 실물 재검증 9건 전건 확정(보고서 §2·§2 2차). 감사 에이전트 보고 불신 원칙대로 하중 주장은
> 착수 전 재확인 완료.

## 1. 세부 결정 (사용자 승인 — 보고서 §9)

| # | 결정 | 채택 |
|---|------|------|
| 1 | watcher `.git` 필터 수정 시 기존 테스트 뒤집기 | **승인** — 테스트가 결함을 정상으로 고정 중, 함께 정정 |
| 2 | editor-instance-registry 수정 방식 | **effect 이동**(계약 무변경. `key={tabId}` 는 hot-exit 미러 전제 변경이라 배제) |
| 3 | F5#1 severity | **critical→major 하향 승인** |
| 6 | `lsp_install` 원격 정책 | **거부**(plugin_install·vsix_* 와 일관) |
| 7 | 설정 "해제" 규약 | **빈 문자열=해제 일반화**(ai_omlx_base_url 기존 선례 계승, Option<String> 국소. Option<Option<T>> 배제) |
| 8 | git 반환 경로 규약 | **StatusRow/CommitFile 에 절대 경로 동봉**(계약 변경·정확) |
| 9 | pty 세션 소유자 | **Rust: project_close 일괄 kill + 탭 닫기 시 pty_kill**(PtySession Drop 동반) |
| 11 | X-A 살리기/지우기 | **일괄 승인**(살리기 3: cwd-changed·from_app·revision / 지우기: focus-kind-changed·중복 커맨드 5종 / layout_set_view_state 는 살리기). 단 구현은 X 트랙(별도 커밋)—T0 에는 이미 승격된 X1#3(#14)·X1#7(#15)만 |

## 2. T0 24항목 (보고서 §6.1)

### 2.1 T0-a 보안·신뢰 경계 (Rust dispatch 중심)

- **#12** `agent_cli_install`·`agent_cli_uninstall` 원격 deny arm(공용 헬퍼, IMPLEMENTED 유지·파리티). 근거: 원격이 데스크톱에 관리자 프롬프트+`/usr/local/bin` 심링크. dispatch.rs:636-637 실확인.
- **#13** `agent_hooks_install` — Project 스코프만 원격 허용, User 스코프(`~/.codex` 등 root-guard 밖 command 훅 주입) 거부 분기.
- **#14** `AgentExternalOpen` 을 `fanout_remote_events!` 에서 제외 + `agent_pending_external_opens` deny arm(원격이 데스크톱 CLI 열기 가로채고 waitMarker 원격 realm 등록→CLI 영구 블로킹).
- **#15** `project_close` 에 `asset_protocol_scope().forbid_directory(root)` 추가(닫힌 프로젝트 트리를 webview 가 계속 읽는 표면 회수. forbid_directory 현재 0건).
- **#16** `lsp_install` 원격 deny arm(사용자 결정 6 — 거부).

### 2.2 T0-b 데이터 손실·손상

- **#17** `search_replace` 뮤테이션 가드 적용(현재 무가드 — 프로젝트 전역 파일 덮어쓰기. line 97-110 재확인). 대상 파일 확정 후 파일 단위 재획득 권장(전역 락 장기 점유 회피).
- **#18** git `StatusRow`·`CommitFile` 에 절대 경로 동봉(결정 8). repo 상대 경로가 file 도메인 절대 전제와 어긋나 dev 에서 다른 파일 열림/저장 덮어씀. 소비부(git-panel-container) 배선 갱신.
- **#19** `file_save`·`search_replace` 가 원본 파일 모드 보존(`write_atomic_with_mode` — persist.rs 의 create→rename 이 실행 비트 소실). 실행 스크립트 회귀 테스트.
- **#20** `pty_write` 락 범위를 핸들 조회까지로 축소(현재 store lock 을 write 완료까지 보유 → 자식 stdin 미독 시 앱 전체 프리즈. terminal/commands.rs:200-204).

### 2.3 T0-c 기능 무력화

- **#21** pty 세션 회수 경로 신설(결정 9 — Rust): `project_close` 가 해당 projectId 세션 일괄 kill, 탭 닫기 시 `pty_kill` 배선(현재 호출부 0), `PtySession` Drop + pause 해제 선행(1차 #10 흡수).
- **#22** `SettingsPatch` 3상태 표현(결정 7 — 빈 문자열=해제 일반화, Option<String> 6필드: editorFontFamily·uiFontFamily·terminalFontFamily 등). font-picker 의 null 선택이 백엔드 도달.
- **#23** 검색 오프셋을 UTF-16 코드유닛 변환 + `column` 1-기반 보정(search/service.rs 바이트 인덱스가 프론트 UTF-16/1-기반과 어긋나 ASCII 에서도 1칸·한글에서 다칸 어긋남). 한글 회귀 테스트.
- **#24** LSP 자동 재시작 완화(재핸드셰이크 없이 `Running` 보고하지 않고 `Crashed` 유지 — 죽은 세션 running 오보 차단. 근본 수정=세션 세대 이벤트는 T1-D).

### 2.4 1차 T0 #1~#11 (보고서 §5 T0)

- **#1** watcher `.git` 필터를 감시 루트 기준 상대경로로 좁힘(현재 조상 경로에 `.git`/`dist`/`target` 있으면 워처 전면 무력. 결정 1 — 기존 테스트 함께 정정). 수정 후 이벤트 폭주 여부는 실기 확인 이월.
- **#2** `project_close` 시 `dirty_layouts` 동기 flush + flusher `filter_map` 미스를 warn 노출(미저장 레이아웃 무경고 폐기).
- **#3** editor-instance-registry 등록을 tabId 의존 effect 로 이동(결정 2).
- **#4** auto-save 타이머에 path 캡처 + 발화 시 일치 검사(경로 전환 시 A 에 B 내용 기록 차단. previewTimeout 동반).
- **#5** `isWithinRoot` 경로 정규화 + 구분자 중립화(문자열 prefix `..` 우회 + Windows 전량 거부 동시 해소).
- **#6** 세션 applyEdit 핸들러를 `initialize` **이전** 등록, 무루트 전역 fallback 제거(#5 와 한 작업).
- **#7** `delete_theme`·`load_theme` 에 `ensure_safe_component` 적용(save_theme 은 검증, delete 는 미검증 — 원격 노출).
- **#8** ime-debug 수집을 진단 플래그 게이트(기본 비수집. 현재 터미널 입력 원문 300건 상시 + 클립보드 복사 커맨드).
- **#9** IDE 프로토콜 3종 + 상태 sync + 진단 push 를 상시 프로바이더로 승격(현재 Zen+상태바 숨김 시 사라지는 StatusBarContent 소유. C4 대표).
- **#10** → #21 로 흡수.
- **#11** wait-marker 해제를 탭 생명주기로 이동(창 간 이동 존재 — realm 로컬 Map 부적합. 외부 CLI 앱 종료까지 블록).

## 3. 실행 구조

- **Phase R1(Rust 보안 단독, sonnet+xhigh)**: §2.1 전량(dispatch.rs deny arm 5클러스터 + agent 스코프 분기) + 원격 거부 테스트 + fanout 목록 정정 + ipc-contract 원격 거부 표 갱신. cargo fmt/clippy/test·bindings.
- **Phase R2(Rust 데이터·기능 단독, sonnet+xhigh — R1 직후 순차)**: §2.2 #17·#19·#20, §2.3 #21·#22·#23·#24, §2.4 #1·#2·#7 + #18 의 git 반환 절대경로(Rust 측). watcher 테스트 정정. locale 신규 키(있으면 4곳). cargo 그린 + bindings 재생성(R1 이후라 충돌 없음).
- **Phase F 병렬 4(sonnet+xhigh, 파일 소유 분리 — R2 완료 후)**: F1=editor-pane auto-save(#4)+editor-instance effect(#3) / F2=isWithinRoot·applyEdit(#5·#6, shared/lib/lsp) / F3=IDE 프로바이더 승격(#9)+wait-marker 생명주기(#11, app/providers·app-shell·status-bar) / F4=git 경로 소비부 절대경로(#18 프론트)+ime-debug 게이트(#8)+settings patch 프론트 반영(#22).
- **Phase D 통합(단독)**: 접합부·문서(architecture·ipc-contract·data-model·features/git·terminal·lsp·qa6 T0 재검 절·docs/bug 신규) + 전체 verify + vite build.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 정확성: 락 축소 경합·pty Drop·SettingsPatch 하위호환 / 보안: deny arm 완전성·경로 정규화 우회) → 적대적 검증(opus+high) → 수정 → 메인 2차 → 커밋(dev).

## 4. 범위 밖 (이번 T0 제외 — 후속 트랙)

- **X-A 배선(#14·#15 제외)**: 살리기 3·지우기 판정은 승인됐으나 구현은 별도 X 트랙 커밋(cwd-changed 는 resolve_terminal_path 와 한 세트·layout_set_view_state 는 viewState 저장 배선 — 규모 큼).
- **T1 전체**(11묶음 — 대형 3건 T1-H·T2-E·C13 포함, 사용자 결정 3). T1-H 는 착수 시 동시성 위험 재고지.
- **X-B 테스트 신설**(T1-E), **X-C 문서 7건**(코드 수정 후 자동 정합분 제외).

## 5. 완료 조건

- `bun run verify` 전체 + vite build. 파리티(신규 deny arm·SettingsPatch bindings)·locale en⊆required.
- 4렌즈+적대적 검증+메인 2차 통과. 초점: deny arm 라우팅 실효(헬퍼 직접 호출 아닌 dispatch 경로)·search_replace/pty 락 축소 경합·SettingsPatch 빈문자열 규약 하위호환(구 settings.json)·경로 정규화 우회(`..`·심링크·유니코드)·watcher 필터 정정 후 이벤트 정합·pty Drop 이중 kill 무해성.
- 실기 이월(qa6): watcher 이벤트 폭주·pty 탭닫기 회수·검색 한글 오프셋·설정 해제 반영·IDE 배지 Zen 무관 표시·원격 거부 5종.

---

## 6. Phase E 검토 결함 수정 반영 (2026-08-18)

> 검토 wf_ae1ed818-690(4렌즈 opus+xhigh, 대상 T0 워킹트리 diff) — 발견 9(major 3·minor 6) →
> 적대적 검증 major **3건 전건 confirmed·반증 0**(2건은 forbid_directory 동일 근원) → 수정 9
> fixed·0 rejected. 메인 2차: 실물 재검증(#15 롤백·#18 절대경로·spawn_blocking) + verify·vite build.

- **[major] #15 forbid_directory 가 되돌릴 수 없어 재오픈·중첩 프로젝트 asset(video/audio 미리보기)
  접근 영구 차단** — Tauri 2.11.5 fs::Scope 는 add-only(forbidden 제거 API 부재)+forbid 가 allow
  우선. 프로젝트 닫고 같은 경로 재오픈 시 allow_directory 재호출이 무효. 벤더 소스 확인. **→ #15
  (forbid_directory) 롤백(#15 이전 동작 복귀).** **asset scope 회수(감사 X1#7 보안)는 T0 에서
  미달성 — T1 이월**(register_uri_scheme_protocol 앱 레벨 재등록으로 근본 해결). 회귀 도입 금지가
  미완성 보안 개선보다 우선. docs/bug 15행 취소선+T1 후속 기록.
- **[major] #18 abs_path 소비처 마이그레이션 누락 — editor-pane isConflicted 가 repo-상대 row.path 를
  절대 탭 path 와 비교해 영구 false** — 병합 충돌 데코·region resolver·스테이징 차단 가드 3종이
  죽는 회귀. → isPathConflicted(row.absPath === path) 로 정정, conflict-status.ts 순수 추출+테스트 4건.
- **minor 6 수정**: search_replace 블로킹 I/O 를 spawn_blocking 안으로(begin_mutation_blocking 신설·원격
  경로 동기화)·service::replace 죽은 코드 제거(테스트 헬퍼 재조준)·commit-graph revert 충돌 파일 절대경로
  (RevertOutcome.conflicted_abs_paths 신설)·wait-marker localStorage 재시작 누적 → sessionStorage 세션
  경계(clearStaleWaitMarkersOnStartup)·terminal.md foregroundPid 오기술 정정. 부수: CommitFile.absPath
  bindings 기존 드리프트 정정.
