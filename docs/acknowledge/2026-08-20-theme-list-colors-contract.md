# 번들 테마 list 색 결함 수정 계약 (2026-08-20, d-26b)

> 사용자 실기 보고(2026-08-20): d-26 의 팔레트 선택 하이라이트 수정 후에도 "여전히 안보이는데
> 작동은 하나봐. 색상코드 잘 확인해봐" — 적중. 메인 실측: twMerge 병합 정상(bg-accent 제거
> 확인)·CSS 배선 정상. **근본 원인 = 테마 데이터**: 사용자 활성 테마 darcula 가
> `list.hoverBackground`·`list.activeBackground`·`panel.background` 를 전부 #242424 로 정의.
> 전 번들 스윕(36개): **결함 12개** —
> 삼중 동일(선택 완전 비가시): darcula(#242424×3)·vscode-dark-plus(#1E1E1E×3) /
> active==panel: vscode-light-plus / hover==panel: night-owl /
> active==hover(선택·hover 구분 불가): ayu-dark·ayu-light·gruvbox-dark·night-owl-light·
> one-dark-pro·vitesse-dark·vitesse-light·vscode-dark-modern.
> 정황: 테마 포팅(theme-convert) 시 원본 키 부재·오매핑의 폴백 산물로 추정 — 원인 실사 포함.

## 1. 범위

- **12개 테마의 list.hoverBackground·list.activeBackground 를 업스트림 충실 값으로 정정**:
  각 테마의 원전(VS Code 기본 테마의 `list.activeSelectionBackground`/`list.hoverBackground`,
  서드파티 테마의 공식 저장소 값)을 조사해 반영. 원전에서 값이 실재하는데 포팅이 누락/오매핑한
  것이면 그 값, 원전 자체가 미정의면 테마 팔레트 정합 파생값(배경·전경 혼합)을 채택하고 산출
  근거 기록. active 와 hover 는 구분되는 값이어야 함(VS Code 관례).
- **theme-convert 파이프라인 원인 실사**: `shared/lib/theme-convert/mapping-tables` 가 이
  결함을 만든 매핑/폴백인지 확인 — 맞으면 매핑 정정(향후 가져오기에도 적용), 아니면 데이터만.
- **데이터 린트 테스트(Rust)**: 번들 테마 전수에 대해 `list.activeBackground != panel.background`
  && `list.activeBackground != list.hoverBackground` 기계 강제(재발 방지) — alpha 표기(#rrggbbaa)
  정규화 비교. 예외가 정당한 테마가 있으면 사유와 함께 화이트리스트.
- 범위 외: list 외 토큰 전수 감사(R5#9 테마 토큰 파리티 — 기존 이월 유지)·테마 에디터 UI.
- **주의 고지**: resources/themes 는 include_str! — 수정 시 Rust 재빌드로 dev 앱 재시작.

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독): 업스트림 값 조사(웹 가용 시 공식 저장소·불가 시 로컬
  근거) → 12개 데이터 정정 → 파이프라인 실사 → Rust 린트 테스트 → cargo fmt → verify +
  vite build exit 0 + bindings 무변경.
- 검토: 2렌즈(데이터 충실성 — 업스트림 대조 독립 재검증 / 계약 — 린트 실효·표면 무변경) +
  major 적대적. 이후 메인 2차 → 커밋 → 병합.
- 실기 확증(사용자): darcula 에서 ⌘P 선택 가시·탐색기 hover/선택 구분.

---

## 3. 구현 완료 기록 (검토 전)

### 3.1 12개 테마 정정 표

업스트림 원전(각 테마의 W7 취득 매니페스트 — `docs/utils/2026-08-12-w7-theme-original-sources.md`
경로로 재취득해 `colors["list.hoverBackground"]`/`colors["list.activeSelectionBackground"]` 를
직접 대조)을 기준으로 정정했다. `#2A2D2E`/`#04395E`(dark)·`#ADD6FF`(light)는 이미
`mapping-tables.ts` 의 `VSCODE_LIST_HOVER_BACKGROUND_DEFAULT`/
`VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT` 로 코드베이스에 존재하던 VS Code 공식
레지스트리 기본값이며(라이트의 `#ADD6FF` 는 VS Code 실제 기본값 `#0060C0` 이 아니라 대비 확보를
위해 이미 채택된 자체 정책값 — 해당 상수의 기존 JSDoc 참조), 번들 `explorer.itemHover`/
`explorer.itemSelected` 가 이미 이 값으로 정확히 계산되어 있어 대조군으로 썼다.

| 테마 | 결함 유형 | hover 이전→이후 | active 이전→이후 | 근거(sourceBasis) |
|---|---|---|---|---|
| darcula | 삼중 동일 | `#242424`→`#2A2D2E` | `#242424`→`#04395E` | 업스트림(rokoroku/vscode-theme-darcula)은 `list.*` 미정의(22개 색만 정의) — VS Code dark 레지스트리 기본값 채택. `explorer.itemHover/itemSelected` 가 이미 이 값으로 계산되어 있어 일치 확인 |
| vscode-dark-plus | 삼중 동일 | `#1E1E1E`→`#2A2D2E` | `#1E1E1E`→`#04395E` | 업스트림 `dark_plus.json`→`dark_vs.json` 체인 모두 `list.hoverBackground`/`list.activeSelectionBackground` 미정의 — 실제 VS Code Dark+ 는 레지스트리 기본값(`#2A2D2E`/`#04395E`)으로 렌더됨. explorer 대조 일치 |
| vscode-dark-modern | active==hover | `#1F1F1F`→`#2A2D2E` | `#1F1F1F`→`#04395E` | 업스트림 `dark_modern.json` 은 `dark_plus.json` 을 include 하며 `list.*` 오버라이드 없음(`dropdown.listBackground` 만 추가) — dark-plus 와 동일 근본원인. explorer 대조 일치 |
| vscode-light-plus | active==panel | 변경 없음(`#E8E8E8` 유지) | `#FFFFFF`→`#ADD6FF` | 업스트림 `light_vs.json` 은 `list.hoverBackground=#E8E8E8` 만 정의, `list.activeSelectionBackground` 미정의 — VS Code 실제 기본값(`#0060C0`)은 `list.activeSelectionForeground`(백색)와 짝을 이루나 TAIDE list 토큰엔 대응 전경색이 없어 대비 부족(§ mapping-tables.ts 기존 주석) → 코드베이스가 이미 채택한 `#ADD6FF` 로 통일. explorer.itemSelected 대조 일치 |
| night-owl | hover==panel | `#011627`→`#2A2D2E` | 변경 없음(`#234d708c` 유지) | 업스트림(sdras/night-owl-vscode-theme)이 `list.hoverBackground=#011627` 로 `editor.background` 와 동일값 정의 — 업스트림 자체의 사실상 미기능 hover. `list.activeSelectionBackground=#234d708c` 는 배경과 구분되어 유지. VS Code 기본값으로 대체. explorer.itemHover 대조 일치 |
| ayu-dark | active==hover | 변경 없음(`#47526640` 유지) | `#47526640`→`#04395E` | 업스트림(ayu-theme/vscode-ayu)이 `list.hoverBackground`=`list.activeSelectionBackground`=`#47526640` 로 동일 정의(의도된 단일톤 UI) — hover 는 유지하고 active 만 VS Code 기본값으로 강화해 두 상태를 구분 |
| ayu-light | active==hover | 변경 없음(`#6b7d8f24` 유지) | `#6b7d8f24`→`#ADD6FF` | 상동(ayu-light, `#6b7d8f24` 동일값) |
| gruvbox-dark | active==hover | 변경 없음(`#3c383680` 유지) | `#3c383680`→`#04395E` | 업스트림(jdinhify/vscode-theme-gruvbox v1.22.0)이 hover/active 를 `#3c383680` 로 동일 정의 |
| night-owl-light | active==hover | 변경 없음(`#d3e8f8` 유지) | `#d3e8f8`→`#ADD6FF` | 업스트림(sdras, Night Owl-Light)이 hover/active 를 `#d3e8f8` 로 동일 정의 |
| one-dark-pro | active==hover | 변경 없음(`#2c313a` 유지) | `#2c313a`→`#04395E` | 업스트림(Binaryify/OneDark-Pro)이 hover/active 를 `#2c313a` 로 동일 정의 |
| vitesse-dark | active==hover | 변경 없음(`#181818` 유지) | `#181818`→`#04395E` | 업스트림(antfu/vscode-theme-vitesse)이 hover/active 를 `#181818` 로 동일 정의 |
| vitesse-light | active==hover | 변경 없음(`#f7f7f7` 유지) | `#f7f7f7`→`#ADD6FF` | 업스트림(vitesse-light)이 hover/active 를 `#f7f7f7` 로 동일 정의 |

원칙: hover 는 업스트림이 배경과 구분되는 값을 정의한 한 그대로 보존(사용자가 이미 보고 있던
동작을 바꾸지 않음), active 만 VS Code 관례대로 "hover 보다 강한 대비"를 만족하도록 정정.
`darcula`·`vscode-dark-plus`·`vscode-dark-modern` 3개는 hover 도 업스트림에 값 자체가 없어
함께 정정했다.

### 3.2 파이프라인 실사 결과 — mapping-tables.ts 가 원인이었다

`src/shared/lib/theme-convert/mapping-tables.ts` 의 `COLOR_MAPPING` 을 확인한 결과, 이 결함은
**파이프라인이 만든 것으로 확인**했다(데이터만의 우연한 결함이 아님):

- `explorer.itemHover`/`explorer.itemSelected` (444번대 줄)는 이미 `derived()` + `isUsableListBackground`
  가드로 구현되어 있어, 소스 값이 없거나(undefined) 배경과 동일하면 `VSCODE_LIST_HOVER_BACKGROUND_DEFAULT`/
  `VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT` 로 안전하게 대체된다.
- 반면 `list.hoverBackground`/`list.activeBackground` (구 588~590번대 줄)는 가드 없는 단순
  `chain(['list.hoverBackground'])`/`chain(['list.activeSelectionBackground'])` 였다. 후보가
  없으면 `resolveColorEntry` 가 `FAMILY_FALLBACK_SOURCE_KEYS.background = ['editor.background']`
  로 조용히 폴백해, `list.background`/`panel.background` 와 동일한 색이 되어버린다 — 정확히
  darcula/vscode-dark-plus/vscode-dark-modern 에서 관측된 증상.
- **수정**: `list.hoverBackground`/`list.activeBackground` 를 `explorer.*` 와 동일한
  `derived()` + `isUsableListBackground` 패턴으로 전환(향후 VSIX 가져오기·`themes:convert`
  재변환에도 적용됨). `list.activeBackground` 는 추가로 이미 resolve 된 `list.hoverBackground`
  와도 `isUsableListBackground` 로 비교해, 업스트림이 hover==active 로 동일 정의한 8개 테마
  패턴(§3.1 의 active==hover 그룹)도 향후 가져오기에서 같은 방식으로 방지한다.
- 변경 파일: `src/shared/lib/theme-convert/mapping-tables.ts` (COLOR_MAPPING 의 `list.*` 3개
  엔트리, 그 중 2개를 `chain`→`derived` 전환). `resolve-colors.ts`/`isUsableListBackground` 자체는
  무변경(기존 가드 재사용).

### 3.3 데이터 린트 테스트 — 재현 후 정정

`src-tauri/src/domain/theme/service.rs` 의 `#[cfg(test)] mod tests` 에 추가:

- `normalize_hex_color`: 소문자화 + 6자리 hex 는 `ff` 알파를 붙여 8자리로 정규화(요청대로
  alpha 는 제거하지 않음 — `#47526640` 같은 반투명값은 그대로 보존해 불투명값과 구분 유지).
- `번들_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다`: 번들 전 테마를 순회해
  `list.activeBackground != panel.background` && `!= list.hoverBackground` 를 정규화 비교로
  강제. 위반은 모아서 한 번에 assert(테마 하나 실패 시 나머지를 가리지 않음).

> [d-36 각주 2026-08-25] 위 테스트명은 d-36 에서 `카탈로그_테마는_...` 로 개명되고 순회 범위가 38종(번들 36+빌트인 2)으로 확장됨 — 정본은 `2026-08-25-d36-theme-catalog-audit-contract.md` §3.2.
- `LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS`: 화이트리스트 상수. **현재 비어 있음** — 정정 후
  36개 번들 테마 전량이 예외 없이 통과.

**재현(수정 전 FAIL 실측)**: 12개 JSON 정정본을 `git show HEAD:<path>` 로 일시 원복한 뒤(작업
트리 조작만, git stash/commit 미사용) 새 테스트를 실행 — 정확히 계약 §서두가 보고한 12개 테마
(13건: darcula·vscode-dark-plus 는 각 2건씩)에서 실패함을 실측했다:

```
'one-dark-pro': list.activeBackground(Some("#2c313a")) == list.hoverBackground
'gruvbox-dark': list.activeBackground(Some("#3c383680")) == list.hoverBackground
'ayu-dark': list.activeBackground(Some("#47526640")) == list.hoverBackground
'ayu-light': list.activeBackground(Some("#6b7d8f24")) == list.hoverBackground
'night-owl-light': list.activeBackground(Some("#d3e8f8")) == list.hoverBackground
'vitesse-dark': list.activeBackground(Some("#181818")) == list.hoverBackground
'vitesse-light': list.activeBackground(Some("#f7f7f7")) == list.hoverBackground
'vscode-dark-plus': list.activeBackground(Some("#1E1E1E")) == panel.background
'vscode-dark-plus': list.activeBackground(Some("#1E1E1E")) == list.hoverBackground
'vscode-light-plus': list.activeBackground(Some("#FFFFFF")) == panel.background
'vscode-dark-modern': list.activeBackground(Some("#1F1F1F")) == list.hoverBackground
'darcula': list.activeBackground(Some("#242424")) == panel.background
'darcula': list.activeBackground(Some("#242424")) == list.hoverBackground
```

정정본을 복원한 뒤 재실행하여 PASS 확인. 다른 24개 번들 테마는 정정 전후 모두 위반 없음(전수
스윕 별도 스크립트로 교차 확인).

### 3.4 검증

- `cargo test --package taide domain::theme::service` — 27개 전량 통과(신규 린트 테스트 포함).
- `cargo fmt --all --check` — 통과.
- `bun run verify`(typecheck → eslint → prettier check → bun test 1421건 → cargo fmt →
  cargo clippy -D warnings → cargo test 전 워크스페이스) — exit 0. eslint 경고 6건은 모두
  이번 변경과 무관한 기존 파일(file-tree.tsx 등)의 사전 경고.
- `bunx vite build` — exit 0.
- `git diff --stat -- src/shared/api/bindings.ts` — 무변경(빈 diff) 확인.
- 변경 파일: 12개 테마 JSON(`list.hoverBackground`/`list.activeBackground` 2줄씩만),
  `mapping-tables.ts`(`list.*` COLOR_MAPPING 3엔트리), `service.rs`(정규화 헬퍼 + 린트 테스트
  + 예외 화이트리스트). 그 외 표면 무변경.
