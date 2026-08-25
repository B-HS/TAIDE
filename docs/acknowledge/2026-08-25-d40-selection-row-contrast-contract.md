# d-40 — 선택 행 표면 전경 대비 정공법 계약 (2026-08-25)

> 정본: d-36 계약 §4(적대적 재프레이밍)·§5(정정 수치표 — 알파 합성 필수)·
> `2026-08-25-post-batch-user-decisions.md` §5(사용자 확정 "전면 게이트+데이터 정정").
> **재론 금지 승계**: 테마 하드 구별성 필터·validate 단계 임포트 차단은 d-33 적대적이 실테마
> 임포트 차단으로 기각 확정 — 이번 게이트는 **수리(repair) 경로**로 설계해야 하며 임포트
> 거부를 새로 만들면 안 된다. 번들 재변환 강제 금지(§8.2.3)도 승계.

## 0. 실사 기준 (d-36 §5 승계 — 구현이 착수 시 재실측)

- **결함 클래스**: 선택 행 배경(`list.activeBackground`)에 대한 전경 대비 가드가 매치 전경
  (`panel.matchHighlight`)·일반 전경(`list.foreground`) 양축 모두 부재.
- 정정 수치(알파 합성 — `compositeOverBackground` 기준): 매치축 <3 = 7/38(비예외 5:
  nord **1.00**·vscode-monokai-dimmed 1.82·vscode-abyss 2.18·vscode-quiet-light 2.59·
  taide-light 2.98 / 예외 등재분: rose-pine-dawn 2.36·everforest-light 2.45), 일반축 <3 =
  8/38(최악 **palenight 1.13**·nord 1.48·ayu-dark 1.99·ayu-light 2.19 …).
- nord 특이: `panel.matchHighlight == list.activeBackground == app.accent ==
  explorer.itemSelected == #88c0d0` — 선택 링 2차 단서까지 동시 무력.
- 렌더 경로 확증(d-36 §4): 팔레트 선택 행 = `list.activeBackground` 배경 위
  `text-panel-match-highlight`(매치)·`text-accent-foreground`→`list.foreground`(일반).
  비선택 행은 `panel.background`(현행 게이트가 정확).

## 1. 범위 (사용자 승인 — 설계 세부 위임, 원칙 고정)

- **a. TS 파이프라인 게이트 확장**: `contrast.ts` `CONTRAST_PAIRS` 에 선택 행 2쌍 추가 —
  (`panel.matchHighlight`, `list.activeBackground`)·(`list.foreground`, `list.activeBackground`).
  알파 합성 기존 일괄 적용 승계. **수리 후보 사슬 설계 필수**(매치축은 d-33 상태색 우선 2패스
  승계 검토, 일반축은 업스트림 불투명 전경 계열) — 두 후보 모두 실패 시 기존 safe-default
  폴백 경로 유지(임포트 거부 신설 금지 — 상단 재론 금지 확인). `repairContrastPairs` 확장이
  기존 5쌍 판정·수리를 불변으로 유지하는지 테스트로 고정.
- **b. 번들 데이터 정정**: 게이트 확장으로 FAIL 하는 테마를 d-31 §3-A 방식(업스트림 팔레트
  스케일 내 재선정 — 재변환 아님·손수정)으로 정정. nord 동일색은 최우선(4토큰 동일은 예외
  사유 불성립). 업스트림에 대체 후보가 없는 테마만 예외 등재(사유 필수 — 기존 2종 형식).
  builtin 2종(taide-light 매치축 2.98 포함)은 Rust 하드코딩 값 정정으로 동일 원칙.
- **c. Rust 카탈로그 린트 확장**: d-36 게이트 계열에 선택 행 2쌍 추가(38종·예외 목록·FAIL
  재현 확증). TS·Rust 임계 동일값.
- **d. 재스윕 기록**: 38종 × 신규 2쌍 전후 수치표를 §3 에 기록(알파 합성 적용 명시).
- 범위 외: `explorer.itemSelected` 등 다른 표면 쌍(발견 시 이월 기록만)·팔레트 UX 재설계·
  ring 단서 재설계(nord 데이터 정정으로 자연 해소되는지 확인·기록만).

## 2. 실행·검증

- 구현: d-41 종료 후 순차 2 — A(TS 게이트+수리 사슬+번들 데이터 정정+재스윕, sonnet+xhigh) →
  B(Rust 린트+builtin 정정, sonnet+xhigh — Rust 단독). 각자 §3 기록.
- 검토 3렌즈(opus+xhigh): ① 대비·수리 수학(합성·후보 사슬·재스윕 재계산) ② 임포트 행동
  동등성(**실테마 임포트 차단 0 — 수리로만 흡수되는지가 최우선**·기존 5쌍 불변) ③ 데이터
  정정 충실성(업스트림 원본 대조·테마 정체성 유지·예외 사유). major 적대적(opus+high) →
  수정 → 메인 2차(verify+vite). 커밋은 사용자 규칙.

---

## 3. 구현 완료 기록 (구현 에이전트가 기록)

### 3-A. A축(TS) 완료 — 임포트 거부 실사·수리 사슬 설계

**실사(§1-a 필수 선행 조사)**: `validateOutputColors` 의 반환값은 `convert.ts` 에서
`outputColorErrors` 로 그대로 넘어가고, `vsix-theme-import.ts:65`
(`if (conversion.outputColorErrors.length > 0) return 'contrast'`)와
`scripts/convert-vscode-theme.ts:133`(`exit 1`) 양쪽에서 **그 배열이 비어있지 않으면 즉시
임포트/변환을 거부**한다 — 기존 5쌍은 원래도 이 경로로 거부될 수 있었다(문서화된 사실,
`docs/theme-system.md` §9.3). `CONTRAST_PAIRS` 에 신규 2쌍을 그대로 얹으면 `repairContrastPairs`
가 후보를 못 찾는 실사용 VSIX(감사되지 않은 미래 임포트)에서 **신규 거부 케이스가 열린다** —
계약이 우려한 대로 실제로 발생 가능. → 아래 "수리-전용(비거부) 설계"로 조정.

