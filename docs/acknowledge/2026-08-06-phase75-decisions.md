# Phase 7.5 결정·합의 (2026-08-06)

> Phase 0~7 구현 완료 후 사용자가 실기기에서 확인하며 제기한 18건. Phase 8(배포) 전에 처리한다.
> 각 항목의 상세 스펙은 `docs/features/*.md`, 순서는 `docs/roadmap.md` Phase 7.5.

## 1. 사용자 확인을 받은 결정 3건

### 1.1 탭 열기 규칙 — VSCode 규칙 그대로 (7번)

사용자가 "새창의 기준이 모호하다"고 지적. 아래로 확정한다.

| 조작 | 동작 |
|------|------|
| 단일 클릭 | **preview 탭**(제목 이탤릭). Leaf 당 1개, 다음 단일 클릭이 같은 자리를 재사용 |
| 더블 클릭 | 고정 탭으로 승격 |
| 편집 시작 | 고정 탭으로 승격 |
| `⌘`(Ctrl) + 클릭 | **옆 pane 으로 분할**해서 열기 |
| tab context menu → Move into New Window | 별도 **OS 창** |

- "새 OS 창"은 **context menu 를 통해서만** 열린다. 파일 클릭으로는 절대 새 OS 창이 열리지 않는다.
- preview/pin 자체는 이미 `tabs.md` §3 에 구현돼 있다. **`⌘+클릭 → 분할**과 **Move into New Window**가 추가분.

### 1.2 미리보기 범위 — 오피스 전량 + HWP (3번)

이미지 / PDF / 비디오 / HTML **+ xlsx + pptx + HWP** 까지 지원한다.

- HWP 는 사용자가 지정한 **`rhwp`**(https://github.com/edwardkim/rhwp) 를 쓴다.
  실사 확인: Rust + WebAssembly, **MIT**, npm `@rhwp/core` **0.8.2** / `@rhwp/editor` 0.8.2 배포 중,
  HWP 5.0 바이너리 + HWPX 파싱, **SVG/PNG/PDF 렌더 API** 제공 → 데스크톱 임베드 가능.
- 상세 라이브러리 선정·한계는 `docs/features/preview.md`.

### 1.3 footer 확대/축소 범위 — 에디터·터미널 폰트만 (15번)

앱 전체 UI 배율이 아니라 **codeview·터미널의 폰트 크기만** 조절한다.
(사용자 원문의 "다만" 이하가 끊겨 확인한 결과)

## 2. 나머지 15건의 처리 방침

| # | 항목 | 방침 |
|---|------|------|
| 1 | 폰트 커스텀 (시스템 폰트 선택) | 설정에 폰트 선택 UI. 시스템 폰트 열거는 Rust 커맨드 신설 |
| 2 | codeview 테마 미반영 | **버그**. 테마 토큰 → Monaco `defineTheme` 파생이 실제로 적용되는지 확인·수정 |
| 4 | 기동 시 흰 화면 깜빡임 | **버그**. FOUC 방지가 헤더에만 적용됨 — `theme-system.md` §4.1 재점검 |
| 5 | remote-control (잠자기/잠금 중 동작) | **기술 검토 선행** → `docs/research/remote-control.md` |
| 6 | 폴더 아이콘 | material-icon-theme 참조 |
| 8 | 탭 context menu 가 native UI | **view component 로 교체**(전 OS 일관) |
| 9 | 탭 context menu 항목 확장 | 사용자가 제시한 전체 목록 채택 — `tabs.md` §3 갱신 |
| 10 | `⌘P` / `>` 접두 커맨드 | **확장 가능한 커맨드 레지스트리**로 설계 |
| 11 | toast 위치 9분할 설정 | 설정에 위치 선택 |
| 12 | resizer 두께 | 설정에서 조절 |
| 13 | 설정창 레이아웃 | TOC 좌측 탭 + `max-w` 제거 + card + **자체 컴포넌트**(native input 금지) |
| 14 | 테마 커스텀 에디터 + 저장 | 별도 페이지 |
| 16 | xterm 재평가 | **기술 검토 선행** → `docs/research/terminal-reevaluation.md` |
| 17 | 헤더 다크/라이트 미반영 | **버그**. 타이틀바 색이 테마를 따르지 않음 |
| 18 | 헤더 중앙 정보 표시 | 활성 탭 이름 + 프로젝트명 + git 브랜치 |

## 3. 확정한 원칙 (이번 라운드에서 반복 등장)

### 3.1 네이티브 UI 금지 — 전 OS 일관성

8번·13번이 같은 요구다. **OS 기본 컨텍스트 메뉴·`<input>`·`<select>` 등 네이티브 위젯을
그대로 쓰지 않는다.** 전부 `shared/ui` 의 자체 컴포넌트로 만들어 macOS/Windows/Linux 에서
동일하게 보이게 한다. (Tauri 메뉴 accelerator 는 키 선점 목적으로만 유지 — `editor.md` §6)

### 3.2 확장성 — 커맨드 레지스트리

10번의 "나중에 뭐든 잘 붙을 수 있게"를 위해, 커맨드 팔레트는 **하드코딩 목록이 아니라
레지스트리에 등록하는 구조**로 만든다. 기존 `shared/lib/keymap.ts` 선언과 합류시킨다.

### 3.3 버그 4건은 원인을 먼저 확인한다

2·4·17번은 "안 된다"는 증상 보고다. **추측으로 고치지 말고** 실제 렌더 값을 확인한 뒤
근본 원인을 잡는다. 특히 2·17 은 같은 뿌리(테마 토큰이 특정 소비자에게 전달되지 않음)일 가능성이 있다.
