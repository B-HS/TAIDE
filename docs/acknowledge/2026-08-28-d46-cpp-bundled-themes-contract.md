# d-46 — VS Code C/C++ 테마(light/dark) 번들 편입 계약 (2026-08-28)

> 사용자 지시: "기본테마에 현재 설치된 VS Code 폴더에서 C/C++ theme 가져와서 완벽하게
> light/dark 로 넣어라".

## 0. 소스 실사 (메인)

- 설치 확장: `~/.vscode/extensions/ms-vscode.cpptools-themes-2.0.0` — 테마 4종 중 신형 페어
  채택: `cpptools_dark_vs_new.json`(name "Visual Studio 2019 Dark (C++)", uiTheme vs-dark) ·
  `cpptools_light_vs_new.json`(light). 구형(2017) 페어는 범위 외.
- 라이선스: **MIT**(Copyright Microsoft Corporation — 확장 동봉 LICENSE.txt 실물 확인).
  출처: https://github.com/microsoft/vscode-cpptools
- 파일은 JSONC(주석 포함) — 변환기 `parseJsonc` 가 처리. `include` 체인 없음(전수 grep).

## 1. 범위·절차 (프로젝트 정본 도구·게이트 준수)

- 변환: `scripts/convert-vscode-theme.ts`(정본 변환기) 로 2종 변환 —
  id `visual-studio-cpp-dark`/`visual-studio-cpp-light`, name 은 업스트림 충실
  ("Visual Studio 2019 Dark (C++)"/"Visual Studio 2019 Light (C++)"), author "Microsoft",
  license "MIT", source-url 위 저장소. 산출물은 `src-tauri/resources/themes/`.
- 등록: `domain/theme/service.rs` `BUNDLED_THEME_SOURCES` 2항 추가(개수 단언은 len() 파생이라
  자동 추종 — 하드코딩 수 잔존 여부 전수 grep).
- 게이트: TS `bundled-theme-contrast.test.ts`(디렉토리 동적 열거 — 자동 포섭)·Rust 테마 린트
  전량(38종·대비 게이트) 통과가 완료 조건. **게이트 실패 시 d-36/d-40 정본 절차**: 업스트림
  값 보존을 우선하되 수리가 불가피하면 최소 수리 후 `docs/theme-system.md` §8.2.3/§8.2.4
  재변환 비재현 표에 기록(재론 금지 항목 준수 — 어드바이저리 blocking 승격 금지·번들 재변환
  일치 강제 금지).
- 문서: theme-system.md 의 번들 목록 서술(있으면)·개수 언급 갱신.

## 2. 실행·검토

- 구현 fixer(sonnet+xhigh, Rust 단일 점유) → 게이트 결과에 따라 수리 기록 → 검토 1렌즈
  (opus+xhigh — 변환 충실성·게이트·수리 기록) → 메인 2차(verify) → 사용자 실기(테마 선택
  UI 에서 2종 확인).

## 3. 기록 (구현·검증 — wf_061e7305 fixer)

- 변환 2건 exit 0 · **수리 0건**(게이트 위반 없음 — 업스트림 값 그대로 통과, d-36/d-40 수리
  절차 미발동). 경고는 앱 전용 확장 토큰의 safe-default 폴백 21~22종과 terminal.ansi 부재 시
  VS Code 공식 팔레트 폴백 — 기존 번들 다수와 동일 패턴(어드바이저리, 차단 아님).
- 등록: `BUNDLED_THEME_SOURCES` 말미 dark→light 페어 관례로 2항(번들 36→38, 카탈로그 40).
  구조적 총계 서술만 갱신(service.rs 테스트 doc 5곳) — 과거 감사 실측 수치(15/36·ΔE 5.4+ 등,
  코드 4곳·theme-system.md 5곳)는 재감사 없이 유지(임의 치환 금지 원칙). theme-system.md 의
  번들 총계 서술(§ 카탈로그)은 메인이 38로 갱신+연혁 주기.
- 검증: typecheck·bun **1520/0**(TS 대비 게이트 신규 2종 자동 포섭)·cargo workspace
  **1130/0**(테마 린트 전량 — 대비 3종·불투명·구별성 포함)·clippy·fmt·bindings 무변경.
- 잔여: 사용자 실기 — 설정 테마 목록에서 "Visual Studio 2019 Dark/Light (C++)" 2종 확인.

## 4. 검토·수정 라운드 (통합 렌즈 wf_3174823a → wf_6af753e2)

- **major 1 — B-1 `THIRD_PARTY_LICENSES.md` 등재 누락**(MIT 표시 의무·theme-system §8.1 정책,
  grep 기계 확정 — 적대적 생략): 메인이 등재 추가 + 재발 방지로 리소스 디렉토리↔라이선스 파일
  패리티 bun 테스트 신설(`bundled-theme-licenses.test.ts` — 계약 §1 절차 갭이 원인이므로
  구조적 봉쇄).
- minor: B-2 §8.1 표 2행 추가(메인) / B-3 ANSI 폴백 열거 8→10종 정정(메인 — 구조적 사실이라
  "과거 감사 수치 유지" 예외 아님) / **B-4 이름 결정: 확장 라벨 페어 "Dark/Light (Visual
  Studio - C/C++)" 채택 재변환** — light 원본 name("Light (Visual Studio)")과 다르나 두 라벨
  모두 업스트림 package.json 실존 문자열이고 페어 대칭(이탈 아님·근거 기록). name 외 필드
  바이트 무변경을 diff 로 확인.
- verified_ok 요지: 재변환 산출물이 커밋본과 deep-equal(수동 편집 0 확증)·표본 색 대조 전건
  일치(dark 13스코프·light 11스코프)·keyword←keyword.control 매핑은 기존 관례 동일·예외 목록
  무등재·등록 형식/총계 정합.
- 검증: bun **1521/0**·cargo 테마 스위트 45/0(이름 변경 무영향)·typecheck/eslint/prettier 그린.
- 잔여: 사용자 실기 — **앱 재시작 필요**(Rust `include_str` 번들이라 HMR 불가) 후 테마 목록에서
  "Dark/Light (Visual Studio - C/C++)" 확인.
- **실기 확인(2026-08-28, 사용자): 앱 재시작 후 테마 목록에 2종 정상 표시 — d-46 종결.**
  커밋 `f3d92ce`(feat — 테마 2종·등록·라이선스 등재·패리티 테스트).