**설계**: `ContrastPair` 에 `blocking: boolean` 필드 추가. 기존 5쌍은 `blocking: true`(불변),
신규 2쌍(`selectionMatchHighlight`·`selectionForeground`)은 `blocking: false`.

- `repairContrastPairs` — 7쌍 **전부**에 대해 수리를 시도한다(블로킹·어드바이저리 구분 없음).
  블로킹 5쌍을 먼저 처리(기존 로직 100% 그대로, 단일 배경 후보 탐색)한 뒤, 어드바이저리 2쌍을
  처리한다.
- `validateOutputColors` — **블로킹 5쌍만** 검사(기존 시그니처·기존 반환 타입·기존 동작 불변).
  `outputColorErrors` 는 구조적으로 신규 2쌍의 실패를 절대 포함할 수 없다 — "오늘 데이터로는
  안 막힌다"가 아니라 "코드 경로 자체가 막을 수 없다"는 하드 개런티.
- `validateSelectionRowContrast`(신규 export) — 어드바이저리 2쌍만 검사, 번들 감사(§1-d)·
  Rust 카탈로그 린트(§1-c) 전용. 임포트 결정 경로에 배선되지 않음.
- 공유 로직은 `describeContrastViolations(colors, pairs)` 로 통합(2회 이상 재사용 — common.md
  §3.3).

**공유 전경 충돌 가드(`repairPair` 의 `protectedBackgroundKeys`)**: `panel.matchHighlight` 는
블로킹 `matchHighlight`(vs `panel.background`)·어드바이저리 `selectionMatchHighlight`(vs
`list.activeBackground`) 두 쌍이 **같은 토큰을 공유**한다. 순수 per-pair 루프였다면 어드바이저리
수리가 이미 통과한 블로킹 값을 다른 값으로 덮어써 블로킹 쌍을 새로 깨뜨릴 수 있었다(nord 류
시나리오로 실증 가능 — 아래). 어드바이저리 쌍을 수리할 때는 그 전경 키를 공유하는 **블로킹
배경 전체**를 `protectedBackgroundKeys` 로 넘겨, 후보가 자기 배경뿐 아니라 그 보호 배경도 동시에
만족해야만 채택되게 한다. 만족하는 후보가 없으면 조용히 미수리(어드바이저리만 실패로 남음,
블로킹 값은 무손상) — `contrast.test.ts`
`"두 배경을 동시에 만족하는 후보가 없으면..."` 테스트로 고정.

**수리 후보 사슬**: 매치축(`selectionMatchHighlight`)은 `panel.matchHighlight` 를 그대로
공유하므로 기존 `CONTRAST_REPAIR_FOREGROUND_CANDIDATES['panel.matchHighlight']`(d-33 상태색
우선 사슬 + 구별성 가드 2패스)를 **그대로 재사용**한다(신규 목록 불필요, foregroundKey 기준
자동 적용). 일반축(`selectionForeground`, `list.foreground`)은 자체 매핑 체인(`sideBar.foreground`
→ `foreground`)이 이미 실패한 값이므로 그 재시도를 피해 `editor.foreground` 1개 후보로 새로
등록 — `app.foreground`/`panel.sectionHeader` 가 쓰는 것과 동일한 "업스트림 불투명 전경" 원천.

**테스트로 고정한 불변**: `contrast.test.ts` 에 6개 테스트 추가(기존 6개는 무수정, `BASE_COLORS`
에 `panel.matchHighlight`/`list.foreground`/`list.activeBackground` 3키만 추가해 크래시 방지 —
기존 어서션 값은 전부 그대로 통과). 신규 테스트: (1) 선택 행 2쌍이 모두 저대비여도 기존 5쌍이
통과하면 `validateOutputColors` 는 `[]`(거부 없음) — "임포트 거부 신설 금지"의 직접 증거.
(2) 기존 5쌍 중 하나라도 실패하면 여전히 거부(신규 축이 기존 판정을 약화시키지 않음).
(3) `list.foreground` 일반축 수리(`editor.foreground` 후보). (4) 공유 전경 충돌 가드(위 문단).
`bundled-theme-contrast.test.ts` 는 기존 2테스트 무수정(둘 다 `validateOutputColors` 만
호출 — 블로킹 5쌍 불변이므로 자동으로 그대로 통과) + 신규 2테스트(아래 §3-C 예외 포함).

**convert.test.ts 사이드 이펙트 1건**: rose-pine-dawn 프록시 테스트(라인 269)가 신규 어드바이저리
축의 영향을 받았다 — 소스가 `list.activeSelectionBackground` 를 정의하지 않아 VS Code 기본값
(`#ADD6FF`)으로 떨어졌고, 그 값에 대해 기존 블로킹 수리 결과(`#907aa9`)가 어드바이저리 축에서도
실패해 2패스가 재차 발동, 최종값이 `#575279`(본문 전경과 동일)로 바뀌었다 — **판정(pass)은
불변**(`outputColorErrors` 여전히 `[]`)이지만 **선택된 수리값**이 바뀌어 기존 단정을 깰 뻔했다.
근본 수정: 이 프록시가 실제 rose-pine-dawn 처럼 `list.activeSelectionBackground` 를 명시하지
않았던 것 자체가 실제 테마 대비 불완전한 픽스처였다고 판단해, 같은 업스트림 파레트의 "overlay"
색(`#f2e9e1`, `#907aa9` 대비 3.16)을 소스에 추가해 원래 어서션(`#907aa9`)을 그대로 지킴 — 기존
축 하나에만 집중하던 픽스처 의도를 보존하고, 신규 축과의 상호작용은 별도 유닛 테스트
(`contrast.test.ts`)로 분리해서 검증했다.

