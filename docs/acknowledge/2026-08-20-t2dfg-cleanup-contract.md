# T2-D/F/G — 통합 저위험 청소 배치 계약 (2026-08-20)

> 정본: 감사 `2026-08-18-architecture-audit.md` §4.2-C8(접근성 8건)·§5 T2-D/F/G 행·§6.3 증분
> (T2-F +4·T2-G +2) + T2-I 계약 §4 의 미판정 6키(T2-F 잔여).
> 사용자 승인(2026-08-20 goal): "병합 계속하고 추천안대로 배치 계속진행해" — ① 핸드오프 커밋
> `282b4e1` prod 병합 완료(main=282b4e1) ② 추천안 = 본 배치(X-A 선례 형식: **항목별 실사 후
> 유효분만 수정** — 감사 주장을 전제하지 않고 실코드로 재판정, 기처리·무효 판정은 근거와 함께
> 기록만).
> 착수 전 메인 실물 재확인(2026-08-20, d-20 기록 + 본 세션 재실사): role='button' 계열 **7파일**
> 검출(d-20 "6파일" + commit-graph 조건부 role) — 키보드 전무 4종 onKeyDown 부재 전건 실측·
> Enter만 4지점 전건 실측(`event.key === 'Enter'` 만) / `getLocale` 소비처 0 실측 /
> `entities/app` 3파일은 **소비 체인 실재**(app-file-pane ← pane-node-view·ipc-sync-provider —
> 감사 "dead" 주장과 어긋남) / 레인 색 12 는 **3중 정의**(graph-lanes.ts:20·commit-graph.tsx:42·
> Rust git/types.rs:4 `GRAPH_LANE_COLOR_COUNT`) / R4#16 git types 상수 2개 =
> `GRAPH_LANE_COLOR_COUNT`·`LOG_PAGE_SIZE` 특정 / 폰트 범위 드리프트(R5#4)는 **기처리 정황**
> (settings/service.rs:177-180 doc 이 T1-B 통일 근거 서술·clamp 실재·프론트 min/max 검출 0) /
> `DEFAULT_FONT_SIZE` 는 `shared/constants/terminal.ts:9` 1곳만 검출(감사 "중복 정의" 부정확
> 정황) / explorer 행 메트릭은 `FILE_TREE_ROW_HEIGHT_PX` 상수화 기존재 / highlightLimit 1000·
> DNS 253 리터럴 실재.
> **기처리 4건 범위 제외 확정**: app:ready(X-A)·Project.capabilities(T1-I)·
> resolve_terminal_path(X-A 소비자 생김)·미참조 로케일 키 32(T2-I).

## 1. 범위

### 1.1 T2-D 접근성 (C8, 8건 — 5배치 출처)

- **키보드 활성화 전무 4종**(F2#4·F3#9·F4#17): `features/git/status-row-item.tsx`·
  `widgets/file-history/file-history-panel.tsx`·`widgets/git-panel/commit-detail-panel.tsx`·
  `widgets/git-panel/commit-graph.tsx` — Enter+Space 활성화 추가.
- **Enter 만 처리(Space 누락) 4지점**(F2#7): `features/problems/problem-row.tsx:31`·
  `features/search/search-match-row.tsx:30`·`features/outline/outline-symbol-row.tsx:43`·
  `shared/ui/file-group-header.tsx:23`.
- **공용 헬퍼**: 위 8지점이 같은 로직(Enter/Space → activate)을 공유하므로 감사 권고대로
  `shared/lib` 헬퍼 1회 추출(2회 이상 룰 충족). Space 는 스크롤 방지 `preventDefault` 포함
  (WAI-ARIA button 패턴). 기존 마우스 동작·포커스 흐름 무변경이 절대 조건.
- **구조 위반**(F7#9): `FileGroupHeader` 의 `div[role=button]` 안 Checkbox 중첩 — 중첩 인터랙티브
  해소(구현 실사로 최소 수정 형태 결정 — 마크업 재구성 시 기존 클릭·토글 동작 보존).
- **ARIA 미충족**(F2#8·F7#14): color-picker 슬라이더 2종(role=slider 필수 속성 —
  aria-valuemin/max/now)·overlay-scrollbar `role=scrollbar` 필수 속성(aria-controls·
  aria-valuenow) — 실사 후 WAI-ARIA 패턴 최소 충족.
- **label 부재 4곳**(F5#13): theme-editor·keybindings-editor·new-snippet-file-dialog 입력 —
  label 또는 aria-label 부여. **문자열은 로케일 카탈로그 경유**(하드코딩 금지 — i18n 4곳 동기).
  T2-I 미판정 6키 중 `agent.badgeAriaLabel`·`themeEditor.preview*` 5키가 이 작업의 예약분인지
  실사 — 소비 성립 시 사용(미판정 해소), 무관 확정 시 제거(4곳 동기 삭제).

### 1.2 T2-F dead code · 네이밍 (실사 대상 11항목 — 기처리 4건 제외)

- `entities/app` 3파일(app.ipc·app.query·app.type): **소비 체인 실재 실측** — dead 아님 정황.
  체인 전체(app-file-pane 의 위젯 소비 포함)의 실제 도달 여부만 판정해 기록(무효 판정 유력).
- `getLocale`(entities/locale/locale.ipc.ts:6): 소비처 0 실측 — 제거.
- `WindowStore::new`(window/commands.rs): `Self::default()` 위임뿐 — 호출자 실사 후 default 로
  일원화 또는 유지 근거 기록.
- `SYSTEM_FONT_FAMILY_VALUE`(features/settings/font-picker.tsx:11): 동일 파일 내 사용 실재 —
  dead 는 export 여지뿐. 외부 소비처 0 이면 export 만 제거.
- `graph-lanes.ts` `color` 필드(:14·:66): commit-graph 가 자체 `laneColor(lane)` 계산 —
  `color` 소비처 0 이면 필드 제거(테스트 동반 갱신).
- `OpenWithOverride.'preview'`(entities/editor/open-with-registry.ts:1): 'preview' 대입 지점
  실사 — 도달 불가 변형이면 제거(타입·소비 분기 동반).
- `overwrite` 오도 네이밍: 감사 압축으로 대상 특정 소실 — grep 실사로 식별 시도, **식별 불가면
  수정하지 않고 보고**(추측 수정 금지).
- `usize::MAX` 센티널(layout/service.rs:625·745·769): `move_tab` append 의미 — 명명 상수
  (`APPEND_AT_END` 계열) 또는 Option 정리 중 국소 쪽 선택(동작 무변경).
- `NO_MATCH_SCORE` 재사용(shared/lib/fuzzy-match.ts): 인덱스 센티널(-1)로 재사용 — 의미 분리
  (indexOf 부재 비교는 별도 명명 또는 직접 비교로).
- R4#16 git types 상수 2개(git/types.rs:4-5 `GRAPH_LANE_COLOR_COUNT`·`LOG_PAGE_SIZE`):
  Rust 측 소비처 실사 — 소비처 0 이면 제거. 단 `GRAPH_LANE_COLOR_COUNT` 는 §1.3 레인 색
  정본화와 연동(TS 2중 정의의 정본 후보) — 제거가 아니라 정본 승격이 나을 수 있음(실사 판단).
- T2-I 미판정 6키(`agent.badgeAriaLabel`·`themeEditor.previewAnsiTitle`·
  `themeEditor.previewChromeTitle`·`themeEditor.previewGitStatusSample`·
  `themeEditor.previewSidebarTitle`·`themeEditor.previewSyntaxTitle`): §1.1 label 작업과 연동
  판정 — 소비 성립/제거 양단 중 하나로 해소(제거 시 3 JSON+MESSAGE_NAMESPACES 4곳 동기).

### 1.3 T2-G 매직넘버 · 상수 정본화 (실사 대상 6항목)

- DNS 63/253(features/settings/remote-allowed-hosts-row.tsx:37-38): 253 리터럴 + 63
  (`isValidAllowedHostLabel` 내부 실사) — RFC 1035 근거 명명 상수화.
- 폰트 범위 8~32 vs 6~48 드리프트(R5#4): **기처리 정황(T1-B)** — 재확인 후 기처리 판정 기록만
  (잔여 드리프트 발견 시에만 수정).
- 터미널 highlightLimit(features/terminal/terminal-view.tsx:167): `1000` 명명 상수화.
- explorer 행 메트릭: `FILE_TREE_ROW_HEIGHT_PX` 상수화 기존재 — 감사가 지목한 잔여 메트릭
  (들여쓰기 폭 등) 실사 후 유효분만.
- 레인 색 12 정본화: 3중 정의(graph-lanes.ts:20·commit-graph.tsx:42·git/types.rs:4) 를 단일
  정본으로 — TS 2곳은 `shared/constants` 또는 graph-lanes 한 곳으로 통일. Rust 상수와의 교차
  언어 동기는 파리티 테스트 선례(테마 토큰) 또는 한쪽 소거 중 실사 판단(bindings 로 상수는
  전사되지 않음 — 수동 동기면 근거 doc).
- `DEFAULT_FONT_SIZE` 중복 정의: 1곳만 검출 — 재실사 후 **무효 판정 유력**(기록만).

### 1.4 범위 외

- T2-A 중복 제거·T2-B 분해·T2-C 재구조화·T2-E/J AppError 캠페인. 접근성 심화(포커스 트랩·
  스크린리더 전수 — Radix/shadcn 기본 활용 원칙 유지, 이번 배치는 감사 지목 8건만).
- 커맨드 표면·이벤트·원격 정책·와이어 무변경(커맨드 176+raw3·이벤트 23·ALLOWED 160⊎DENIED 19).
- 도메인 경계 화이트리스트 무변경(신규 도메인 간 참조 없음 — domain_boundaries.rs 그린 유지).

## 2. 실행 구조

- **단일 구현 에이전트(sonnet+xhigh, Workflow)**: TS·Rust 혼재 소규모라 순차 단독(Rust 한 시점
  한 에이전트 규칙 자동 충족). 항목별 **실사 → 판정(유효/기처리/무효/식별불가) → 유효분만 수정**
  — 판정 근거 전수 기록이 산출물의 절반. git stash 금지. Rust 변경 시 cargo fmt.
- **Phase E 검토(별도 Workflow, 배치 특성 재구성 4렌즈 opus+xhigh)**: ① 실사충실성(각 항목
  판정의 독립 재검증 — 감사 주장 vs 실코드, 무효·기처리 판정의 근거 실증) ② 접근성 규약(WAI-ARIA
  button/slider/scrollbar 패턴 부합·Space preventDefault·중첩 인터랙티브 해소의 적정) ③ 정확성
  (기존 마우스·포커스·클릭 동작 무변경, dead code 제거의 소비자 0 재확인 — 동적 참조 포함) ④
  계약(표면 무변경·bindings·i18n 4곳 동기·en⊆required·domain_boundaries 그린) → 적대적 검증
  (opus+high, major 이상 건별) → confirmed 수정 → 메인 2차(스팟 실물+verify·vite 직접 재실행)
  → 커밋(dev) → prod 병합.

## 3. 완료 조건

- `bun run verify` + `bunx vite build` 그린. bindings 무변경(Rust 상수 제거 시 diff 확인 —
  상수는 전사 안 되므로 무변경 기대). 표면 파리티 전부 그린.
- 항목별 판정표(유효/기처리/무효/식별불가/보류 + 근거) 전수 기록(계약 §4 에).
- 로케일 키 증감 시 3 JSON+MESSAGE_NAMESPACES 4곳 동기 + 기존 파리티 테스트 그린.
- 실기 이월(qa6): 8지점 키보드 활성화(Tab 이동+Enter/Space)·스크린리더 라벨 낭독·테마 에디터/
  키바인딩 에디터/스니펫 다이얼로그 입력 라벨 — 실기 몫 명시.

---

## 4. 구현 완료 기록 (2026-08-20, Phase E 검토 전)

> 구현 wf_31e24384-ef3(단독 sonnet+xhigh, 41파일 +199/−99) 완료. 판정 전문(항목별 evidence)은
> 스크래치패드 `t2dfg-impl-report.json` 보존. 메인 2차: 스팟 실물 재검증(헬퍼·8지점 적용·
> 로케일 776×3·entities/app 잔여 참조 0·OpenWithOverride 좁힘·APPEND_AT_END·git/types 상수
> 소거·신규 키 ko/ja 실번역) + `bun run verify`·`bunx vite build` 메인 직접 재실행 exit 0·
> bindings 무변경. 보고 filesChanged 중 agent-status-badge.tsx 는 실변경 없음(과기재 — 코드
> 문제 아님).

**판정 집계: fixed 21 · already-handled 1 · invalid-claim 1 · unidentifiable 1.**

- **T2-D(8건 전부 유효·수정)**: `shared/lib/activation-key.ts`
  `createActivationKeyDownHandler` 1회 추출(Enter+Space·Space preventDefault·테스트 3케이스)
  → 키보드 전무 4종+Enter만 4지점 8곳 적용(commit-graph 는 조건부 role 과 동일 조건 유지).
  FileGroupHeader 중첩 해소(바깥 div role 제거·토글 대상만 안쪽 role=button — 체크박스 형제
  분리, 마우스·포커스 순서 보존). color-picker SV/Hue 에 role=slider+aria-value*+화살표 키보드
  조작(순수 추가). overlay-scrollbar 에 aria-controls(useId)+aria-valuenow
  (`scrollbar-metrics.ts` `computeScrollPercent` 추출+테스트 — 기존 measure() 경로 병행 갱신).
  입력 label 4곳 은 기존 placeholder 키 재사용(신규 키 0). **T2-I 미판정 6키 전량 해소**:
  previewAnsiTitle·previewSyntaxTitle 2키 소비 성립(theme-live-preview role=group aria-label)
  / agent.badgeAriaLabel(badge 는 의도적 aria-hidden — 부모 button 라벨 중복 방지)·
  previewChromeTitle·previewGitStatusSample·previewSidebarTitle(해당 미리보기 섹션 미실재)
  4키 제거.
- **T2-F**: `entities/app` 3파일 — **계약의 "무효 유력" 예상 뒤집힘**: 착수 전 실사의 소비
  체인은 별개 슬라이스 `entities/app-file`(app-file-pane 실소비)과의 혼동으로 판명, entities/
  app(AppInfo 조회)은 외부 소비처 0 — 감사 원 주장이 정확. 3파일 삭제+연쇄 dead
  QUERY_KEY.APP 블록·테스트 분류표 동반 제거. getLocale 제거·WindowStore::new 삭제(default
  일원화 기존재)·SYSTEM_FONT_FAMILY_VALUE export 제거·graph-lanes color 필드+LANE_COLOR_COUNT
  제거(소비처 0 — 테스트 갱신)·OpenWithOverride 'editor' 로 좁힘('preview' 대입 0·소비 분기
  도달 불가)·usize::MAX → `APPEND_AT_END` 명명 상수(3곳·동작 무변경)·NO_MATCH_SCORE →
  `NOT_FOUND_INDEX` 개명(점수 아닌 인덱스 센티널)·R4#16 git/types.rs 상수 2개 제거(Rust 소비
  0·specta 미부착·프론트는 독립 상수가 실구동 — "유령 상수"). **unidentifiable 1**: overwrite
  오도 네이밍 — 전수 grep 약 25건 전건 의미 부합·감사 원문 추가 단서 부재, 추측 수정 안 함.
- **T2-G**: DNS 63/253 → `DNS_LABEL_MAX_LEN`·`DNS_HOSTNAME_MAX_LEN`(백엔드 동명 상수 미러·
  RFC 1035 JSDoc). highlightLimit → `SEARCH_HIGHLIGHT_LIMIT`. explorer 잔여 메트릭 =
  `ROW_INDENT_PX`·`ROW_ICON_SIZE_CLASS` 2파일 중복 정의 → file-tree-row export 로 통일.
  레인 색 12 → T2-F 두 제거로 commit-graph.tsx 1곳 자연 단일화(Rust 소거 방식 — 색상이
  데이터 모델에 없어 Rust 강제 지점 부재·상수는 bindings 미전사라 파리티 검증 대상 없음).
  **already-handled**: 폰트 범위 드리프트(T1-B — service.rs rustdoc 이 통일 결정 서술·잔여
  드리프트 0 재확인). **invalid-claim**: DEFAULT_FONT_SIZE 중복 정의(전역 1곳만 실재).
- **로케일 증감**: −4 +2 = 776키×3언어(신규 2키 ko/ja 실번역·MESSAGE_NAMESPACES 동기·기존
  파리티 그린).
- **범위 외 발견(보고만·미수정)**: `shared/constants/terminal.ts:7` MIN_FONT_SIZE 소비처 0 /
  `entities/theme/theme-tokens.ts` COLOR_NAMESPACES graph 네임스페이스가 lane1..12 를 별도
  나열(레인 수의 잠재 4번째 정본 — ref* 토큰 혼재로 단순 유도 불가·후속 판단).
- **검증**: verify exit 0(bun 1382·cargo 전량 그린)·vite build exit 0·bindings 무변경.
  신규 테스트: activation-key 3·scrollbar-metrics(computeScrollPercent) 4·query-key/
  graph-lanes 갱신.
