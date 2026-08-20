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

(작성 예정)