### 3-B. 번들 데이터 정정 (§1-b, d-31 §3-A 방식 — 업스트림 대조·스케일 내 재선정·손수정)

13개 번들 테마를 정정했다(각 항목당 VS Code/테마 업스트림 원본 저장소를 직접 재취득해 대조).
공통 발견: 다수 업스트림이 `list.activeSelectionForeground` 라는 **선택 행 전용 전경 토큰**을
따로 정의하는데, TAIDE `list.*` 스키마엔 그 짝이 없어(§0 렌더 경로 확증대로 `list.foreground`
하나가 선택/비선택 공용) 변환 시 버려지고 있었다 — 있는 테마는 그 값을 `list.foreground` 에
직접 반영해 정정(배경은 그대로 두어 identity 보존), 없는 테마만 배경을 바꿨다.

| 테마 | 토큰 | 이전 | 이후 | 근거 |
|---|---|---|---|---|
| nord | `list.foreground` | `#d8dee9` | `#2e3440` | 업스트림 `nordtheme/visual-studio-code`(develop, `themes/nord-color-theme.json`)의 `list.activeSelectionForeground`(nord0, 짙은 배경색) 그대로 반영 — `list.activeBackground`(`#88c0d0`, nord8)는 `app.accent`/`explorer.itemSelected` 와 동일하게 identity-critical 이라 불변. `panel.matchHighlight` 는 §3-C 예외(수학적으로 두 배경 동시 불가). |
| vscode-tomorrow-night-blue | `list.activeBackground` | `#ffffff60`(반투명) | `#003f8e` | 업스트림 `microsoft/vscode` `extensions/theme-tomorrow-night-blue` 의 `editor.selectionBackground` — 같은 테마의 다른 "선택" 개념을 그대로 재사용. `list.activeSelectionForeground` 미정의(업스트림에 없음). |
| vscode-monokai-dimmed | `list.activeBackground` | `#707070` | `#404040` | `list.activeSelectionForeground` 업스트림 미정의. 같은 파일의 `tabBar.tabInactiveBackground`(중립 진회색, 스케일 내 재선정) — `editor.selectionBackground`(반투명 `#676b7180`)는 두 축 중 매치축을 못 살려 기각. |
| vscode-abyss | `list.activeBackground` | `#08286b` | `#000c18` | `list.activeSelectionForeground` 업스트림 **빈 문자열**(명시적 미정의). `app.background`(테마 자체 최암색, 스케일 내 재선정) — `editor.selectionBackground`(`#770811`, 심홍색)도 시도했으나 매치축 미달로 기각. |
| everforest-light | `list.foreground` | `#939f91` | `#5c6a72` | 업스트림 `sainnhe/everforest-vscode`(master, `themes/everforest-light.json`)의 `list.activeSelectionForeground` — 우연히 `app.foreground` 와 동일값. `panel.matchHighlight` 는 §3-C 예외(기존 예외 사유가 이 축에도 적용). |
| vscode-quiet-light | `panel.matchHighlight` | `#9769dc` | `#7A3E9D` | `list.activeSelectionForeground`(`#6c6c6c`)는 이미 정의돼 있으나 이 테마는 listForeground 축이 이미 통과(11.04) — 실패 축은 매치뿐. 같은 업스트림 파일의 `tokenColors` 보라 계열(`#7A3E9D`, d-33 "임무 C" 방식과 동일하게 같은 파일 내 syntax 강조색 재사용)로 정정 — 두 배경(panel.background 6.25·list.activeBackground 4.63) 모두 개선. |
| palenight | `list.foreground` | `#6C739A` | `#ffffff` | 업스트림 `whizkydee/vscode-palenight-theme`(master)의 `list.activeSelectionForeground` — `app.foreground` 와 동일값(흰색). |
| rose-pine | `list.foreground` | `#908caa` | `#e0def4` | 업스트림 `rose-pine/vscode`(main, `rose-pine-color-theme.json`)의 `list.activeSelectionForeground`. |
| ayu-dark | `list.foreground` | `#5a6378` | `#bfbdb6` | 업스트림 `ayu-theme/vscode-ayu`(master)의 `list.activeSelectionForeground` — 참고: 이 테마의 `list.activeSelectionBackground`(반투명 `#47526640`)는 업스트림 `list.hoverBackground` 와 동일해 기존 변환기 가드(`mapping-tables.ts` `list.activeBackground` derive, ayu 를 명시 언급)가 이미 VS Code 기본값(`#04395E`)으로 정확히 대체해뒀다 — 그 판단은 불변, 전경만 정정. |
| ayu-light | `list.foreground` | `#828e9f` | `#5c6166` | 위와 동일 사유(업스트림 `vscode-ayu` `list.activeSelectionForeground`) — `list.activeBackground` 도 동일 가드로 이미 `#ADD6FF`. |
| vscode-solarized-dark | `list.activeBackground` | `#005A6F` | `#274642` | `list.activeSelectionForeground` 업스트림 빈 문자열. 같은 파일 `editor.selectionBackground` 재사용(§1-b 방식의 "선택 개념 재사용" 패턴). |
| everforest-dark | `list.foreground` | `#859289` | `#d3c6aa` | 업스트림 `sainnhe/everforest-vscode`(master, `themes/everforest-dark.json`)의 `list.activeSelectionForeground`. |
| solarized-light | `list.foreground` | `#657B83` | `#6C6C6C` | 업스트림 `microsoft/vscode` `extensions/theme-solarized-light` 의 `list.activeSelectionForeground`. |

### 3-C. 예외 등재 (업스트림 대체 후보 없음 — `bundled-theme-contrast.test.ts` 확장)

