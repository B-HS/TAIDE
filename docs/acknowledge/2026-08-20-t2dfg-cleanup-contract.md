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

> 구현 wf_31e24384-ef3(단독 sonnet+xhigh, 42파일 +241/−90 — 최초 기록 "41파일 +199/−99" 는
> 오기재, Phase E 검토 contract-5 로 정정) 완료. 판정 전문(항목별 evidence)은
> 스크래치패드 `t2dfg-impl-report.json` 보존. 메인 2차: 스팟 실물 재검증(헬퍼·8지점 적용·
> 로케일 776×3·entities/app 잔여 참조 0·OpenWithOverride 좁힘·APPEND_AT_END·git/types 상수
> 소거·신규 키 ko/ja 실번역) + `bun run verify`·`bunx vite build` 메인 직접 재실행 exit 0·
> bindings 무변경. 보고 filesChanged 중 agent-status-badge.tsx 는 실변경 없음(과기재 — 코드
> 문제 아님).

**판정 집계(Phase E 검토 audit-fidelity-1 반영 정정): fixed 21 · already-handled 2 ·
unidentifiable 1.** (최초 기록은 `already-handled 1 · invalid-claim 1` 이었으나 DEFAULT_FONT_SIZE
항목이 already-handled 로 재판정되어 invalid-claim 은 0건이 됐다 — 상세는 §5.)

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
  레인 색 12 → 계약이 지목한 3중 정의(graph-lanes.ts·commit-graph.tsx·git/types.rs)는 T2-F 두
  제거로 commit-graph.tsx 1곳으로 단일화(Rust 소거 방식 — 색상이 데이터 모델에 없어 Rust 강제
  지점 부재·상수는 bindings 미전사라 파리티 검증 대상 없음). **다만 이 "1곳 통일"은 `LANE_COLOR_COUNT`
  상수 자체에 한정된다 — CSS 변수 공급원(`entities/theme/theme-tokens.ts` COLOR_NAMESPACES.graph
  의 lane1..12 나열, `shared/styles/global.css` 의 `--taide-graph-lane1..12`)과 Rust 테마 서비스
  (`domain/theme/service.rs:181-192` 의 동일 lane1..12 나열+기본값)에는 "12" 가 여전히 별도
  열거로 남아 있고 어느 쪽도 서로의 개수를 검증하지 않는다(Phase E 검토 audit-fidelity-2/
  contract-6 — §5).
  **already-handled**: 폰트 범위 드리프트(T1-B — service.rs rustdoc 이 통일 결정 서술·잔여
  드리프트 0 재확인) · DEFAULT_FONT_SIZE 중복 정의(Phase E 검토 audit-fidelity-1 로 재판정 —
  §5. 감사 원문이 지목한 대상은 `settings-view.tsx:73` 지역 상수였고 이는 T1-B 커밋 6917978 에서
  이미 제거됨. 최초 기록의 "invalid-claim"(전역 이름 grep 만으로 판정)은 오판).
- **로케일 증감**: −4 +2 = 776키×3언어(신규 2키 ko/ja 실번역·MESSAGE_NAMESPACES 동기·기존
  파리티 그린).
- **범위 외 발견(보고만·미수정)**: `shared/constants/terminal.ts:7` MIN_FONT_SIZE 소비처 0 /
  `entities/theme/theme-tokens.ts` COLOR_NAMESPACES graph 네임스페이스가 lane1..12 를 별도
  나열(레인 수의 잠재 4번째 정본 — ref* 토큰 혼재로 단순 유도 불가·후속 판단) /
  **(Phase E 검토로 추가 확인)** `shared/constants/terminal.ts:9` `DEFAULT_FONT_SIZE=13` 과
  `shared/constants/code-font-size.ts:5` `DEFAULT_CODE_FONT_SIZE=13` 이 "터미널 폰트 크기
  기본값"을 이중 정의하고 소비처가 갈린다(`terminal-session.tsx:197` 는 전자, `settings-view.tsx:728`·
  `status-bar-content.tsx:54` 는 후자를 사용) — 현재 값이 같아 무증상이나 한쪽만 바꾸면
  드리프트가 재발한다(§5 audit-fidelity-1 이월) / `shared/styles/global.css` 의
  `--taide-graph-lane1..12`(24줄, light/dark) 와 `src-tauri/src/domain/theme/service.rs:181-192`
  의 lane1..12 나열+기본 색상값도 레인 수 "12" 의 추가 미보증 열거다(§5 audit-fidelity-2 이월).
