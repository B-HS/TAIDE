# 앱 아이콘 확정 — Prompt Spark (2026-08-28)

## 결정

- 시안 4종(Caret T / Prompt Spark / Bracket Monogram / Gradient Field, macOS 1024 캔버스·824
  스쿼클 그리드·taide 기본 테마 액센트 팔레트) 중 **B "Prompt Spark" 를 사용자 확정 채택**.
- 구성: 터미널 프롬프트 셰브론(블루 그라디언트 `#a6c8ff→#6f92f5`) + 블록 커서(`#b4befe`) +
  AI 스파크(`#cba6f7`), 다크 스쿼클 배경(`#1d2030→#101321`) + 내부 하이라이트 보더.
- 미채택 시안 3종은 아카이브만(스크래치패드 소멸 가능) — 재론 시 재제작.

## 정본과 파이프라인

- **정본 = `src-tauri/icons/icon.svg`** (1024 viewBox·스쿼클 마진 포함). 아이콘 수정은 반드시
  이 SVG 를 고친 뒤 아래 파이프라인으로 전 세트를 재생성한다. 개별 PNG 수기 편집 금지.
- 재생성 절차:
  1. SVG → 1024 PNG(알파 보존): 프로젝트 동봉 Playwright(webkit)로 렌더 — `setContent` 후
     `screenshot({ omitBackground: true })`, viewport 1024·deviceScaleFactor 1.
  2. `bun run tauri icon <1024.png>` — `src-tauri/icons/` 전 세트(icns/ico/png/Square*/
     android/ios, 50파일) 재생성. `tauri.conf.json` 의 icon 목록은 파일명 불변이라 무수정.
- 2026-08-28 적용 실측: 1024 렌더 `hasAlpha: yes`·icns "Mac OS X icon"·ico 6사이즈 RGBA·
  128px 판독성 확인. 코드 무접촉(에셋 전용)·bun 스위트 그린.

## dev 도크 아이콘 미반영 — 근본 원인·수정 (2026-08-28)

- 증상: 아이콘 세트 교체 후 `bun run tauri dev` 재시작에도 도크 아이콘이 구버전 유지.
- 근본 원인(소스 실물 확인): macOS dev 의 도크 아이콘은 `tauri::generate_context!` 가
  **컴파일 타임**에 `icons/icon.icns`(bundle icon 목록의 `.icns` 우선)를 읽어 임베드하고
  `RunEvent::Ready` 에서 `setApplicationIconImage` 로 적용한다(tauri 2.11.5 `app.rs:2578`,
  tauri-codegen 2.6.3 `context.rs` `new_raw`). 프록 매크로의 `fs::read` 는 cargo 가 추적하지
  못하고, tauri-build 2.6.3 의 `rerun-if-changed` 목록(config·resources·binaries)에 아이콘이
  없다 — 아이콘만 변경 시 재컴파일이 일어나지 않아 구 임베드가 잔존한다.
- 수정: `src-tauri/build.rs` 에 `cargo:rerun-if-changed=icons` 1줄 추가(디렉토리 지정 =
  하위 파일 재귀 추적). 이후 아이콘 변경 → build script 재실행 → 크레이트 재컴파일 →
  새 아이콘 임베드가 자동 성립. `cargo fmt`·`cargo check`(taide 재컴파일 확인) 그린.