- **nord** — `selectionMatchHighlight` 만 예외(`selectionForeground` 는 §3-B 로 해소). `panel.background`(`#2e3440`, 어두움)와 `list.activeBackground`(`#88c0d0`, 밝은-중간)의 광도 차가 커서, nord 16색 팔레트 전체를 대입해도(nord1~nord15 실측) **양쪽 배경에 동시에 3:1 을 만족하는 색이 하나도 없다**(한쪽을 만족하면 반드시 다른 쪽이 깨지는 구조적 대칭). `panel.matchHighlight` 는 이미 통과 중인 `panel.background` 축(6.24)을 지키기 위해 불변 유지.
- **everforest-light** — `selectionMatchHighlight` 만(`selectionForeground` 는 §3-B 로 해소). 기존 `panel.background` 축 예외 사유(green `#8da101` 에 업스트림 대체 shade 없음)가 이 축에도 그대로 적용 — `list.activeBackground`(`#e6e2cc`, 더 밝음)가 panel.background 보다 요구 조건이 더 엄격해 기존 shortfall 이 넓어질 뿐.
- **rose-pine-dawn** — `selectionMatchHighlight`·`selectionForeground` 둘 다. 업스트림이 `list.activeSelectionBackground` 를 **반투명 오버레이**(`#6e6a8614`, 알파 ~8%)로 설계했는데, 이 게이트(`contrast.ts` `hexToRgb`)는 배경의 알파를 무시하고 raw RGB(`#6e6a86`)로 읽는다 — 실제 렌더(연한 크림 위 8% 오버레이, 사실상 거의 base 색)보다 훨씬 어둡게 측정된다. 업스트림 자신의 전용 선택 전경(`#575279`, 이미 이 테마의 `app.foreground`)조차 그 raw-RGB 배경 앞에서 1.41 로 미달 — 대체 후보가 없는 게 아니라 **게이트의 측정 방식과 업스트림의 반투명 설계가 근본적으로 안 맞는 케이스**.

`bundled-theme-contrast.test.ts` 에 `SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`·
`SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS` 2개 맵 + 감사 테스트 2개 추가(기존
`MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS` 절은 무수정).

### 3-D. 재스윕 표 (38종 × 신규 2쌍, 알파 합성 적용 — `compositeOverBackground` 기준)

전경이 8자리(알파 포함)면 배경 위에 합성 후 측정(기존 파이프라인과 동일 규칙), 배경 자체의
알파는 (기존 5쌍과 동일하게) 무시하고 raw RGB 로 측정 — §3-C rose-pine-dawn 참고. FAIL 이던
14개 행(정정 13 + 예외 전용 1[everforest-light 는 두 축 중 하나만 FAIL])만 "이전≠이후"다.

| id | matchHl:activeBg 이전 | 이후 | listFg:activeBg 이전 | 이후 |
|---|---|---|---|---|
| ayu-dark | 6.28 | 6.28 | 1.99 | 6.38 |
| ayu-light | 4.78 | 4.78 | 2.19 | 4.12 |
| catppuccin-mocha | 6.19 | 6.19 | 8.69 | 8.69 |
| darcula | 4.07 | 4.07 | 7.46 | 7.46 |
| dracula | 6.61 | 6.61 | 8.59 | 8.59 |
| everforest-dark | 4.01 | 4.01 | 2.47 | 4.75 |
| everforest-light | 2.23 | 2.23(예외) | 2.12 | 4.29 |
| github-dark | 7.21 | 7.21 | 7.02 | 7.02 |
| github-light | 5.09 | 5.09 | 5.05 | 5.05 |
| gruvbox-dark | 3.78 | 3.78 | 6.99 | 6.99 |
| intellij-islands-light | 7.55 | 7.55 | 15.30 | 15.30 |
| kanagawa-wave | 4.31 | 4.31 | 8.16 | 8.16 |
| monokai | 3.45 | 3.45 | 4.60 | 4.60 |
| night-owl-light | 4.51 | 4.51 | 6.74 | 6.74 |
| night-owl | 8.87 | 8.87 | 3.42 | 3.42 |
| nord | 1.00 | 1.00(예외) | 1.48 | 6.24 |
| one-dark-pro | 10.07 | 10.07 | 5.62 | 5.62 |
| one-monokai | 7.57 | 7.57 | 8.81 | 8.81 |
| palenight | 3.48 | 3.48 | 1.13 | 5.21 |
| rose-pine-dawn | 1.81 | 1.81(예외) | 1.17 | 1.17(예외) |
| rose-pine | 3.06 | 3.06 | 1.60 | 3.91 |
| solarized-light | 5.22 | 5.22 | 2.75 | 3.24 |
| tokyo-night | 4.46 | 4.46 | 3.82 | 3.82 |
| vitesse-dark | 3.28 | 3.28 | 6.18 | 6.18 |
| vitesse-light | 4.26 | 4.26 | 5.46 | 5.46 |
| vscode-abyss | 2.18 | 3.12 | 3.90 | 5.59 |
| vscode-dark-modern | 4.07 | 4.07 | 7.46 | 7.46 |
| vscode-dark-plus | 4.07 | 4.07 | 8.09 | 8.09 |
| vscode-kimbie-dark | 3.70 | 3.70 | 3.39 | 3.39 |
| vscode-light-modern | 4.69 | 4.69 | 9.14 | 9.14 |
| vscode-light-plus | 3.79 | 3.79 | 13.84 | 13.84 |
| vscode-monokai-dimmed | 1.82 | 3.81 | 2.94 | 6.15 |
| vscode-quiet-light | 2.59 | 4.63 | 11.04 | 11.04 |
| vscode-red | 3.01 | 3.01 | 9.66 | 9.66 |
| vscode-solarized-dark | 3.36 | 4.44 | 2.47 | 3.25 |
| vscode-tomorrow-night-blue | 1.44 | 6.94 | 1.00 | 9.99 |
| taide-dark(builtin) | 7.18 | 7.18(미정정) | 6.31 | 6.31(미정정) |
| taide-light(builtin) | 2.98 | 4.45(B축 §3-G 확정 `#6611d4` 반영 — A축 기록 시점 2.98 은 미정정 원값) | 4.39 | 4.39(미정정) |

