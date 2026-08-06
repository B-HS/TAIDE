# 기술 스택 (버전 확정 — 2026-08-06 스냅샷)

> **버전의 정본.** 각 값은 `docs/research/*.md` 에서 1차 출처(공식 레지스트리·릴리즈)로 확인된 것.
> 변화가 빠른 패키지는 캐럿 없이 **정확한 버전으로 고정**하고 업그레이드는 명시적 작업으로만 한다.
> 전 항목 확정(2026-08-06 사용자 결정 반영 — `docs/acknowledge/2026-08-06-final-tech-decisions.md`).

## 런타임 · 도구

| 항목 | 선택 | 비고 |
|------|------|------|
| 패키지 매니저/스크립트 | **Bun** (컨벤션 기본) | Vite dev/build 는 Vite 자체 사용. bun 으로 `tauri dev` 실행 |
| 프론트 빌드 | Vite **8.2.0** (Rolldown 기반) | `build.target`: win `chrome105` / 그 외 `safari13` (Tauri 템플릿 — 필수) |
| Rust | stable 최신 + `src-tauri` | release 프로파일: lto, codegen-units 1, strip (`research/performance-memory.md` §8) |
| 테스트 | bun:test + Testing Library(프론트), `cargo test`(Rust) | 컨벤션 frontend.md §11 |

## 프론트엔드 (npm)

| 패키지 | 버전 | 용도 |
|--------|------|------|
| react / react-dom | 19.2.8 | UI |
| @vitejs/plugin-react | 6.0.5 | oxc 기반 — **`react({ babel })` 옵션 사용 불가** |
| @rolldown/plugin-babel + babel-plugin-react-compiler | 0.2.3 / 1.0.0 | React Compiler (`reactCompilerPreset()` — research/react-frontend-stack.md §1) |
| eslint-plugin-react-hooks | 7.1.1 | Compiler 규칙 lint 게이트 |
| @tanstack/react-query | 5.101.4 | Rust 데이터 소비 (ADR-0008 — `networkMode: 'always'` 필수) |
| @tauri-apps/api | 2.11.1 | IPC |
| tailwindcss / @tailwindcss/vite | 4.3.3 | 스타일 (v4.3 실재 확인) |
| shadcn CLI | 4.16.1 | UI 킷 (`shared/ui` 로 vendored — ADR-0003). 전체 63개 컴포넌트 목록: research/tailwind-shadcn.md §6 |
| radix-ui (통합 패키지) | 1.6.7 | shadcn 기반 + fallback primitive (구 `@radix-ui/react-*` 와 혼용 금지) |
| tailwind-merge / clsx / CVA | 3.6.0 / 2.1.1 / 0.7.1 | cn 유틸 (v4 는 tailwind-merge 3.x 필수) |
| tw-animate-css | 1.4.0 | 애니메이션 (tailwindcss-animate 는 deprecated) |
| lucide-react | 1.28.0 | 아이콘 |
| monaco-editor | 0.56.0 | 에디터 (**경로 체계 변경 — `esm/vs` 접두 금지**, research/monaco.md) |
| @xterm/xterm + addons | 6.0.0 (addons 동반 릴리즈) | 터미널 (canvas addon 은 v6 미대응) |
| react-resizable-panels | 4.12.2 | 스플릿 (**v4 API — `Group`/`Separator`**, 구 `PanelGroup` 예제 금지) |
| @dnd-kit/core + sortable + utilities | 6.3.1 / 10.0.0 / 3.2.2 (확정 — 레거시 라인. 신 라인 @dnd-kit/react 는 pre-1.0) | 탭 DND |
| @tanstack/react-virtual | 3.14.9 (확정 — Rust 트리 + 가상스크롤) | 파일 트리 가상화 |
| zustand | 도입 보류 (ADR-0008) | 실수요 발생 시 5.0.14 |

## Rust (crates)

| crate | 버전 | 용도 |
|-------|------|------|
| tauri | "2" (현행 2.11.5) | 코어 |
| tauri-plugin-dialog | "2" | 폴더 선택 |
| tauri-plugin-log | "2" | 로깅 |
| tauri-plugin-window-state | "2" | 윈도우 기하 복원 (ADR-0009) |
| tauri-plugin-single-instance | "2" | CLI/중복 실행 (agent-integration) |
| tauri-plugin-opener | "2" | Finder 열기·외부 링크 |
| portable-pty | 0.9 | 터미널 pty (ADR-0005) |
| git2 | 0.21 (`features = ["ssh","https","vendored-libgit2"]`, Linux 배포 + `vendored-openssl`) | Git 읽기·stage (ADR-0006 — **feature 미지정 시 push 인증 런타임 실패**) |
| notify + notify-debouncer-full | 8.2 / 0.7 (stable 짝) | 파일 와처 (9.x/0.8.x 는 rc — 미사용) |
| tokio | 1.x (`fs,io-util,process,sync`) | async 런타임 |
| serde / serde_json / thiserror | 1 / 1 / 2 | 직렬화·에러 |
| parking_lot | 0.12 | 락 |
| sysinfo | 0.39 | Windows 프로세스 트리(에이전트 감지) |
| uuid | 1.x | ProjectId·마커 파일 |
| tauri-specta + specta + specta-typescript | **=2.0.0-rc.25 / =2.0.0-rc.25 / 0.0.12** (확정 — ADR-0011) | Rust→TS 타입 생성. **`cargo add` 기본은 v1(1.0.2)이 설치되므로 버전 명시 필수** |

미사용 결정: tauri-plugin-fs·shell·store (view 직접 접근 불필요 — 커스텀 command 경유, ipc-contract §4·ADR-0009), deep-link(2차 — EDITOR 경로에는 부적합), updater(배포 단계에서 도입).

## 외부 LSP 서버 (앱 번들 아님 — 로컬 감지, ADR-0007)

| 언어 | 서버 | 버전 기준 |
|------|------|----------|
| JS/TS | vtsls | 0.3.0 (TS7 `tsc --lsp` 는 옵션 백엔드) |
| Rust | rust-analyzer | 주간 릴리즈 추종 |
| Python | basedpyright + ruff server | 1.39.9 / 0.16.1 |
| Markdown | marksman | 2026-02-08 릴리즈 (유지보수 정체 인지) |

## 정정 기록 (research 문서 간 불일치)

- `research/react-frontend-stack.md` 의 버전 표에 있는 "tauri-specta 1.0.2" 는 **Tauri v1 용 stable** 을
  가리킨 것 — v2 대응은 rc.25 가 맞다(`research/tauri-v2.md` §7 이 정본).
- `research/react-frontend-stack.md`·`xterm-pty.md` 의 zustand/persist·localStorage 제안은 일반 패턴
  설명으로, TAIDE 에서는 ADR-0004/0008 이 우선한다(레이아웃·설정은 Rust 소유).