- **검증**: verify exit 0(bun 1382·cargo 전량 그린)·vite build exit 0·bindings 무변경.
  신규 테스트: activation-key 3·scrollbar-metrics(computeScrollPercent) 4·query-key/
  graph-lanes 갱신.

## 5. Phase E 검토 결함 수정 반영 (2026-08-20)

> Phase E 검토(4렌즈 opus+xhigh + major 이상 적대적 검증, `t2dfg-review-report.json`)가 발견
> 24건(major 5·minor 19)을 냈고, major 5건 전건이 적대적 검증에서 confirmed(refuted 0)됐다.
> 아래는 그 결함 수정 반영 기록이다. 중복 축(같은 근본 원인을 4렌즈가 각자 이름 붙인 것)은
> 대표 id 로 묶었다.

### 5.1 major 5건 — 전건 수정

1. **activation-key 헬퍼 target 가드 부재**(`a11y-spec-1` = `correctness-1`, status-row-item 중첩
   인터랙티브 포함) — `shared/lib/activation-key.ts` `createActivationKeyDownHandler` 에
   `if (event.target !== event.currentTarget) return` 가드 추가(8지점 공통 적용, 자식의 네이티브
   컨트롤에서 버블링된 keydown 을 원천 차단). `status-row-item.tsx` 는 FileGroupHeader 선례를
   따라 재구성 — 바깥 `div` 는 role 없는 레이아웃/hover 컨테이너, 파일명·경로만 감싸는 안쪽
   `div[role=button]` 을 분리하고 상태 문자·액션 아이콘 그룹은 형제로 뺐다(중첩 인터랙티브
   해소). 액션 아이콘의 `IconButton onClick` 에 있던 `event.stopPropagation()` 은 더 이상 막을
   부모 onClick 이 없어 죽은 코드가 됐으므로 함께 제거(ai-process §6.8). **액션 span 의 키보드
   도달성**은 `group-hover:flex` 만으로는 키보드 전용 사용자가 도달할 수 없던 기존 결함이라
   최소 수정 원칙상 범위 밖이지만, 코드베이스에 이미 동일 패턴의 선례
   (`file-tree-toolbar.tsx:20` 의 `group-focus-within/explorer:opacity-100`)가 있어 **회복**
   시키기로 판단 — `group-focus-within:flex`(액션 span)·`group-focus-within:hidden`(상태 문자
   span, hover 쪽과 대칭)를 추가했다.
   테스트: `activation-key.test.ts` 에 target≠currentTarget 케이스 추가.
2. **color-picker SV 사각형 aria-valuenow 가 채도를 반영하지 않음**(`a11y-spec-2` = `contract-3`)
   — 실코드(단일 `role=slider` 유지, 2 slider 분리는 마크업·상태 관리 재설계가 필요해 이번
   배치 규모를 초과) 기준 최소·정확한 형태로 **단일 slider + `aria-valuetext` 동시 노출**을
   선택. `aria-valuetext={t('themeEditor.saturationValueValueText', { saturation, value })}` 를
   매 렌더(=매 값 변경)마다 재계산해 채도·명도 두 축을 문자열로 통지하고, `aria-orientation='vertical'`
   을 명시(aria-valuenow 가 나타내는 축이 명도=수직이므로 표시 축과 통지 축을 일치시킴). 신규
   로케일 키 `themeEditor.saturationValueValueText`(en/ko/ja 실번역) 1개, 4곳 동기 완료.
   **s=0 에서 hue 가 소실되는 hex 왕복 부수 결함**(같은 finding 의 부수 지적)은 `shared/lib/color.ts`
   의 `rgbToHsv`(delta=0 일 때 h=0 반환)가 원인인 **기존 결함**(이번 배치가 만들지 않음)이라
   수정하지 않고 보고만 한다 — 별도 후속 이월.