**§0 대비 재실측 정정**: 구현 착수 시 재실측 결과 §0 수치와 소폭 다르다(재론 금지 대상 아님,
§0 자체가 "구현 시 재실측" 명시) — d-36 시점 표본에 없던 `vscode-tomorrow-night-blue`(매치축
1.44)가 실제로는 FAIL 로 확인됐고, rose-pine-dawn/everforest-light 매치축 수치도 2.36→1.81,
2.45→2.23 으로 갱신됐다(같은 결론 — 예외 유지, 수치만 정밀화).

### 3-E. builtin 2종 후보 (B축/Rust 전달용 — 수치만, `src-tauri/src` 무접촉)

- **taide-dark**: 두 축 모두 이미 통과(7.18/6.31) — 정정 불요.
- **taide-light**: `selectionMatchHighlight` 만 FAIL(2.98, §0 수치와 일치). `panel.matchHighlight`
  는 catppuccin latte "mauve"(`#8839ef`) 그대로, `list.activeBackground` 는 "surface1"
  (`#bcc0cc`) 그대로 — 둘 다 이 팔레트의 다른 다수 토큰이 재사용 중인 identity 색이라 배경 쪽은
  건드리지 않는 편이 안전. 후보(같은 보라 계열 내 재선정, B축이 최종 결정):
  `#6d28d9`(panel.background 대비 5.84·list.activeBackground 대비 3.91) 또는
  `#6a3aad`(6.12·4.09) — 둘 다 catppuccin 표준 팔레트엔 없는 "짙은 mauve" 변형이라, B축에서
  이 값을 채택하려면(또는 다른 값을 고르려면) catppuccin 자체 tint/shade 시스템 대조를 별도로
  거치는 편이 낫다(A축은 조사만, 결정·구현은 B축 몫).

### 3-F. 검증

- `bun run typecheck` — 통과.
- `bunx prettier --check`(변경 TS 4개 + JSON 13개) — 통과.
- `bunx eslint`(변경 TS 4개) — 위반 0.
- `bun test` — 1487 pass / 0 fail (전량, `bundled-theme-contrast.test.ts` 포함).
- `git status --porcelain src-tauri/src` — 비어있지 않음(`domain/remote/dispatch.rs`·
  `domain/sync/service.rs` 2건 M) — **본 세션이 만든 변경이 아니다**(세션 내내 Read/Edit/Write
  대상에 `src-tauri/src` 파일을 한 번도 포함하지 않았음 — 병행 진행 중인 별도 세션/작업의
  기존 dirty 상태로 추정, A축 범위 밖). A축이 만진 파일은 `src-tauri/resources/themes/*.json`
  13개 + `src/shared/lib/theme-convert/{contrast.ts,contrast.test.ts,bundled-theme-contrast.test.ts,convert.test.ts}`
  4개뿐 — `src-tauri/src` 무접촉 확인.

### 3-G. B축(Rust) 완료 — 카탈로그 린트 2쌍 확장·taide-light 값 정정·FAIL 재현

Rust 단독 1 에이전트(sonnet+xhigh), 2026-08-25. 변경 파일은
`src-tauri/src/domain/theme/service.rs` 1개(`git diff --stat`: 206 삽입/20 삭제) —
계약 절대 규칙(A축 산출물 13+4·d-41 산출물 `dispatch.rs`/`sync/service.rs`/`app/commands.rs`
무접촉)을 `git status --porcelain src-tauri/src src-tauri/resources src`(작업 종료 시점)로
확인, 위 세 d-41 파일은 세션 시작 전부터 이미 dirty했고 본 세션은 Read/Edit/Write 어느 것도
그 파일들에 적용하지 않았다.

**실사(§1-c 필수 선행 조사) — 신규 2쌍에 알파 합성이 필요한가**: 38종 카탈로그(36 번들 JSON +
builtin 2종) 전수를 스캔해 `panel.matchHighlight`·`list.foreground`(신규 쌍의 **전경** 키)가
전부 6자리 불투명 hex임을 확인(예외 0건). 반면 **배경** 키 `list.activeBackground` 는 5개
테마(`everforest-dark` `#47525880`·`everforest-light` `#e6e2cc80`·`night-owl` `#234d708c`·
`rose-pine-dawn` `#6e6a8614`·`rose-pine` `#6e6a8633`)에서 실제로 8자리 알파를 가진다. 그런데
TS `contrast.ts`(`foregroundContrastRatio` → `compositeOverBackground`)를 재확인한 결과 —
**배경 인자는 애초에 합성 대상이 아니다**: `compositeOverBackground`는 *전경* 인자가 9자리일
때만 그 전경을 배경 위에 합성하고, 반환값을 다시 `contrastRatio(composited, backgroundHex)`
로 넘긴다 — `backgroundHex` 자체는 `relativeLuminance`→`hexToRgb`가 앞 6자리만 읽어 **항상
raw RGB(알파 무시)**로 측정된다. 이는 §3-C `rose-pine-dawn` 예외 사유가 이미 명시한 바로 그
동작("게이트가 배경의 알파를 무시하고 raw RGB로 읽는다")과 정확히 같다. 즉 TS·Rust 양쪽 모두
배경 알파는 원래부터 무시하도록 설계돼 있고, 유일하게 실제로 합성이 필요한 경우(전경이
8자리)는 신규 2쌍에서 한 번도 발생하지 않는다(전경 두 키가 38종 전부 불투명) — 기존 Rust
`hex_to_rgb`(수정 없이 그대로: `normalize_hex_color`로 8자리 정규화 후 앞 6자리만 파싱, 알파
버림)를 신규 2쌍에 그대로 재사용해도 TS와 동일한 결과가 나옴을 **38종 전량 재계산**(Python
으로 WCAG 공식 독립 재구현, `contrast.ts`/Rust 상수와 값 일치 확인)으로 실측 확증했다 —
계산 결과가 A축 §3-D 재스윕 표의 "이후" 열 76개 수치(38종 × 2쌍) 전부와 소수점 둘째 자리까지
정확히 일치했다. 결론: **합성 이식 불요**, `hex_to_rgb`/`contrast_ratio` 함수 본체는 무수정,
그 위 doc 주석만 이 실사 결과를 반영해 갱신(배경 알파를 무시하는 것이 "우연히 오늘 데이터가
그런 것"이 아니라 TS와 공유하는 의도된 설계임을 명시, `list.activeBackground` 알파 보유 5종을
근거로 인용).

