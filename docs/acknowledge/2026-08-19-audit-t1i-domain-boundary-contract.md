# 감사 T1-I — 도메인 경계 재조립(C13) 계약 (2026-08-19)

> 정본: 감사 `2026-08-18-architecture-audit.md` §4.2-C13(12건·30엣지)·§6.2 T1-I·§9 결정 10/12.
> 사용자 승인(2026-08-19): prod 병합 완료(main=353d590) + T1-I 착수 + **결정 10-A(코드를
> architecture.md:77 규칙에 맞춤)·12-A(`Project.capabilities` attach/detach 확장점 실현) 확정**
> ("병합 잘 하고 전부 추천대로 계속 진행해봐"). 위험 재고지 승인 포함(30엣지·순환 절단·15~20파일·
> 위험 중~높음 — T1 중 최대급 diff).
> 착수 전 메인 실물 재확인(전건 감사 상태 유효): 도메인 간 참조 전수 grep 실측 — project→
> ide·git·tree·terminal·layout·file·agent / settings→remote·ide·ai·theme·sync·agent / sync→
> settings·locale·theme·ai / system→terminal·lsp·agent(Store 직접) / terminal→ide / ide→
> layout(10)·project·plugin·file·remote / git·file→plugin / plugin↔vsix 양방향 / ide/server.rs 가
> `layout_commands::layout_open_tab`(:271)·`close_tab_and_finish`(:509·528)·`plugin_commands::
> ensure_loaded`(:286·370)·`super::commands`(:16) 호출 / ide→file full path(commands.rs:457-458) /
> infra/language.rs:3 → `domain::plugin::types::LoadedPlugin` 계층 역방향. `ProjectCapability`
> trait 부재(grep 0)·`Project.capabilities: Vec<CapabilityKind>` 필드만 존재(R4#9 스텁 유효).
> **내부 broadcast 이벤트 버스 부재 확인** — events.rs 는 tauri_specta IPC 이벤트(프론트행) 전용.
> architecture.md:77 의 "이벤트 버스 또는 상위 조립부" 중 실존 기구는 조립부(lib.rs setup)뿐.
> project_open/close 수기 조립 순서 실측(close 는 correctness-sensitive — dirty layout flush 가
> removal 선행·terminal kill_project·store evict 순서, doc 명시).

## 1. 범위

### 1.0 규칙 해석 명문화 (결정 10-A 의 실행 기준)

architecture.md:77 "도메인 간 직접 호출 금지"의 판정 기준을 다음으로 명문화(architecture.md 갱신):

- **금지**: 도메인 간 함수 호출(`commands::`·`service::`·`hooks::` 등 실행 경로)과 타 도메인
  Store 타입 직접 참조(`app.state::<XxxStore>()` 포함).
- **허용**: 직렬화 데이터 타입(`types.rs`) 참조 — entities 성격. 감사 C13 표의 12건도 전부
  실행·Store 계열이며 타입 참조는 지목하지 않음.
- **infra → domain 은 타입 포함 전면 금지**(계층 역방향 — R4#6). domain → infra 는 trait 경유
  (기존 규칙 유지).
- 불가피 엣지(§1.4 sync 등)는 화이트리스트로 명시 승인하고 §1.5 테스트가 그 목록을 기계 강제.

### 1.1 `ProjectCapability` attach/detach 확장점 실현 (결정 12-A — 핵심 기구)

- architecture.md §3 선언대로 attach(project open)/detach(project close) 확장점을 실현. 각
  도메인이 자기 자원의 부착·회수를 스스로 소유하고, 조립부(lib.rs setup)가 구현체를 등록,
  `project_open`/`project_close` 는 등록된 capability 를 순회 호출만 한다.
- 이관 대상(project/commands.rs 의 수기 조립 겸업 — R4#5): open 의 layout load·watcher 2종
  attach·ide refresh_lockfile·agent reconcile_installed_hooks / close 의 dirty layout flush·
  layouts/watchers/git_watchers remove·TerminalStore kill_project·GitStore/TreeStore evict·
  ide refresh_lockfile. C14(자원 회수 누락)의 기구이기도 하나 **이번 배치는 기존 회수 항목의
  이관만** — 신규 회수 추가(C14 잔여)는 범위 외.
- **절대 조건: 동작 무변경** — 기존 수기 호출과 동일 순서·동일 잠금 문맥(`begin_mutation` guard
  하)·동일 이벤트 발행 타이밍(ProjectOpened/Closed/Activated/list-changed 는 capability 순회
  후 기존 위치 유지). close 의 correctness-sensitive 순서(flush 선행 등, commands.rs doc)를
  capability 순서 규약으로 보존하고 그 근거를 doc 으로 이전.
- `Project.capabilities: Vec<CapabilityKind>` 필드가 실소비를 얻는 형태(capability 등록·조회와
  정합)로 정리 — 과설계 금지: 동적 플러그인식 레지스트리가 아니라 조립부 정적 등록으로 충분.

### 1.2 ide 도메인 정리 (R6#3·R6#11·R6#2)

- **제2 커맨드 진입점 제거**: ide/server.rs 의 `layout_commands::layout_open_tab`·
  `close_tab_and_finish`·`plugin_commands::ensure_loaded` 호출을 service 경유로 대체(R6#3).
  커맨드 함수에만 있는 부수효과(이벤트 발행 등)가 있으면 service 로 내려 동등성 보존 — 커맨드는
  얇은 래퍼가 된다(architecture.md 기존 규칙 정합).
- **ide commands ↔ server 양방향 순환 절단**(R6#11): server.rs:16 의 `super::commands` 의존과
  commands→server 역방향을 정리(공유 상태·타입은 별도 모듈로 하강).
- **ide → file 가드 누락**(R6#2): commands.rs:457-458 의 `file::service::save_file`/
  `clear_mirror` 직접 호출을 root guard 경유 경로로 정리(보안 — 기존 file 커맨드와 동일 검증).

### 1.3 순환·계층 역방향 절단 (R7#4·R4#6)

- **plugin ↔ vsix**: 실행 경로 상호 참조를 단방향으로(공유 로직은 한쪽 소유 또는 shared 하강).
- **infra/language → domain::plugin 제거**: 경량 `LanguageOverlay` 타입을 infra 측에 정의하고
  domain::plugin 이 그것으로 변환해 전달(감사 제안 그대로 — 의존 방향 반전).

### 1.4 잔여 명령형 엣지 정리

- **settings → agent·ide·remote**(R5#6): 같은 함수가 `SettingsChanged` 를 발행하면서 명령형
  호출도 함 — 조립부 경유 또는 capability/구독 지점으로 이관(선택은 구현 판단·근거 기록).
- **system → terminal·lsp·agent Store 직접 참조**(R8#9): 라벨·통계 조회를 조립부 경유로.
- **terminal → ide**(R8#10): 의존 정리(spawn 의 최대 2초 폴링 지연 구조 개선이 자연스러우면
  포함, 대형 재설계면 보고 후 이월).
- **git·file → plugin::commands**(R4#6): service 하강 또는 조립 경유.
- **sync → ai·locale·settings·theme**(R5#14): 집계 도메인 — 감사가 "일부 불가피" 판정. 실사 후
  불가피분은 화이트리스트 등재(사유 doc), 가피분만 정리.
- 감사 미지목 실측 엣지(layout→ide·app→settings 등)는 **동일 원칙으로 실사·분류만**(정리가
  국소면 포함, 아니면 화이트리스트+후속 보고 — 범위 폭주 방지).

### 1.5 기계 강제 (C12 연계 — T1-K 파리티 선례)

- **도메인 경계 아키텍처 테스트 신설**: 소스 스캔(include_str! 정규식 — lib.rs·T1-K arm 파리티
  선례)으로 도메인 간 `commands::`/`service::`/Store 참조를 검출, §1.0 화이트리스트 밖이면 실패.
  infra→domain 역방향도 검출. **신규 위반은 화이트리스트 등재 없인 테스트 실패**(C13 재발 방지
  — T1-K "미등재=거부"와 동형). 인위 위반 주입으로 테스트의 테스트 1회 실측.

### 1.6 범위 외

- C14 신규 자원 회수 추가(pty Drop 이중화 등 — §9 결정 9 는 T1-J 몫)·T1-H 락 IO·T2 전체.
- 제품 결정 3건(ide 브로드캐스트 게이트·agent_hooks 대칭화·키링 게이팅 — 기결 이월).
- 커맨드 표면·와이어 프로토콜·원격 정책 변경 일절 없음(ALLOWED 160 ⊎ DENIED 19·커맨드 176+
  raw3·이벤트 23 무변경이 완료 조건).

## 2. 실행 구조

- **Rust 전용 배치 — 한 시점 한 에이전트, 순차 3단(sonnet+xhigh)**:
  - R1 = §1.1 확장점 신설 + project/commands.rs 이관(기구 확립 — 후속 단이 소비).
  - R2 = §1.2 ide 정리(제2 진입점·순환·가드).
  - R3 = §1.3 + §1.4 실사·정리 + §1.5 아키텍처 테스트 + 문서(architecture.md §2/§3 규칙
    명문화·ipc-contract 무변경 확인·data-model 해당 시) + 전체 verify.
  - 각 단 완료 시 cargo fmt·clippy·test 그린 후 다음 단. **git stash 금지 명시**.
- **메인 통합 확인**: 접합부·bindings diff(무변경 기대)·verify·vite build 직접 재실행.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 정확성: open/close 순서·잠금 의미론
  보존(디바운스 flush·kill_project 타이밍)·server service 전환의 부수효과 동등성 / 설계:
  capability 추상화 적정선(과설계·과소설계)·순환 절단 방식 / 계약: 아키텍처 테스트 실효성·
  화이트리스트 최소성·표면 무변경 전수 / 보안: ide→file 가드·원격 정책 무변경) → 적대적
  검증(opus+high, major 이상 건별) → confirmed 수정 → 메인 2차(스팟 실물+verify 직접) →
  커밋(dev).

## 3. 완료 조건

- `bun run verify` 전체 + `bunx vite build` 그린. bindings 무변경(변경 시 사유 보고).
  커맨드 176(+raw3)·이벤트 23·ALLOWED 160⊎DENIED 19·기존 파리티 전부 그린.
- 아키텍처 테스트 그린 + 인위 위반 실측 1회. 화이트리스트 각 항목에 사유 doc.
- project open→close→재오픈 시퀀스 회귀 테스트(capability 순회가 기존 수기 조립과 동일 자원
  상태를 만드는지 — 기존 테스트 유지 + 필요 시 보강).
- 실기 이월(qa6): 프로젝트 열기/닫기/재열기·멀티 프로젝트 전환·ide 서버 경유 탭 열기/닫기
  회귀 무변화.

---

## 4. 구현 완료 기록 (2026-08-19, Phase E 검토 전)

> 구현 wf_e4ab6095-929(R1→R2→R3 순차 단독, 전원 sonnet+xhigh) 완료. 메인 2차: 스팟 체크
> 전건 실물 일치(신규 4파일·project/commands 타 도메인 참조 0·ide server→commands 0·아키텍처
> 테스트 3종+stale 검출·LanguageOverlay 반전·plugin→vsix 소거·ALLOWED 160⊎DENIED 19 무변경)
> + `bun run verify`·`bunx vite build` 메인 직접 재실행(결과는 §4.5).

- **R1(§1.1)**: `project/capability.rs` — `ProjectCapability` trait(동기·무오류 — 기존 수기
  호출이 전부 log-warn 삼킴형인 실코드에서 도출, architecture.md §3 의 async/Result 선언과
  다름은 문서 갱신으로 정합) + 정적 레지스트리. capability 구현 8종을 각 도메인 소유로
  (layout·file watcher·git watcher/cache 분리 2종·terminal·tree·ide lockfile·agent hooks).
  단일 리스트 전방 순회로 close 의 correctness-sensitive 순서 바이트 재현 + **등록 순서 핀
  테스트**·detected_kinds↔service 파리티 테스트(§5 에서 단일 출처화 동작 고정 테스트로 대체).
  `Project.capabilities` 는 `GitWatcherCapability::attach` 의 `contains(Git)` 게이트로 실소비
  (§5 D1 수정 후 성립 — 구현 시점 기록은 명목 소비였음이 검토로 확인됨). 부트 복원 경로는 순회
  미적용(hooks 무조건 spawn 등 동작 변경 방지 — 기존 부분 부착 보존). project/commands.rs
  **타 도메인 참조 0**. `project_close` 의 Store State 파라미터 제거(와이어 표면 아님).
- **R2(§1.2)**: `ide/store.rs` 하강(commands 455→240줄)으로 commands↔server 순환 절단.
  server 의 layout/plugin commands 호출 → service 경유(`layout/service.rs` 오케스트레이션 절
  — `close_tab_and_finish` 순수 이동+`open_tab_and_finish` 신설, `plugin/service.rs` 로
  Store+`ensure_loaded` 하강 — 부수 효과로 git·file→plugin::commands(R4#6)도 해소).
  **R6#2 실사 정정**: root guard 는 기존재(`ensure_path_within_any_project`) — 실제 결손은
  `begin_mutation` 가드 부재·`self_writes.mark` 부재(IDE 저장이 외부 변경 오인)·file_save
  본문 복제. `file::service::save_file_within_open_projects` 신설(가드→저장→mark→미러 정리
  공유 경로)로 셋 다 해소 + 회귀 테스트.
- **R3(§1.3~1.5·문서)**: plugin↔vsix — 공유 `extract_hardened_zip` 을 `infra/archive.rs` 로
  하강(한쪽 소유는 반대 엣지를 남김)해 vsix→plugin 단방향만 잔존. `infra/language.rs` —
  `LanguageOverlay` 정의·`LoadedPlugin` 참조 제거(의존 반전, 소비부 6파일 변환 주입).
  조립부 배선 3종(전부 R1 과 동형의 정적 등록 — listen_any 구독안은 토글 반응이 커맨드 반환
  후로 밀리는 타이밍 변화라 기각): `SettingsToggleObservers`(R5#6 — 인라인 await 유지로
  타이밍 동일)·`SystemUsageLabelProviders`(R8#9 — terminal→agent→lsp 등록 순 = pid 덮어쓰기
  우선순위 보존)·`PtySpawnEnvProvider`(R8#10 — 대기 로직·상수를 ide 소유로 이관, 2초 폴링
  구조는 유지·watch 채널화 이월). `SettingsPatch`→settings/types.rs 등 타입 자리 정정 2건.
  **아키텍처 테스트 3종**(도메인 간 기본 거부+화이트리스트 24건 각 사유 doc / infra→domain
  4건 / bare import 는 dispatch 게이트웨이 전용) + **stale 등재도 실패**(화이트리스트 부패
  방지) + 인위 위반 실측(FAILED→원복→touch→그린). architecture.md §2 판정 기준·§3 실현
  시그니처·§6.3 갱신.
- **메인 처리**: ipc-contract.md 어긋난 문구 4곳 갱신(:393·:1169 SettingsToggleObservers·
  :770·:1108 infra::archive). bindings 22+/21- 는 전부 rust doc→JSDoc 전사(invoke 이름·
  파라미터·타입·커맨드 176+raw3·이벤트 23 무변경) — 사라진 수기 조립을 서술하는 거짓 doc
  복원은 무의미하므로 **갱신 유지 승인**.
- **이월·후속(보고)**: window↔layout 실행 경로 순환(탭↔OS창 이동이 두 소유자 가로지름 —
  양방향 화이트리스트 등재, 절단은 대형이라 후속 배치 후보) / infra→domain types 4건
  화이트리스트 유지(반전 후속 후보 — mask_provider_error 는 §5 에서 infra 하강 완료) /
  terminal spawn 2초 폴링 구조(소유만 ide 이관, watch 화 이월).
- **Phase E 렌즈 초점(구현 자기 표기 포함)**: 설정 토글 타이밍 동일성(관찰자 인라인 await)·
  usage 라벨 pid 우선순위·`claude_terminal_env` 락 문맥·`ide_resolve_diff` 의
  begin_mutation+self_writes.mark 추가가 의도 승인 범위인지·open/close 순서 바이트 재현
  전수 대조·화이트리스트 최소성.

---

## 5. Phase E 검토 결함 수정 반영 (2026-08-19)

> 검토 wf_8a02c83f-46a(4렌즈 opus+xhigh, 대상 dev `1852820` diff) — 발견 17(major 2·minor 15)
> → 적대적 검증(opus+high, major 2 전건) **confirmed 2·refuted 0**(C-1 major 유지·D1 major→
> minor 강등 — 런타임 결함 아님·과대 기술 판정) → 수정 wf_5de1f040-63f(Rust 단독 fixer
> sonnet+xhigh, 11항목). 메인 2차: 스팟 5축 실물 재검증 + verify·vite build 직접 재실행.
> 정확성 렌즈는 open/close 순서·잠금 바이트 재현·부트 복원 부분 부착·server service 전환·
> 토글 타이밍·라벨 우선순위·락 문맥·재진입 데드락 전수 대조에서 **동작 동등성 결함 0** 확증.

- **[confirmed major·C-1+SEC-2] 경계 스캔 import 우회 봉쇄** — 스캔 정규식이 모듈 import
  (`use crate::domain::layout;`)·중괄호 그룹 2형·crate 레벨 그룹을 무음 통과시키던 것을
  게이트웨이 외 전면 금지로 확장(검증 verdict 가 추가 지적한 domain 레벨 그룹 포함 5형태).
  **인위 우회 4종 각각 FAILED 실측 후 원복**·as-별칭 포함 15케이스 정규식 단위 실측 오탐 0·
  기존 코드 신규 패턴 매치 0(화이트리스트 완화 없음). 과대 서술 doc 정정(`super::super::`·
  재export 잔존 명시).
- **[confirmed major→minor·D1] `Project.capabilities` 단일 출처화** — `open_project` 에
  `detect_capabilities` 주입(FnOnce), `project_open` 이 레지스트리 `detected_kinds` 를 주입해
  프로덕션 소비 확보. `GitWatcherCapability::attach` 는 무검사 `register_git_watcher` 를 불러
  `contains(Git)` 게이트가 open 경로의 유일 실판정이 됨. 부트 복원 전용 `attach_git_watcher`
  는 자체 `.git` 검사 유지(lib.rs 호출부 무변경 — 두 경로 동작 보존). `GIT_DIR_NAME` 2중
  정의 해소. 동작 고정 테스트 2종(git/비git 루트 기록값 명시·주입 결과 기록).
- **minor 실질 수정 9**: C-2 `for_each_in_registration_order` 단일 순회 지점+
  `RecordingCapability` 전방 순회 회귀 테스트(mock-app 부재 제약으로 공유 walker 고정 방식 —
  사유 doc) / T1I-E1 intra-doc 링크 2건+bindings 전사 / SEC-1 원시 `save_file` private 화
  (도메인 밖 호출자 0 실사 — 타입 시스템이 R6#2 재발 차단) / SEC-3 "동일 최종 맵" doc 을
  실동작(provider 별 스냅샷·등록 순서 우선 결정화)으로 정정 / D5(2) `mask_provider_error`
  클러스터 `infra/redact.rs` 하강(호출부 5파일·화이트리스트 엣지 제거)+D5(1)(3) 화이트리스트
  doc 에 제거 가능 대안·유지 근거 명기 / C-4 layout↔ide 순환 표기 추가 / D9 신뢰 아카이브
  추출기 3종 경고 doc / D10 `open_tab_and_finish` doc 영어화 / C-3 라이브 문서 4곳(ipc-
  contract:128·agent-integration 3곳·plugins:130) 실물 경로 갱신.
- **기각(수정 안 함·이월)**: D2(순회 리스트 3계약 인코딩 — 구조 재설계급) / D3(부트 복원
  레지스트리 우회 — R1 의 의도 결정: 전체 순회 시 hooks spawn 등 동작 변경) / D4(trait DI
  스타일) / D6(조립부 배선 4기구 형태 통합 — 재설계) / D7(ide 서버 수명주기 commands 잔존)
  / D8(**AppState 공유 필드 경유 도메인 간 접근은 규칙·스캔의 사각지대** — 후속 배치 재판단
  후보로 기록). refuted 0 이므로 수정 금지 대상 없음.
