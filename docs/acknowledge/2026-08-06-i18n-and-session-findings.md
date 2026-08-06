# 다국어(i18n) 방침 · 세션 운영 결정 (2026-08-06)

> Phase 7.5 구현 세션에서 사용자가 추가로 지시·확정한 사항. 기존 결정은
> `2026-08-06-phase75-decisions.md`, 순서는 `docs/roadmap.md` Phase 7.5.

## 1. 다국어 지원 — react-i18next + Rust 소유 (사용자 확정)

**요구**: en / ko / ja 지원 + "언어팩도 넣으면 바로 사용할 수 있도록".

### 1.1 라이브러리 — react-i18next

| 후보 | 판정 |
|------|------|
| **react-i18next (i18next)** | **채택.** 런타임에 리소스를 추가할 수 있는 유일한 후보(`addResourceBundle`). 언어팩 요구를 만족한다 |
| Paraglide JS / LinguiJS | 기각. **컴파일 타임에 메시지가 고정**돼 사용자 언어팩을 런타임에 추가할 수 없다 — 요구와 정면 충돌 |
| 자체 구현 | 기각. 복수형·보간 규칙을 직접 지게 된다 |

**참조 구현**: 사용자의 `/Users/gkn/raw-viewer` 가 이미 같은 스택이다
(`i18next` ^26.3.6 / `react-i18next` ^17.0.9, `src/i18n/{i18n,en,ko,ja}.ts`,
`AppLanguage = 'system' | 'ko' | 'en' | 'ja'` + `resolveLanguage` 로 system 추종).
**키 네임스페이스 구조와 `system` 추종 방식은 이 구현을 따른다.**

### 1.2 소유권 — Rust (ADR-0004)

raw-viewer 와 다른 점은 **TAIDE 는 언어 선택을 view 가 소유하지 않는다**는 것이다.

- 현재 언어는 `settings` 도메인의 필드(`language`)로 Rust 가 소유·영속화한다.
- 사용자 언어팩은 **`{app_data}/locales/*.json`** 에 두고 Rust 가 열거·로드해 IPC 로 넘긴다.
  **테마(`themes/*.json`)와 완전히 같은 구조**를 쓴다 — 열거/로드/watcher 핫리로드.
- 내장 en/ko/ja 는 앱 리소스로 포함하고, 사용자 팩은 내장을 base 로 **부분 오버라이드**한다
  (누락 키는 base 에서 채움 — 테마 `extends` 와 동일 정책).
- 번역 문자열을 localStorage 나 view 상태로 선주입하지 않는다.

### 1.3 순서

**7.5-G(터미널) 다음.** 사용자 확정. 잔상·입력 지연이 체감상 더 급하다.

## 2. 세션 운영 — 앱 실행은 사용자가 한다 (실패 후 확정)

에이전트 셸에서 `bun run tauri dev` 를 띄웠다가 **두 번 오진**했다.

1. **샌드박스 셸에서 띄운 인스턴스**는 웹뷰가 IPC 부트스트랩을 못 받아 파일트리가 비고
   테마가 적용되지 않았다. 이것을 코드 회귀로 오인해 원인을 한참 잘못 짚었다.
2. **macOS TCC 권한은 부모 프로세스에 귀속**된다. 에이전트 셸에서 띄우면 Desktop/Documents 등
   보호 폴더 접근이 `Operation not permitted (os error 1)` 로 막힌다.
   사용자가 본 "업데이트 전 프로젝트가 에러난다"가 이것이다.

**결정: 앱 실행·시각 확인은 사용자가 직접 한다.** 에이전트는 코드 수정과 기계 검증
(`bun run verify`)까지만 하고, 실행 결과 판정은 사용자에게 넘긴다.

부수 주의: 진단 목적으로 Chrome 을 `localhost:5173` 에 붙이면 **vite 클라이언트 로그가 앱 로그에 섞인다.**
Tauri 런타임이 없어 `__TAURI_INTERNALS__` undefined 에러가 대량 발생하는데 이것을 앱 콘솔로 오인하기 쉽다.
진단 후 탭을 반드시 닫는다.

## 3. 부수 발견 — 별건으로 기록

| 항목 | 판정 |
|------|------|
| **새 터미널을 여는 경로 부재** | 기본 레이아웃의 Terminal 탭 1개뿐, 닫으면 복구 불가. `toggle-terminal` 은 keymap 선언만 있고 핸들러 없음 → keymap `new-terminal`(`⌃⇧\``) 추가로 **이번 세션에서 해소** |
| **`⌘W` 가 앱을 종료** | IDE 라면 탭 닫기여야 한다. keymap 에 `close-tab`(`⌘W`)이 있으나 WebView 기본 동작이 선점하는 것으로 보인다. **7.5-C 에서 처리** |
| **타이틀바 중앙 정보 미표시** | 버그가 아니라 **미구현**. 타이틀바 위젯 자체가 없다(`grep tauri-drag-region` 0건). 7.5-D(18번) |
| **`applyMonacoTheme` 의 엄격 hex 검증** | 6자리가 아니면 예외를 던진다. 내장 테마는 안전하나 사용자 테마(7.5-D §7.3) 도입 시 앱이 죽을 수 있다. 방어 위치는 `theme-system.md` §6 이 정한 **Rust 로더 검증** |