**임계 상수 — `MATCH_HIGHLIGHT_MIN_CONTRAST` → `MIN_CONTRAST_RATIO` 로 일반화(관행 판단)**:
d-36 이 신설한 상수는 그 시점엔 단일 쌍(matchHighlight vs panel.background) 전용이라 이름이
그 쌍을 가리켰다. 이번 배치로 같은 임계(3.0)를 3개 쌍이 공유하게 되면서 옛 이름이 부정확해져
(d-36 §3.2 가 "번들"→"카탈로그" 테스트명 개명에 적용한 것과 같은 "네이밍은 정확하게" 논리),
TS 가 처음부터 쓰던 일반명 `MIN_CONTRAST_RATIO` 로 리네임했다(값 불변, 사용처 4곳 + doc 주석
갱신). 계약이 위임한 "재사용 또는 공용 상수화" 중 후자를 선택한 근거: 상수 자체는 이미
파일에 1개뿐이라 "공용 상수화"를 위한 신규 상수 도입은 불필요했고, 기존 상수를 정확한
이름으로 리네임하는 편이 diff 최소·개념 일치 양쪽에 더 부합했다.

**린트 2종 신설**: 기존 5개 린트와 동일한 구조(위반 `Vec` 수집 → 예외 스킵 → 단일 `assert!`)로
`카탈로그_테마는_panel_매치_하이라이트가_선택_행_배경과_최소_대비를_가진다`(`panel.matchHighlight`
vs `list.activeBackground`)·`카탈로그_테마는_list_전경색이_선택_행_배경과_최소_대비를_가진다`
(`list.foreground` vs `list.activeBackground`) 2개 `#[test]`를 `theme_catalog()`(38종) 순회로
추가. 각각 전용 예외 감사 테스트(`선택_행_매치_하이라이트_대비_예외_등재분은_실제로_최소_대비에_미달한다`·
`선택_행_전경색_대비_예외_등재분은_실제로_최소_대비에_미달한다`)를 동반 — 기존 matchHighlight
쌍의 감사 테스트와 1:1 대응. 기존 5개 린트·감사 테스트는 로직·값 무수정(diff 상 리네임한
상수명 4곳만 변경).

**예외 목록 — TS 와 동일 사유 문자열로 이식**: `SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`
3건(`nord`·`everforest-light`·`rose-pine-dawn`)·`SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS` 1건
(`rose-pine-dawn`) — TS `bundled-theme-contrast.test.ts` 의 동명 두 `Record` 리터럴에서 문자
그대로(영어 원문 그대로) 복사했다. 지시문("A축 §3-C 확정 2건(nord·everforest-light)")은
selectionMatchHighlight 축의 대표 사례만 요약한 것으로 판단했다 — §3-C 본문·TS 소스
(`SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS`)모두 `rose-pine-dawn` 이 **두 축 모두**(사유:
`list.activeSelectionBackground` 가 8% 알파 오버레이라 raw-RGB 측정이 실제 렌더보다 훨씬
어둡게 읽힘) 예외임을 명시하고 있고, 실측(§0 계산·아래 FAIL 재현)도 `rose-pine-dawn` 의
listFg 축이 1.17 로 미달함을 재확인해 — 정본(§3-C)·실측 양쪽이 일치하는 3+1 구성으로
등재했다. 프로덕션 예외 3건 모두 감사 테스트로 "정말 미달하는지" 재확인 통과.

