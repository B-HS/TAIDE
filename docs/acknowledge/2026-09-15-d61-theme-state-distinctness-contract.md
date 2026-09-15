# d-61 — 사용성 배치 5 웨이브 4: 테마 상태색 구별성·대비 전수 린트 · UI 토큰 배선 갭 · 형제 variant 9종 · 스크린샷 매트릭스 (2026-09-15)

> 사용자 항목 13("더 많은 스킨 테마, 모든 UI 를 고려해서 제대로"). 결정 전제: `acknowledge/2026-09-15-usability-batch5-user-decisions.md` §1 #13 · §3 #2·§3.1(상태색 겹침 정정 + 메인 집계) · §4(웨이브 3 과 worktree 병렬).
> 조사 정본 `research/2026-09-15-batch5-research.md` T9. 테마 정본 `docs/theme-system.md`. **worktree `/Users/gkn/taide-w4`(브랜치 `d61-themes`, dev 기점)에서 구현·검토 후 dev 머지.**
> 규칙: 코드 주석 금지(JSDoc·Rust /// 만)·arrow only·매직넘버 상수화·이모지 금지·`#[allow]` 금지·신규 의존성 0. 로케일 키 추가 시 3종 + MESSAGE_NAMESPACES 등재.

## 0. 현행 (T9 + 메인 직접 집계 — 근거 확인)

| 사실 | 근거 |
|------|------|
| 번들 테마 **38종**(+Rust 리터럴 builtin 2 = 카탈로그 40). 토큰 184/테마(colors 133·syntax 31·terminal 20) **전량 명시**(변환 스크립트가 미충족 시 exit 1) → "누락 폴백" 은 0. 실제 위험은 **서로 다른 토큰이 같은 값으로 접혀 상태·경계가 사라지는 것** | `src-tauri/resources/themes/*.json`, `src-tauri/src/domain/theme/service.rs:14-85,753-758`, `src/shared/lib/theme-convert/ui-token-vocabulary.ts` |
| **상태색 = 바탕색 동일값**(메인 집계, 38종): 에디터 드래그 선택 = 에디터 배경 6종(vscode-dark-plus·light-plus·dark-modern·light-modern, visual-studio-cpp-dark·light) / 설정 토글 켜짐 트랙(`button.primaryBackground`=`--primary`) = 설정 카드 배경(`panel.background`=`--card`) 5종 / 토글 손잡이(`app.background`) = 켜짐 트랙 6종 / 터미널 선택 = 터미널 배경 21종 / 비활성 선택 = 에디터 배경 17종 / 활성 탭 = 탭바 배경 10종 / 현재 줄 강조 = 에디터 배경 7종 / 버튼 hover = 버튼 배경 8종. 스위치: 켜짐 `bg-primary`·꺼짐 `bg-input`(=`input.border`)·손잡이 `bg-background` | 결정 문서 §3.1, `src/shared/ui/switch.tsx:11,17`, `src/shared/styles/global.css:132-149` |
| 대비 가드는 **7쌍**(앱·에디터·패널 헤더·툴팁·매치 강조 5 blocking + 선택 행 2 advisory), `MIN_CONTRAST_RATIO = 3`. ΔE 헬퍼 `deltaE76`(CIE76) + `MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E = 2.3` 존재. Rust 카탈로그 린트 7종(토큰 전량·app 대비·list 구분·matchHighlight 불투명/ΔE·대비 3축·list 전경) + 예외 등재분 역검증 테스트 | `src/shared/lib/theme-convert/contrast.ts:33,70-83,207,250,274`, `mapping-tables.ts:82-114`, `src/shared/lib/color.ts`, `service.rs:1349-1919` |
| 미배선 토큰 9종: `menu.background/border/itemHover/separator`(드롭다운·컨텍스트메뉴는 `--popover/--accent/--border` 사용), `scrollbar.thumb/thumbHover/track`(`::-webkit-scrollbar` 규칙 없음·OverlayScrollbar 미소비), `app.shadow`(그림자는 Tailwind 기본), `popover.separator`·`tooltip.itemHover/separator`·`modal.itemHover/separator`, `explorer.folderIcon/gitModified/gitAdded/gitDeleted` | T9 Q2(b), `src/shared/ui/dropdown-menu.tsx`, `context-menu.tsx`, `src/shared/scroll/overlay-scrollbar.tsx` |
| raw 색 5곳: `dialog.tsx:33`·`alert-dialog.tsx:24` `bg-black/50`, `button.tsx:14` `text-white`, `features/preview/html-preview.tsx:9` `bg-white`, `features/theme/color-picker.tsx:217,242` `border-white`. 죽은 `dark:` 분기 6곳(`button.tsx:8,14,16,18`·`dropdown-menu.tsx:56`·`context-menu.tsx:97` — `.dark` 클래스를 붙이는 코드 없음) | T9 Q2(a) |
| 형제 variant 후보(라이선스 MIT 1차 확인, 기존 등재 저장소): Catppuccin Latte(light)/Frappé/Macchiato, Tokyo Night Storm/Light(light), Gruvbox Light Medium(light)/Dark Hard/Dark Soft, Rosé Pine Moon, Ayu Mirage, GitHub Dark Dimmed (+ GitHub HC 2종은 `--type` 스키마 밖 → 제외·backlog). 변환 CLI `bun run themes:convert --input --id --name --type --source-url --author --license [--include-dir]`. 등록 4단계: 변환 → `service.rs` `include_str!` 배열 → `THIRD_PARTY_LICENSES.md` 백틱 id → 검증(licenses·contrast 테스트·Rust 린트). `THIRD_PARTY_LICENSES.md:17` "36 color themes"·`mapping-tables.ts:79`·`contrast.ts:57` 주석 stale(실제 38) | T9 Q3 |
| 신규 토큰 추가는 5곳 동기(vocabulary·theme-tokens·Rust types·builtin 2종·JSON 전량, `service.rs:2146` 대조 테스트) → **이 계약은 신규 토큰을 만들지 않는다** | T9 리스크 5 |
| e2e 하네스: `e2e/specs/09-theme-switch-revert.e2e.ts`(테마 전환 선례), `bun run e2e` 는 에이전트 실행 가능(dev 인스턴스 `TAIDE_E2E_NO_HMR=1 bun run tauri dev` 기동은 사용자) | `docs/quality-assurance/2026-08-18-e2e-harness.md` |

## 1. 수정 방향

### 1.A 상태색 vs 바탕색 구별성 린트 + 데이터 정정 (M~L, TS+Rust+JSON) — 최우선

- **쌍 표 정본** `src/shared/lib/theme-convert/state-distinctness-pairs.ts`: 컴포넌트가 실제로 어떤 바탕 위에 어떤 상태색을 그리는지 코드(`global.css` `@theme inline` 매핑 → `shared/ui/*`·위젯 클래스)로 도출한 전수 표 `STATE_DISTINCTNESS_PAIRS: { label, stateKey, containerKey, minDeltaE, alternative? }[]`. 최소 포함: 에디터 선택/비활성 선택/현재 줄 강조/찾기 매치/괄호 매치 vs `editor.background`; 터미널 선택 vs `terminal.background`; 목록 hover/선택 vs `list.background`, 선택 vs hover; 활성 탭 vs `tabBar.background`(대안: 활성 탭 상단 테두리 토큰이 있으면 그것의 구별성으로 대체); 스위치·체크박스 켜짐(`button.primaryBackground`) vs 설정 카드(`panel.background`)·앱 배경, 켜짐 vs 꺼짐 트랙(`input.border`), 손잡이(`app.background`) vs 켜짐·꺼짐 트랙; 버튼 hover vs 버튼; `input.border` vs `input.background`; `app.focusBorder` vs `app.background`·`panel.background`; `scrollbar.thumb` vs `scrollbar.track`; 배지·상태바·사이드바 활성 항목 등 실사용에서 발견되는 쌍 전부. 각 쌍은 어느 컴포넌트·클래스에서 유래했는지 JSDoc 1줄.
- **판정**: `#rrggbbaa` 는 바탕 위에 알파 합성 후 비교. `deltaE76(state, container) >= minDeltaE`(기본 상수 `STATE_MIN_DISTINCT_DELTA_E`, 값은 fixer 가 38종 분포를 실측해 "위 §0 동일값 사례는 전부 잡고 의도적 subtle 디자인은 통과" 하는 경계로 정하고 근거를 §3 에 기록 — 예: 동일값 = 0, subtle = 2~4, 명확 = 5+). 현재 줄 강조·비활성 선택처럼 관례상 subtle 한 쌍은 개별 `minDeltaE` 를 낮게.
- **TS**: `contrast.ts` 옆 `state-distinctness.ts`(`validateStateDistinctness(colors)` + `repairStateDistinctness(colors, palette)`), 변환기 `convert.ts` 파이프라인에 검증·수리 단계 추가, `bundled-theme-state-distinctness.test.ts`(38종 + 신규 전수 통과). 수리 규칙: **바탕이 아니라 상태 토큰을 고친다.** 테마 자체 `palette` 안에서 조건을 만족하는 가장 가까운 색을 고르고, 없으면 바탕↔전경 혼합(`mix(container, app.foreground, t)`, t 를 최소 ΔE 를 넘는 최소값으로 이진 탐색)으로 파생. 수리 결과는 결정적.
- **Rust 미러**: `service.rs` 카탈로그 린트에 같은 쌍 표·임계·알파 합성으로 `번들_테마는_상태색이_바탕색과_구별된다` 추가(선례 matchHighlight ΔE 린트). 쌍 표는 TS 정본을 Rust 가 복제하되 개수·라벨 일치 테스트로 드리프트 방지(선례 `service.rs:2146` 토큰 대조).
- **데이터 정정**: 38종 JSON 을 수리 규칙으로 갱신하는 1회 스크립트(`scripts/repair-theme-state-distinctness.ts`, 실행 후 결과 JSON 을 커밋·스크립트는 재실행 가능·멱등). 테마별 변경 토큰·전후 값 표를 `docs/theme-system.md` 신규 절에 기록. 예외 등재는 원칙적으로 0(의도적 flat 디자인이라 주장하려면 원본 VS Code 테마의 같은 토큰이 동일값임을 근거로 제시하고 역검증 테스트 짝 추가).

### 1.B 글자/배경 대비 전수 (M, TS+Rust)

- `CONTRAST_PAIRS` 를 컴포넌트 실사용 전경/배경 쌍 전수로 확장(목록 글자/hover·선택 행, 배지, 상태바, 입력 placeholder·글자, 버튼 글자/배경(primary·secondary·destructive), 탭 글자/활성·비활성 탭 배경, 메뉴 항목 글자/메뉴 배경·hover, 터미널 ANSI 16색 vs `terminal.background`(advisory), git 데코 색 vs 트리 배경, diff 추가/삭제 글자 vs 배경, 문제 심각도 색 vs 패널 배경). `MIN_CONTRAST_RATIO = 3` 유지(기존 임계). 실패는 `repairContrastPairs` 확장으로 전경 쪽 수리. Rust 대비 린트도 동일 확장. 38종 결과·정정 표 기록.

### 1.C UI 배선 갭 (M, TS/CSS)

- `menu.*` 4토큰: `global.css` `@theme inline` 노출 + `dropdown-menu.tsx`·`context-menu.tsx` 가 `bg-menu-background`·`border-menu-border`·`focus:bg-menu-item-hover`·구분선 `bg-menu-separator` 사용(팝오버 토큰 의존 제거). `popover.separator`·`tooltip.*`·`modal.*` 미배선 5종은 해당 컴포넌트(popover/tooltip/dialog 구분선·hover)에 배선.
- 스크롤바: `overlay-scrollbar.tsx` 가 `scrollbar.thumb/thumbHover/track` 을 사용, `global.css` 에 네이티브 `::-webkit-scrollbar` 규칙(OverlayScrollbar 를 안 쓰는 잔여 표면용).
- 그림자: `app.shadow` 를 팝오버·다이얼로그·드롭다운의 `shadow-*` 대신 `box-shadow` 색으로 배선. 모달 오버레이 `bg-black/50` 2곳 → `app.shadow` 기반 스크림(`color-mix(in srgb, var(--taide-app-shadow) 50%, transparent)` 또는 동등, 신규 토큰 없이).
- raw 색: `text-white`→`text-button-primary-foreground`(있으면) 또는 대응 토큰, `bg-white`(html 프리뷰 iframe)→`bg-editor-background`, `border-white`(색상환 핸들)→`border-app-border`. 죽은 `dark:` 6곳 제거. `explorer.folderIcon`→파일트리 폴더 아이콘 색, `explorer.gitModified/gitAdded/gitDeleted`→트리 git 데코 색(현 `git.*` 사용처를 explorer 토큰으로 전환하되 변환기가 두 토큰을 같은 원천에서 파생하도록 mapping 확인).
- 테스트: 토큰 소비자 존재를 기계 검사하는 테스트(`ui-token-vocabulary` 의 모든 colors 토큰이 `src/**` 에서 최소 1회 소비되거나 명시 예외 목록) 신설 — 재발 방지.

### 1.D 형제 variant 9종 추가 (M, 스크립트+JSON+Rust 등록+문서)

- 대상 9종(라이트 3 포함): Catppuccin Latte·Frappé·Macchiato, Tokyo Night Storm·Light, Gruvbox Light Medium·Dark Hard·Dark Soft, Rosé Pine Moon, Ayu Mirage, GitHub Dark Dimmed 중 **9종을 라이트 비율 ≥ 20% 가 되도록 선정**(fixer 가 최종 목록을 §3 에 기록). 원본 JSON 은 해당 저장소 태그의 raw 파일(WebFetch, 1차 출처)로 받아 `scripts/convert-vscode-theme.ts` 로 변환(`--include-dir` 필요 여부 확인). 1.A·1.B 린트를 **통과한 상태로** 추가.
- 등록: `service.rs` `include_str!` 배열, `THIRD_PARTY_LICENSES.md` 기존 섹션 "Bundled as:" id 추가 + 상단 개수 정정(36→47), `mapping-tables.ts:79`·`contrast.ts:57` 주석 개수 정정, `docs/theme-system.md` 카탈로그 표. `bundled-theme-licenses.test.ts` 가 개수도 검증하도록 보강(stale 재발 방지).

### 1.E 스크린샷 매트릭스 + 비전 검토 (M, e2e — 실행은 사용자 dev 기동 후)

- `e2e/specs/26-theme-surface-matrix.e2e.ts`: 번들 테마 전수를 순회하며 표면별 스크린샷(사이드바+탐색기 선택/hover 행, 에디터 드래그 선택·현재 줄·찾기 매치, 터미널 선택, git 패널, 탭바 활성/비활성, 명령 팔레트, 컨텍스트 메뉴, 설정 화면 토글 켜짐/꺼짐, 다이얼로그 오버레이, 툴팁)을 `e2e/artifacts/theme-matrix/<theme>/<surface>.png` 로 저장. 선례 `09-theme-switch-revert` 의 테마 전환 방식 재사용. 스펙은 작성·`typecheck:e2e` 통과까지, 실행은 사용자가 `TAIDE_E2E_NO_HMR=1 bun run tauri dev` 를 띄운 뒤 메인이 `bun run e2e --grep theme-surface-matrix` 로.
- 실행 후 비전 검토 wf(sonnet): 스크린샷을 읽어 "상태가 안 보임/경계 소실/글자 묻힘" 을 표면별로 판정 → 결과를 `docs/quality-assurance/2026-09-15-theme-matrix-review.md` 에, 결함은 1.A/1.B 표·임계 보정으로 환류.

### 1.F 범위 외

- 고대비(hc) 테마 타입, 신규 토큰 추가, Tier B 서드파티 테마(라이선스 미검증), 취약 테마 전면 재변환(O4 — 1.A/1.B 수리로 대체), Monaco syntax 토큰 19종 확장(backlog 기존).

## 2. 실행 계획

- worktree: `git worktree add /Users/gkn/taide-w4 -b d61-themes dev` → `bun install --frozen-lockfile` → 첫 `cargo test` 로 빌드 캐시 생성(메인이 준비).
- 구현 wf(opus·xhigh, 작업 경로 `/Users/gkn/taide-w4`): **A1**(1.A TS 쌍 표·검증·수리·스크립트·JSON 정정) → **R**(1.A Rust 미러 + 1.B Rust 확장 + 1.D Rust 등록, Rust 단일) ; **A1 완료 후 B1**(1.B TS) ; **C**(1.C, 병렬) ; **D**(1.D 변환·문서, A1 의 수리 규칙이 convert.ts 에 들어간 뒤 실행) ; **E**(1.E 스펙, 병렬). 각 에이전트 종료 조건: worktree 에서 typecheck·lint·prettier·관련 bun test·(Rust) fmt/clippy/test.
- 렌즈 검토 wf(sonnet·xhigh 3렌즈: 데이터 정정의 근거·회귀(기존 테마 외관 변화 최소성)·경계(라이선스·5곳 동기)). 메인 2차 `bun run verify` + vite build(worktree) → 커밋(분할: feat(theme) 린트·data(theme) 정정·feat(ui) 배선·feat(theme) 9종·test(e2e)) → dev 머지(3 과 순서 조율) → 사용자 dev 기동 → 1.E 실행·비전 검토 → 환류 수정.

## 3. 구현 기록

<!-- d61-record-A1 -->
<!-- d61-record-R -->
<!-- d61-record-B1 -->
<!-- d61-record-C -->
<!-- d61-record-D -->
<!-- d61-record-E -->

## 4. 후속

- 스크린샷 매트릭스 실행·비전 검토 결과 환류. 고대비 타입. Tier B 라이선스 검증 후 추가.
