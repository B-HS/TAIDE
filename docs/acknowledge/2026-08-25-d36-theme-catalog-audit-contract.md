# d-36 — 테마 카탈로그 전수검사 + builtin_light 정정 계약 (2026-08-25)

> 정본: `2026-08-25-post-batch-user-decisions.md` §1(사용자: "알아서 잘 고치고 모든 테마들을
> 전수검사") / 이월 원문 d-31 계약 §5.2(builtin_light 2.15·결정 옵션 2)·§5.3(예외 2종) /
> d-33 §3-R(hex 동일성 2차 게이트 선례). 번들 재변환 없음(§8.2.3 정본 — 재론 금지).

## 0. 착수 전 메인 실사 (2026-08-25)

- `taide-light`(`service.rs` `light_colors()`) `panel.matchHighlight` `#df8e1d` vs
  `panel.background` `#e6e9ef` = **2.15**(카탈로그 38종 최저). `taide-dark` 는 `#f9e2af` vs
  `#181825` = 13.81(문제 없음).
- Catppuccin Latte 악센트 14종 전수 대비 실측(vs `#e6e9ef`): 게이트(3) 통과는 **5종** —
  red `#d20f39` 4.46·mauve `#8839ef` 4.45·blue `#1e66f5` 4.04·maroon `#e64553` 3.23·teal
  `#179299` 3.08 (**§4 검토 L1 정정**: 초판 "셋뿐"은 실측표를 잘못 요약한 오기 — maroon·teal
  2종 누락). yellow 계열 어두운 변형은 Latte 에 부재(d-31 §5.2 예상과 일치). 배제 사유:
  red = error 계열(statusIndicator.error·git.deleted) → rose-pine-dawn `love` 배제 선례,
  blue = 앱 메인 accent(app.accent·focusBorder·tabActiveIndicator) → 혼동, maroon = red 색상
  인접(error 혼동 논리 동일 적용), teal = 3.08 게이트 턱걸이(마진 부족). **mauve `#8839ef`
  채택**(bracketMatch·git.staged·lane4 에 쓰이나 error/accent 정체성 충돌 없음, vs
  `app.foreground #4c4f69` 대비 1.47 이지만 이 축은 대비가 아니라 ΔE 구별성 기준 — hue 상
  명백 구별, 구현이 deltaE76 실측 기록. §4 검토 info: 선택 행 표면 `list.activeBackground
  #bcc0cc` 기준으로는 2.98 로 3 미만이나 Latte 내 전 후보가 그 표면에서 3 미만이고 결함값
  1.44 대비 2배 이상 개선 — 게이트 짝 확장 논의는 §4 major 처분과 함께).
- 기존 Rust 린트 5종(`파싱·이름`/`app 전경≠배경`/`list 활성 배경 구분`/`matchHighlight 불투명`/
  `matchHighlight≠app.foreground`)은 전부 `bundled_themes()`(36종 JSON)만 순회 — **builtin
  2종은 어떤 기계 검사에도 미포함**(d-31 §5.2 ②의 구조적 갭 실측 재확인).
- TS `bundled-theme-contrast.test.ts` 는 36종 × CONTRAST_PAIRS 5쌍 커버(예외 2종
  `MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS` 등재) — builtin 은 Rust 하드코딩이라 TS 측 커버 불가,
  Rust 측 신설이 정답.

## 1. 범위 (사용자 승인 — 값 선정 위임)

- **a. builtin_light 정정**: `light_colors()` `panel.matchHighlight` `#df8e1d` → `#8839ef`
  (mauve). 구현이 채택 전 재검증: 게이트(3) 통과·`app.foreground` 와 ΔE 구별성(TS 기준 2.3
  초과)·8자리 아닌 불투명 6자리. 다른 키는 무변경.
- **b. 검사 순회를 카탈로그 38종으로 확장**: 기존 린트 5종의 순회를 `bundled_themes()` +
  `builtin_dark()` + `builtin_light()` 로 확장(공용 헬퍼 1개로 — 테스트 전용). 확장으로
  builtin 이 기존 린트를 위반하면 **데이터 정정이 원칙**(예외 등재는 업스트림 대체 부재
  사유가 있을 때만 — d-31 §5.3 선례).
- **c. matchHighlight 대비 게이트(3) 린트 신설**: 38종 전수 — WCAG 상대 휘도·대비비 계산을
  Rust 로 이식(상수 `MATCH_HIGHLIGHT_MIN_CONTRAST = 3.0` + 근거 doc — TS `MIN_CONTRAST_RATIO`
  와 동일 값·동일 근거 명시). 예외 2종(everforest-light `#8da101` 2.69·rose-pine-dawn
  `#d7827e` 2.60)은 TS 예외 목록과 동일 사유 문자열로 등재.
- **d. 검증 게이트**: cargo fmt/clippy(-D warnings)/test 전량 + FAIL 재현(taide-light 를
  HEAD 값으로 일시 원복해 신규 린트가 정확히 그 1건을 잡는지 — d-33 §3-R 방식, stash 금지) +
  bindings diff 실토큰 0 + `bun test`(TS 무접촉이므로 무회귀 확인만).
- 범위 외: 번들 JSON 수정·재변환(§8.2.3 정본)·TS 파이프라인 변경·CIE76 Rust 전체 이식(hex
  동일성 게이트 유지 — d-33 역할 분담 재론 금지)·다른 색 키 정정.

## 2. 실행·검증

- 구현: Rust 단독 1 에이전트(sonnet+xhigh) — §3 에 기록 필수. cargo 후 fmt.
- 검토 3렌즈(opus+xhigh): ① 대비 수학(휘도 계산 이식 정확성 — TS `contrast.ts` 와 교차 대조·
  mauve 채택 근거 재검증) ② 회귀 동등성(순회 확장이 기존 36종 판정 불변·기존 테스트 전량
  green) ③ 컨벤션(매직넘버·doc·`mod tests` 관행·`#[allow]` 0). major 적대적(opus+high) →
  수정 → 메인 2차(verify 직접). 커밋은 사용자 규칙(요청 시).

---

## 3. 구현 완료 기록 (구현 에이전트가 기록)

구현 Rust 단독 1 에이전트(sonnet+xhigh), 2026-08-25. 변경 파일은
`src-tauri/src/domain/theme/service.rs` 1개(207 삽입/27 삭제, `git diff --stat` 확인) —
계약 범위 외 파일(번들 JSON·TS 파이프라인) 무접촉을 `git status` 로 확인했다.

### 3.1 a — builtin_light `panel.matchHighlight` 값 채택 재검증

`light_colors()`(494행 부근, 실제 리터럴은 491행) `panel.matchHighlight` 를 `#df8e1d` →
`#8839ef`(mauve)로 변경. 채택 전 3게이트를 TS `contrast.ts`/`color.ts` 의 공식 그대로(WCAG
상대휘도 임계 0.03928·CIE76 D65 선형화 임계 0.04045 각각 정확히 재현) Python 으로 독립
재구현해 재검증했다(스크래치패드 `compute_colors.py` — 검증 후 파일은 세션 스크래치패드에만
존재, 저장소에는 미포함):

| 항목 | 값 | 게이트 | 판정 |
|---|---:|---|---|
| 대비(`#8839ef` vs `panel.background #e6e9ef`) | **4.4519**(≈4.45) | 게이트(3) `MATCH_HIGHLIGHT_MIN_CONTRAST = 3.0` | 통과 |
| ΔE76(`#8839ef` vs `app.foreground #4c4f69`) | **88.74** | TS 구별성 기준 2.3 초과 | 통과(압도적 구별) |
| hex 형식 | `#8839ef`(6자리) | 8자리(알파) 아닌 불투명 6자리 | 통과 |

재검증 스크립트는 계약 §0 이 이미 기록한 수치들(red `#d20f39` 4.46·blue `#1e66f5` 4.04·
everforest-light 예외 2.69·rose-pine-dawn 예외 2.60·taide-light 결함값 2.15·taide-dark
13.81)을 전부 재현해 formula 이식이 정확함을 교차 확인한 뒤 mauve 값을 계산했다 — 단 이는
§0 에 **기록된 값의 재현**이지 14종 sweep 재실행이 아니어서 §0 초판의 "셋뿐" 오기를 걸러내지
못했다(§4 검토 L1 정정 참조). (각각
4.4643·4.0397·2.6923·2.6041·2.1517·13.8149 — 계약 §0 표기 반올림과 일치).

### 3.2 b — 기존 린트 5종의 순회를 카탈로그 38종으로 확장

`mod tests` 안에 테스트 전용 공용 헬퍼 `theme_catalog()`(1001행)를 신설했다:

```rust
fn theme_catalog() -> Vec<Theme> {
    bundled_themes().into_iter().chain([builtin_dark(), builtin_light()]).collect()
}
```

기존 5개 린트를 전부 `bundled_themes()` → `theme_catalog()` 순회로 교체하고, "번들" 단어가
이제 사실과 어긋나므로(빌트인 2종도 순회 대상) 테스트명을 `번들_테마는_` → `카탈로그_테마는_`
로 정정했다(common.md "네이밍은 정확하게" — 순회 대상이 바뀌었는데 이름이 "번들"로 남으면
오해를 유발). 관련 exemption 상수(`LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS`)와 §3.4 매치
하이라이트 동일색 린트 위의 doc 주석도 "36종"→"38종(36 번들+2 빌트인)", "bundled theme"→
"catalog theme" 로 함께 정정(수정하는 코드에 붙은 낡은 문언 방치 금지 원칙).

| 순번 | 테스트(신규 이름) | 라인 | 확장 결과 |
|---|---|---:|---|
| 1 | `카탈로그_테마는_모두_파싱되고_이름이_비어있지_않다` | 1242 | 빌트인 위반 0(파싱 카운트 assert 는 `bundled_themes()` 전용으로 유지, name/extends 루프만 38종으로 확장) |
| 2 | `카탈로그_테마는_app_전경색과_배경색이_서로_다르다` | 1290 | 빌트인 위반 0 |
| 3 | `카탈로그_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다` | 1310 | 빌트인 위반 0(exemption 목록 계속 공백) |
| 4 | `카탈로그_테마는_panel_매치_하이라이트가_불투명하다` | 1345 | 빌트인 위반 0(빌트인은 항상 6자리 hex 리터럴) |
| 5 | `카탈로그_테마는_panel_매치_하이라이트가_app_전경색과_동일하지_않다` | 1387 | 빌트인 위반 0 |

**builtin 위반 발견·처분**: 5종 확장 결과 **위반 0건**. 이번 배치가 a 항목으로 taide-light
`panel.matchHighlight` 를 이미 정정했으므로(대비 문제였지 opacity/동일색/배경구분 문제가
아니었음), 확장된 5개 린트 자체는 빌트인에서 새 위반을 내지 않았다 — "데이터 정정이 원칙"
조항이 실제로 발동할 사례가 없어 예외 등재도 발생하지 않았다.

### 3.3 c — matchHighlight vs panel.background 대비 게이트(3) 린트 신설

**WCAG 이식**: TS `contrast.ts` 의 `relativeLuminance`/`contrastRatio`(및 그 하위
`srgbChannelToLinear`)를 Rust 함수로 그대로 이식 — `hex_to_rgb`(1439행, 기존
`normalize_hex_color` 를 재사용해 8자리로 정규화한 뒤 앞 6자리만 파싱, 알파는 무시)·
`srgb_channel_to_linear`(1453행)·`relative_luminance`(1463행)·`contrast_ratio`(1474행).
상수(`SRGB_LINEAR_THRESHOLD=0.03928` 등 — 초판 "7개"는 오기, 실제 **9개**: §4 L3-3 정정)는 TS `contrast.ts` 리터럴과 값을 동일하게
맞췄다. **알파 합성(`compositeOverBackground`) 은 의도적으로 미이식** — doc(1425행 부근)에
근거 기록: ①직전 린트(불투명 게이트)가 이미 반투명 matchHighlight 를 걸러내고 ②38종 전량의
`panel.background` 가 전부 6자리 불투명 hex 임을 직접 확인(스크립트로 36개 JSON 전수 스캔,
이상 0건)했으므로 이 게이트가 받는 입력에서 합성은 항상 항등이다.

**상수**: `MATCH_HIGHLIGHT_MIN_CONTRAST: f64 = 3.0`(1417행) — TS `MIN_CONTRAST_RATIO`
(`contrast.ts:14`)와 동일 값. 영어 `///` doc 로 "WCAG 2.x 1.4.11(non-text contrast) 3:1
최소치, matchHighlight 는 본문 텍스트가 아니라 전경 강조 토큰이므로 이 기준을 적용" 근거를
명시(TS 소스 자체에는 comments.md 금지 규정상 인라인 근거 주석이 없어, WCAG 공식 근거를
직접 기술).

**예외 2종**: `MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`(1492행, `&[(&str, &str)]`)에
everforest-light(`#8da101`, 2.69)·rose-pine-dawn(`#d7827e`, 2.60)를 TS
`MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`(`bundled-theme-contrast.test.ts`)와 문자 그대로
동일한 사유 문자열로 등재.

**신규 테스트 2개**:
- `카탈로그_테마는_panel_매치_하이라이트가_패널_배경과_최소_대비를_가진다`(1513행) — 38종
  전수(예외 2종 skip), 위반을 모아 한 번에 assert(기존 5개 린트와 동일 관행).
- `매치_하이라이트_대비_예외_등재분은_실제로_최소_대비에_미달한다`(1548행) — TS
  `bundled-theme-contrast.test.ts` 의 "예외 등재분은 실제로 대비 부족 사유로만 위반한다"
  선례를 Rust 에도 반영, exemption 목록이 stale 해지는 것을 방지하는 회귀 가드.

두 신규 테스트 모두 taide-dark(13.81)·taide-light 정정값(4.45) 포함 38종 전량 대상 실행에서
통과.

### 3.4 d — 검증 게이트

환경: `CARGO_HOME=$HOME/development/rust/cargo`·`RUSTUP_HOME=$HOME/development/rust/rustup`
(cargo 1.97.1 / rustc 1.97.1).

1. **cargo fmt --all** 적용 → **cargo fmt --all --check** exit 0(클린).
2. **cargo test --workspace** 전량 green: `taide_lib` 1082 pass(`domain::theme::service`
   33 — 기존 31 + 신규 2)·`domain_boundaries` 3 pass·`session_restore` 6 pass·`taide_cli`
   17 pass·doc-tests 0. 합계 1108 pass / 0 fail.
3. **cargo clippy --workspace --all-targets -- -D warnings** → 0건(경고·에러 없음).
   `#[allow]` 사용 0.
4. **FAIL 재현**(d-33 §3-R 방식 — git stash/commit 미사용, 파일 편집만): 491행
   `("panel.matchHighlight", "#8839ef")` 을 `("panel.matchHighlight", "#df8e1d")` 로 일시
   원복 → `cargo test -p taide domain::theme::service` 재실행 → **정확히 1건**
   FAILED(`카탈로그_테마는_panel_매치_하이라이트가_패널_배경과_최소_대비를_가진다`), 나머지
   32개(신규 5종 린트의 다른 4개 포함) 전부 PASS:
   ```
   thread '...카탈로그_테마는_panel_매치_하이라이트가_패널_배경과_최소_대비를_가진다' panicked at
   src-tauri/src/domain/theme/service.rs:1540:9:
   panel.matchHighlight contrast defects in catalog themes:
   'taide-light': panel.matchHighlight("#df8e1d") vs panel.background("#e6e9ef") = 2.15 (최소 3)
   test result: FAILED. 32 passed; 1 failed
   ```
   확인 후 491행을 `#8839ef` 로 파일 편집 복원 → `git diff` 로 원래 diff(207 삽입/27 삭제,
   matchHighlight 리터럴 변경은 정확히 1줄)와 동일함을 재확인(왜곡 없음) → `cargo test -p taide
   domain::theme::service` 재실행 33 pass 재확인.
5. **git diff src/shared/api/bindings.ts** → 0줄(실토큰 0, TS 파이프라인 무접촉 확인).
6. **bun test** 전체 → 1481 pass / 0 fail / 2444 expect() — TS 무접촉이므로 예상대로 무회귀.

### 3.5 특이사항·openIssues

- 카탈로그 38종 전수에서 확장된 5개 기존 린트가 빌트인 위반을 내지 않아, "데이터 정정 원칙
  vs 예외 등재" 분기가 실제로는 발동하지 않았다(§3.2). 대비 게이트(3) 신설(c)만이 빌트인
  taide-light 의 실결함을 잡았고, 그 결함은 이미 a 로 정정된 상태다.
- `hex_to_rgb`/`srgb_channel_to_linear`/`relative_luminance`/`contrast_ratio` 는 CIE76
  ΔE 가 아니라 WCAG 상대휘도·대비비만 이식했다(계약 §1 "CIE76 Rust 전체 이식 금지" 준수) —
  구별성(ΔE) 판정은 여전히 TS 전용(기존 `번들_테마는_panel_매치_하이라이트가_app_전경색과_동일하지_않다`
  → 개명 `카탈로그_테마는_...` 의 hex-동일성 2차 게이트가 Rust 측 유일한 구별성 근사치라는
  구조는 d-33 §3-R 결정 그대로 유지).


---

## 4. 검토·적대적 반영 (2026-08-25)

> 검토 wf_6742670a-7ea(3렌즈 opus+xhigh): major 1·minor 4·info 5. 적대적 wf_445a7e5d-e65
> (opus+high): major **downgraded**. 수정 wf_6dfe49e7-f92(sonnet+xhigh) — minor·info 5건.
> 원문 전문: 태스크 출력 `w46cuu0zk.output`(검토)·`wcju7mcdr.output`(적대적, 세션 스크래치).

- **major d36-l1-gate-pair(→downgraded, 코드·데이터 무변경)**: 렌더 경로·nord 1.00 은 확증
  (팔레트 선택 행 = `list.activeBackground` 배경 + `text-panel-match-highlight` 전경, 덮어쓰기
  경로 부재). 그러나 ① 발견 수치표는 알파 미합성 오류(8종 아님 — 합성 정정 시 7/38, 그중
  vscode-tomorrow-night-blue 1.44 는 실제 3.46 통과) ② nord 선택 행은 **일반 글자부터 1.48**
  로 판독 불가라 matchHighlight 짝은 독립 원인이 아닌 하위 증상 ③ 제안 짝 확장은 최악 사례
  palenight(일반 글자 1.13, 매치축은 3.48 통과)를 못 잡음 ④ 선재 데이터(nord.json 은
  bc27352 이후 무변경)·이번 diff 는 taide-light 해당 축을 1.44→2.98 로 개선 — 재프레이밍해
  §5 이월. 비선택 행 표면은 `command-palette.tsx:285` 가 `bg-panel-background` 오버라이드라
  현행 게이트 짝이 지배 표면에 대해 정확함도 확인됨.
- minor 4(L2-01 메시지 잔존·L3-1 exemption doc 인과 오서술·L3-2 RGB_CHANNEL_MAX 누락·
  §0 Latte "셋뿐" 오기)·info 5 중 코드 5건은 fixer, 문서 3건(§0 정정·§3 오기 마커 2건·시점
  문서 각주 2건)은 메인 직접 반영. d36-L2-02(과거 계약 구 테스트명)는 각주 2건으로 종결.

## 5. 이월 — 선택 행 표면(`list.activeBackground`) 전경 대비 가드 부재 (사용자 결정 필요)

> 적대적 검증이 결함 클래스를 재정의: "matchHighlight 짝 누락"이 아니라 **선택 행 배경에
> 대한 전경 대비 가드가 매치 전경·일반 전경 양축 모두 없음**. 이번 배치 범위 외(선재 데이터·
> §1 봉인)이므로 코드·데이터 무변경, 아래 기록만.

- **정정 수치(알파 합성 적용 — `color.ts` `compositeOverBackground` 기준)**:
  - `panel.matchHighlight` vs `list.activeBackground` < 3 = **7/38**(비예외 5: nord **1.00** ·
    vscode-monokai-dimmed 1.82 · vscode-abyss 2.18 · vscode-quiet-light 2.59 · taide-light
    2.98 / 예외 등재분: rose-pine-dawn 2.36 · everforest-light 2.45).
  - `list.foreground` vs `list.activeBackground` < 3 = **8/38**, < 4.5 = 15/38 — 최악
    **palenight 1.13** · nord 1.48 · ayu-dark 1.99 · ayu-light 2.19.
- **nord 특이**: `panel.matchHighlight == list.activeBackground == app.accent ==
  explorer.itemSelected == #88c0d0` 완전 동일 — 선택 링(`ring-app-accent`) 2차 단서까지 동시
  무력화(`2026-08-20-palette-ux-contract.md` §4 전제가 nord 에서 불성립).
- **결정 갈래(다음 확인 시 제시)**: ① 최소·즉효 — nord 동일 hex 는 예외 사유 불성립의 명백한
  데이터 결함이므로 동일성 짝 린트 1개 추가 + nord.json 정정(권장) ② 정공법 — 선택 행 전경
  대비 게이트 신설(TS·Rust 동시) + 위반 5~8종 업스트림 스케일 내 데이터 정정(독립 배치 규모)
  ③ 아무 것도 안 함(선재 수용). 어느 쪽이든 §1 봉인(번들 JSON 범위 외) 해제가 전제.

> [확정 2026-08-25] 사용자 결정: **② 정공법 채택**("전면 게이트+데이터 정정") — d-40 배치로 착수(PROCESS 큐 등재). ①(nord 즉효만)·③(보류)은 기각.