**taide-light `selectionMatchHighlight` 값 정정 — Catppuccin Latte 팔레트 대조**: A축 §3-E 의
위임("표준 안에서 만족하는 악센트 있으면 그것, 없으면 팔레트 밖 채택 또는 nord형 예외 —
B축 결정")에 따라 아래 순서로 판단했다.

1. **Catppuccin Latte 표준 14개 악센트 전수 실측**(rosewater·flamingo·pink·mauve·red·maroon·
   peach·yellow·green·teal·sky·sapphire·blue·lavender, vs `panel.background #e6e9ef`·
   `list.activeBackground #bcc0cc` 동시): **전부 미달**. 최고값은 원래 채택된 mauve 자신
   (`#8839ef`, panelBg 4.45/activeBg **2.98**)과 red(`#d20f39`, 4.46/**2.99**) — 어느 것도
   activeBg 축에서 3.0 을 넘지 못한다(가장 근접한 두 후보조차 0.01~0.02 부족). d-36 §0 이 이미
   red 를 error 계열과의 정체성 충돌로 배제한 근거는 이 축에도 유효.
2. **Catppuccin 공식 확장 지침 유무 확인**: catppuccin.com 팔레트 페이지·업스트림 저장소 기준
   Catppuccin 은 flavor(Latte/Frappé/Macchiato/Mocha)당 26색 고정 목록만 정의하고, Tailwind
   의 50~900 같은 **악센트별 명도 스케일(tint/shade)을 공식적으로 제공하지 않는다** — 이는
   §0 실사가 이미 기록한 "yellow 계열 어두운 변형이 Latte 에 부재"와 같은 구조적 특징의
   일반화다. 다른 flavor(Mocha 등)의 mauve 는 라이트 배경용이 아니라(다크 배경에서 밝게
   보이도록 설계된 더 옅은 값) 방향이 반대라 대체 후보가 아니다.
3. **표준 팔레트 안에 해가 없고 공식 확장 체계도 없음** → §3-E (a)/(b) 분기: (b) "nord 형
   예외(수학적 불가 증명)"는 **성립하지 않는다** — A축이 제시한 후보(`#6d28d9` panelBg
   5.84/activeBg 3.91, `#6a3aad` 6.12/4.09)가 이미 팔레트 밖에서 **해가 실제로 존재함**을
   보여준다(nord 의 진짜 불가능 — 광도 양극단이 상충해 어떤 색도 두 배경을 동시에 만족 못함 —
   과는 다른 상황). 따라서 (a) "팔레트 밖 짙은 mauve 채택"이 유일하게 유효한 경로.
4. **값 도출 — A축 후보를 그대로 받지 않고 독립 도출**: A축의 두 후보를 HLS 로 역산한 결과
   `#6d28d9`(H 263.4°/S 0.70)·`#6a3aad`(H 265.0°/S 0.50) 는 Catppuccin mauve(`#8839ef`,
   H 266.0°/S 0.85)보다 **채도가 크게 낮아 색상환상 인접하지만 "같은 mauve" 라기보다 별개의
   보랏빛 회색**에 가깝다 — Catppuccin 팔레트 정체성(높은 채도의 파스텔 계열)과 어긋난다고
   판단해 그대로 채택하지 않았다. 대신 Catppuccin mauve 의 **hue(266.04°)·saturation(85.05%)
   을 정확히 보존**하고 HLS lightness 만 낮추는 방식으로 독립 도출했다(파이썬
   `colorsys.rgb_to_hls`/`hls_to_rgb` 로 재현). L 을 0.01 단위로 낮추며 두 배경 동시 3.0
   달성 지점을 스캔한 결과 L=0.570(`#8534ef`)에서 최초로 두 축 모두 통과(activeBg 3.09)하지만
   여유가 없어(원래 결함값 2.98 과 마찬가지로 근소 통과라 후속 배경색 미세조정에도 재발
   가능) 채택하지 않고, 원래 mauve 명도(L=0.580)에서 22.5% 지점을 낮춘 **L=0.450**(`#6611d4`)
   을 최종 채택 — panelBg **6.6517**(≈6.65)·activeBg **4.4495**(≈4.45), 둘 다 여유 있는 마진.
   같은 hue/saturation 이므로 "mauve 를 더 어둡게" 그 자체이며 새로운 색상을 들여온 게 아니다
   (identity 보존 근거). `app.foreground`(`#4c4f69`)와 hex 불일치(Rust 동일색 린트 통과) 확인.
   `light_colors()` 의 다른 mauve 사용처(bracketMatch·git.staged·lane4, d-36 §0 기록)는
   `panel.matchHighlight` 키만 바꿨으므로 영향 없음(단일 값 변경, 나머지 그대로 `#8839ef`).

**taide-dark 재확인**: `panel.matchHighlight #f9e2af` vs `list.activeBackground #45475a` =
7.18, `list.foreground #cdd6f4` vs 동일 배경 = 6.31 — 둘 다 §3-E 기록대로 이미 통과, 정정
불요를 재확인(신규 린트 2종 모두 taide-dark 위반 0건으로 직접 재검증).

**FAIL 재현(d-33/d-36 방식 — git stash/commit 미사용, 파일 편집만)**: 신규 게이트 2개를 각각
독립적으로 검증하기 위해 A축 정정 전 값 2건을 파일 편집만으로 일시 원복했다.

- `vscode-monokai-dimmed.json` `list.activeBackground` `#404040` → `#707070`(§3-B 정정 전
  값) → `cargo test -p taide domain::theme::service` 재실행 → **정확히 2건** FAILED:
  `카탈로그_테마는_panel_매치_하이라이트가_선택_행_배경과_최소_대비를_가진다`(vscode-monokai-dimmed
  1.82, §3-D "이전" 열과 일치) · `카탈로그_테마는_list_전경색이_선택_행_배경과_최소_대비를_가진다`
  (같은 배경을 공유하는 listFg 축도 함께 2.94 로 부수 실패 — 배경 하나를 되돌렸으니 그 배경을
  쓰는 두 쌍 모두 영향받는 게 정상, 결함 아님). 나머지 35개는 전부 PASS.
- 같은 배치에서 `nord.json` `list.foreground` `#2e3440` → `#d8dee9`(§3-B 정정 전 값, 계약
  예시와 동일)도 함께 원복 → 재실행 결과 `카탈로그_테마는_list_전경색이_선택_행_배경과_최소_대비를_가진다`
  위반 목록에 `nord`(1.48, §0/§5 문서 수치와 일치)가 `vscode-monokai-dimmed` 와 나란히 추가된
  것을 확인 — nord 는 `selectionMatchHighlight` 축만 예외 등재돼 있어 `selectionForeground`
  축은 예외 없이 정확히 잡혀야 하고, 실제로 그렇게 잡혔다(예외 스킵 로직이 쌍 단위로 정확히
  동작함을 확증).
- 두 파일을 파일 편집으로 원래 값(`#404040`·`#2e3440`) 복원 → `git diff` 로 두 파일의 diff 가
  A축이 이미 만든 diff(§3-B 표의 해당 두 행)와 **문자 그대로 동일**함을 재확인(왜곡·잔여
  변경 없음) → `cargo test -p taide domain::theme::service` 재실행, 37/37 PASS 재확인.

**검증**: 환경은 A축과 동일(`CARGO_HOME=$HOME/development/rust/cargo`,
`RUSTUP_HOME=$HOME/development/rust/rustup`).

1. `cargo fmt --all` 적용 → `cargo fmt --all --check` exit 0(클린).
2. `cargo test --workspace` 전량 green: `taide_lib` 1097 pass(`domain::theme::service` 37 —
   기존 33 + 신규 4)·`domain_boundaries` 3 pass·`session_restore` 6 pass·`taide_cli` 17 pass·
   doc-tests 0. 합계 1123 pass / 0 fail.
3. `cargo clippy --workspace --all-targets -- -D warnings` → 0건. `#[allow]` 사용 0.
4. FAIL 재현 — 위 문단대로 신규 2종 게이트가 각각 독립적으로 정확한 결함만 잡음을 확인.
5. `git diff src/shared/api/bindings.ts` → 0줄(실토큰 0).
6. `bun test` 전체 → 1487 pass / 0 fail / 2461 expect() — A축 산출물과 합쳐 무회귀(A축이
   기록한 수치와 정확히 동일). TS 예외 목록(`SELECTION_MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`
   nord·everforest-light·rose-pine-dawn / `SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS`
   rose-pine-dawn)과 Rust 예외 목록이 이름·구성·사유 문자열 전부 정합함을 위 "예외 목록"
   문단에서 대조 확인.
7. `git status --porcelain src-tauri/src` — `domain/theme/service.rs` 외 3건(`app/commands.rs`·
   `remote/dispatch.rs`·`sync/service.rs`)은 A축 §3-F 가 이미 "본 세션 무관"으로 기록한 바로
   그 d-41 dirty 상태이며, 본 세션도 그 세 파일을 Read/Edit/Write 어느 것도 하지 않았다(무접촉
   재확인). `src-tauri/resources/themes/*.json`·TS 파이프라인 4개 파일은 FAIL 재현 중 일시
   편집 후 정확히 원상 복구(위 "FAIL 재현" 문단의 `git diff` 대조로 확증) — 최종 diff 는 A축이
   이미 만든 13개 JSON 그대로, B축 추가 변경 없음.

---

## 4. 검토·판정 반영 (2026-08-25)

> 검토 wf_5f25e91b-0b6(3렌즈 opus+xhigh): major 클러스터 2(3렌즈/2렌즈 수렴)+L3-02·minor 8·
> info 2. 수정 wf_c3d84203-883. 원문 전문: 태스크 출력 `whbr6heiu.output`.

- **major A(확정 — 적대적 생략)**: nord `list.foreground` 정정(#2e3440)이 선택 행 1표면을
  살리며 hover·메뉴 11표면을 파괴(vs listBg 1.00·vs hover 1.24). 3렌즈 독립 수렴 + 메인 직접
  기계 재현(global.css :143-144 accent 매핑·nord 실값). 수리 사슬의 구조적 맹점(다중 표면
  미보호)도 동반 확정.
- **major B(확정 — 적대적 생략)**: 어드바이저리 수리 2패스가 통과 중이던 블로킹
  `panel.matchHighlight` 를 본문 전경 동일색으로 교체(d-33 결함 재도입). 2렌즈 수렴 + **메인
  이 실코드 probe 로 직접 재현**(#907aa9→#575279·validate []).
- **L3-02 해소(메인 발견)**: A축의 "nord 매치축 수학적 불가" 는 `list.activeBackground` 불변
  가정에서만 성립 — **팔레트 내 nord3 `#4c566a` 로 배경을 바꾸면 전경 5.46·매치 3.69·ΔE
  8.8/15.5 전부 통과**(메인 실측). 계약 §1-b "nord 최우선 정정" 을 예외 없이 충족하는 해로
  수정 1 에 반영 — nord 예외 등재는 철회.
- **taide-light `#6611d4` 수용 판정(메인)**: 계약 §1-b 의 두 갈래(스케일 내 재선정/예외) 밖
  제3경로이나, ① Latte 표준 14악센트 전수 미달 실측 증명 ② hue·sat 보존 도출 ③ 렌즈 ① 수치
  전량 재현 — 근거 충분으로 수용(L3-07 종결). 이 판정이 §1-b 문언에 대한 예외 승인 기록이다.
- 수정 목록: 계약 §4 수정 wf 지시 11건(§ 위 워크플로 스크립트가 정본 — nord 재정정·수리 가드
  강화+회귀 테스트·동일색 린트 신설·monokai-dimmed/abyss 재선정·everforest 사유 정정·
  불투명 린트·예외 레지스트리 유도·§8.2.3 등재·blocking JSDoc·픽스처 원복·재스윕 표 정정).

## 5. 이월 (기록)

- `list.foreground` vs `list.background`/`hoverBackground` **저대비(비동일) 게이트** 확장 —
  이번엔 동일색 린트만 신설(대비 게이트는 위반 규모 실측 후 별도 판단).
- `explorer.itemSelected` 표면의 전경 대비 가드 — nord·abyss 는 이번 일관 처리로 부분 해소,
  나머지 카탈로그 전수는 미측정(L3-09).
- `list.activeBackground` hex-동일 린트의 ΔE(≥2.3) 강화 여부(L3-03 — 선택 vs hover 어포던스).
- TAIDE `list.*` 스키마에 선택 행 전용 전경 토큰(`list.activeSelectionForeground` 대응) 신설
  여부 — 업스트림 다수가 보유, 현 스키마는 단일 `list.foreground` 공용(major A 의 근본 원인).

### §4 보충 — 수정 결과 (2026-08-25)

- 11건 전건 반영(수정 wf_c3d84203-883 §3 계약 내 기록·검증 수치 포함). 특기: **vscode-abyss
  재선정은 보류** — 팔레트 44색 전수 탐색에서 4조건 동시 만족값이 현행 `#000c18` 뿐(보류가
  잘못된 수정보다 낫다 원칙). nord FAIL 재현으로 신규 동일색 린트 확증. 최종 기준선: bun
  1490·Rust 1125(theme 39)·bindings 0.
- §5 이월 추가: explorer.itemSelected 분리 잔존 4종(abyss·monokai-dimmed·solarized-dark·
  tomorrow-night-blue — 선재·게이트 범위 외)·abyss 선택 행 ΔE 어포던스(팔레트 내 해 부재).
