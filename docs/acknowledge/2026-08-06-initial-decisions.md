# 2026-08-06 — TAIDE 초기 결정·합의

> 세션 첫 질문 묶음에서 사용자가 확정한 사항. 이후 모든 문서·구현은 이 결정을 따른다.

## 사용자 확정 결정

| 항목 | 결정 | 비고 |
|------|------|------|
| 코드 에디터 코어 | **Monaco Editor** | VSCode 단축키·diff split·LSP 요구사항 충족 비용이 가장 낮음 |
| UI 컴포넌트 | **shadcn/ui 기본 + Radix Primitives fallback** | shadcn 에 없는 컴포넌트는 Radix 로 직접 제작. **shadcn 컴포넌트 전체 목록을 완벽하게 확보해 누락 없이 활용** (`docs/research/tailwind-shadcn.md`) |
| 세션 범위 | **문서화만 완결** | PRD·ADR·아키텍처·기능 스펙·체크리스트 완성 후 멈춤. 구현은 다음 세션 |
| git | **git init + 수동 커밋** | 커밋·푸시는 사용자가 요청할 때만. auto-commit 미설정 |

## 사용자 지시 (원 프롬프트에서)

- 기술스택: **Tauri + TypeScript(view)**. Electron 배제(무게·용량).
- 뷰 참조: warp, VSCode, Cursor IDE. Tailwind **v4 최신(4.3 확인 필수)**. 디자인 컴포넌트는 shadcn 어레인지 또는 radix/base-ui/react-aria 직접 제작 → 위 결정으로 확정.
- **누수 없는 환경**: 데이터는 Rust 단이 소유, view 는 표시 전용. view reload 로도 상태가 유지되는 구조.
- **프로젝트 추상화 필수**: 추후 프로젝트별 remote-control(자체 서버 + 웹 패널) 등 기능 부착 예정.
- 문서화 수준: 구현 시 추가 탐색이 "context7 등에서 간단한 사용법 확인" 수준으로 끝나도록 **모든 기술·체크리스트·상세 구현사항을 docs/ 에 완벽하게** 정리. PRD·ADR 포함. 정합성·타당성·위반사항 검수까지.
- **Workflow 사용 시 opus + medium**. general-purpose 등 Fable 상속 에이전트 사용 금지(사용량 문제).
- 궁금함이 생기면 임의 진행하지 말고 사용자에게 먼저 질문(쉽게 설명).

## 진행자(AI) 가정 — 사용자 이의 없음

- 제품명: **TAIDE** (폴더명 그대로)
- 뷰: **React 19 + React Compiler + FSD** (컨벤션 기본)
- 플랫폼: **macOS 우선 개발, Windows/Linux 지원 가능한 설계**
- 터미널: **xterm.js + Rust portable-pty**