3. **color-picker 팝오버 자동 포커스 회귀**(`correctness-2`) — SV/Hue 에 `tabIndex={0}` 을 준
   순간부터 Radix `PopoverContent`(FocusScope)의 마운트 자동 포커스 대상이 hex 입력에서 SV
   사각형으로 바뀌는 회귀. `PopoverContent` 에 `onOpenAutoFocus` 를 지정해
   `event.preventDefault()` 후 `hexInputRef.current?.select()` 로 기존 동작(hex 입력 포커스+
   전체 선택)을 복원. 이를 위해 hex `<input>` 에 `ref={hexInputRef}` 추가.
4. **audit-fidelity-1 — DEFAULT_FONT_SIZE 오판정** — 코드 수정이 아니라 기록 정정. §4 판정
   집계를 `already-handled 2 · invalid-claim 0` 으로 정정(근거: 감사 raw 원문 R5#4 가 지목한
   대상은 `settings-view.tsx:73` 지역 상수였고 T1-B 커밋 `6917978` 에서 이미 제거됨 — 최초
   판정은 전역 이름 grep 만으로 "동명의 두 번째 정의 없음" 을 결론 내려 원 지목 대상을 놓친
   오판). 잔여 발견 2건을 §4 "범위 외 발견" 에 이월 등재: (a) `shared/constants/terminal.ts:9`
   `DEFAULT_FONT_SIZE` 와 `shared/constants/code-font-size.ts:5` `DEFAULT_CODE_FONT_SIZE` 가
   동일 필드(`settings.terminalFontSize`)의 폴백을 이중 정의하고 소비처가 갈림(현재 값 동일이라
   무증상, 드리프트 위험), (b) `terminal.ts:7` `MIN_FONT_SIZE` 소비처 0(기존 기록 유지).

### 5.2 minor 19건 — 실질/기각 분류

**실질(수정)**

- `audit-fidelity-4` = `correctness-6` = `contract-2` (overlay-scrollbar `aria-valuemax` 리터럴
  100) — `SCROLLBAR_SCROLL_PERCENT_MAX` import 후 `aria-valuemax={SCROLLBAR_SCROLL_PERCENT_MAX}`
  로 교체(1줄).
- `a11y-spec-4` (FileGroupHeader `aria-expanded` 부재) — 토글 `div[role=button]` 에
  `aria-expanded={expanded}` 추가. `aria-controls` 는 소비처 2곳(search-results-list·
  problems-panel)에 id 배선이 추가로 필요해 이번 항목의 핵심 결함(상태 미노출) 밖이라 보류.
- `a11y-spec-8` (aria-label 이 placeholder 와 완전 동일해 이름·설명 중복 낭독) — 4곳 실사 후
  둘로 나눠 처리. **검색 인풋 2곳**(`themeEditor.searchTokensPlaceholder`/
  `settings.keymapSearchPlaceholder`, 말줄임표 포함)은 라벨로 부적절한 결함(말줄임표)이 별도로
  확정돼 전용 aria-label 키를 신설: `themeEditor.searchTokensAriaLabel`("Search tokens")·
  `settings.keymapSearchAriaLabel`("Search commands or keys") — en/ko/ja 실번역, 4곳 동기,
  `theme-editor.tsx`·`keybindings-editor.tsx` 의 `aria-label` 을 새 키로 교체. **이름 인풋
  2곳**(`themeEditor.themeNamePlaceholder`/`snippetEditor.newFileGlobalNamePlaceholder`, 말줄임표
  없음)은 §5.2 기각 목록 참조.
- `a11y-spec-6` (overlay-scrollbar 이름·키보드 부재, aria-valuemax 리터럴) — 실사 결과 이
  트랙을 실제로 조작 가능하게 만들려면(키보드 스크롤) `ScrollContainer` 의 뷰포트 자체가
  애초에 포커스 불가(`tabIndex` 없음)라 이 오버레이만 손봐서는 "조작 가능한 위젯" 이 되지
  않는다 — 진짜 조작성을 주려면 뷰포트 포커스·키보드 스크롤까지 다시 설계해야 해 이번 저위험
  배치 범위를 크게 넘는다. 검토 원문이 스스로 제시한 대안(track 을 장식 요소로 명확화)을
  채택 — `aria-hidden='true'` 를 트랙에 추가해 "조작 불가능한데 role=scrollbar 로 위장" 하는
  상태를 없앴다. `role`/`aria-orientation`/`aria-valuemin`/`aria-valuemax` 자체는 남겨 뒀다
  (aria-hidden 하에서는 AT 에 노출되지 않아 무해하고, `use-overlay-scrollbar.ts`·
  `scrollbar-metrics.ts` 의 기존 aria-valuenow 갱신 경로·테스트를 건드리지 않기 위함). 초기
  프레임 aria-valuenow 공백 문제는 aria-hidden 으로 무의미해져 별도 조치하지 않는다.
- `a11y-spec-7` (color-picker Home/End 부재) — SV 사각형·Hue 슬라이더 keydown 핸들러에 Home(→
  최소)/End(→ 최대) 분기 추가(최소선 — PageUp/PageDown 등 추가 확장은 하지 않음).
- `a11y-spec-3` (activation-key Space 가 keydown 즉시 활성화, auto-repeat 시 반복 실행) — 가장
  실동작에 영향이 큰 결함(키를 누르고 있으면 콜백이 프레임 단위로 반복 호출)만 최소 수정:
  `event.repeat` 가드를 추가해 반복 keydown 에서는 `preventDefault()`(스크롤 방지)만 계속하고
  `onActivate()` 는 최초 1회만 호출. **Enter/Space 를 keydown/keyup 두 단계로 나눠 네이티브
  button 과 동일한 타이밍(Space 활성화를 keyup 으로 이동)을 구현하는 전체 APG 재작성은
  8지점의 이벤트 배선을 전부 바꿔야 하는 규모라 이번 저위험 정리 배치에는 과도한 확장으로
  판단해 보류**(후속 이월). 테스트 1건 추가(반복 keydown 시 콜백 1회만 호출·preventDefault 는
  계속 호출 확인).
- `contract-5` (계약 §4 변경 규모 수치 오기재, 41파일 +199/−99 → 실제 42파일 +241/−90) — §4
  수치 정정.
- `audit-fidelity-3` = `contract-4` (`entities/app`·`getLocale` 제거 후 ipc-contract.md 가
  구 용도·소비처를 그대로 서술) — `docs/ipc-contract.md` 의 `app_get_info`·`locale_get` 서술을
  "현재 프론트 소비처 0(백엔드·원격 표면만 유지)" 으로 갱신.
- `audit-fidelity-2` = `contract-6` (레인 색 12 "정본화 완료" 서술 과대 — CSS 변수·Rust 테마
  서비스 쪽 12 나열이 무보증으로 잔존) — 코드 변경이 아니라 계약 §4 서술 정정(3중 정의 해소는
  `LANE_COLOR_COUNT` 상수 자체에 한정됨을 명시) + `theme-tokens.ts`·`global.css`·
  `domain/theme/service.rs` 3곳을 §4 범위 외 발견에 이월 등재. 파리티 테스트 신설(예:
  `LANE_COLOR_COUNT` 를 `theme-tokens.ts` 의 graph 네임스페이스에서 유도)은 graph 네임스페이스에
  `ref*` 토큰이 섞여 있어 단순 필터링을 넘는 리팩토링이 필요해 이번 배치 범위 밖 — 후속 이월.

**기각(사유 명시)**

- `a11y-spec-8` 의 이름 인풋 2곳(`themeEditor.themeNamePlaceholder`·
  `snippetEditor.newFileGlobalNamePlaceholder`) — placeholder 문구가 "Theme name"/"File name" 형
  짧은 명사형이라 말줄임표 결함이 없고, 시각 라벨이 없는 인풋에 짧은 필드명을 placeholder+
  aria-label 로 겸용하는 것은 실무에서 흔히 쓰이는 저위험 패턴이다. 이름·설명 중복 낭독은
  실재하는 AT 동작이지만 정보를 그릇되게 전달하지는 않고, 전용 키 신설의 로케일 거버넌스
  비용(3언어 실번역+4곳 동기) 대비 개선 폭이 작아 기각한다.
- `a11y-spec-5` = `correctness-3` = `contract-1` (FileGroupHeader 클릭 영역 축소) — 실사 후
  **국소 수정 가능한 부분만 수정, 나머지는 수용된 편차로 기록**(아래 §5.3 참고). 전면 복원은
  체크박스가 role=button 의 형제로 남아 있는 한(중첩 인터랙티브 재발 없이) 불가능해 전건 기각은
  아니고 부분 수정 + 부분 수용.
- `correctness-4` (FileGroupHeader Tab 순서 반전) — 수용된 편차로 기록(§5.3). 코드 수정 안 함.
- `correctness-5` (FileGroupHeader Checkbox 의 무의미해진 `stopPropagation`) — 별도 조치
  불필요: 이번 배치의 `a11y-spec-5`/`contract-1` 클릭 영역 수정 작업 중 이미 제거됨(§5.2 실질
  항목에 포함되지 않은 이유는 별도 diff 가 아니라 같은 파일 재작성에 자연 포함됐기 때문).

### 5.3 수용된 편차 (코드 미수정, 사유 기록)

- **FileGroupHeader 클릭 영역** — 체크박스가 없는 경로(`problems-panel.tsx`)는 `pl-2`(좌측
  8px)를 안쪽 `role=button` div 로 옮겨 **완전히 복원**했고, 상하 `py-0.5`·체크박스-콘텐츠
  사이 4px(`gap-1` → 안쪽 div 의 `pl-1`)는 체크박스 유무와 무관하게 안쪽 div 소유로 옮겨
  **양쪽 경로 모두** 복원했다. 유일하게 남는 잔여 사각지대는 **체크박스가 있는 경로에서
  체크박스 앞의 좌측 8px(`pl-2`, 바깥 div 소유)** 뿐이다 — 이 부분을 마저 클릭 가능하게
  하려면 role=button 요소의 히트박스가 앞선 형제(체크박스)의 자리까지 시각적으로 겹쳐야 하는데,
  일반 flex 흐름에서는 절대 위치·오버레이 기법 없이는 불가능하다. 오버레이 도입은 stacking·
  포인터 이벤트 우선순위 리스크가 있는 더 큰 변경이라 이번 저위험 배치에서는 보류하고, 8px
  잔여 사각지대를 수용된 편차로 기록한다.
- **FileGroupHeader Tab 순서 반전**(행 토글 → 체크박스, 에서 체크박스 → 행 토글로) — 중첩
  인터랙티브를 해소하려면 체크박스가 `role=button` div 의 자손이 아닌 형제여야 하고, 그 결과
  DOM 순서상 체크박스가 먼저 오는 한 원래 순서(행 먼저)는 자연히 뒤집힌다. DOM 순서를 그대로
  두고 CSS `order` 로 시각/포커스 순서만 되돌리는 방법은 최신 스펙에서 포커스 내비게이션이
  `order` 값을 따르도록 개정됐지만 Tauri 가 구동하는 WebKit(macOS)/Chromium(Windows) 두
  웹뷰 엔진 간 지원 시점이 달라 플랫폼별로 결과가 갈릴 위험이 있다. 두 컨트롤 모두 여전히
  Tab 으로 도달 가능하고 활성화도 정상 동작하므로(순서만 바뀜), 안전성이 불확실한 트릭을
  쓰기보다 순서 반전을 수용된 편차로 기록한다.

### 5.4 후속 이월 (범위 외 — 코드 변경 없음)

1. `shared/constants/terminal.ts:9` `DEFAULT_FONT_SIZE` vs `code-font-size.ts:5`
   `DEFAULT_CODE_FONT_SIZE` 잔여 이중 정의(소비처 갈림) — T2-A 후속.
2. `shared/constants/terminal.ts:7` `MIN_FONT_SIZE` 소비처 0(미사용 export) — T2-A 후속.
3. 레인 색 "12" 의 CSS 변수·Rust 테마 서비스 잔여 열거(`theme-tokens.ts`·`global.css`·
   `domain/theme/service.rs`) — `LANE_COLOR_COUNT` 와의 파리티 테스트 신설 여부 포함 후속 판단.
4. `a11y-spec-3` 전체 APG 준수(Enter=keydown·Space=keyup 이원화) — 8지점 이벤트 배선 재설계
   필요, 후속 배치에서 별도 판단.
5. FileGroupHeader 클릭 영역·Tab 순서 잔여 편차(§5.3) — 오버레이/포커스 순서 안전 확보 방안이
   확정되면 후속 배치에서 재검토.
6. `a11y-spec-4` 의 `aria-controls` — 소비처 2곳의 id 배선 필요, 후속 판단.
