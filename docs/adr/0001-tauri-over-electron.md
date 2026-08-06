# ADR-0001 — 셸 프레임워크: Tauri 2 (Electron 배제)

- 상태: 승인 (사용자 확정, 2026-08-06)
- 관련: `docs/PRD.md` NFR-1, `docs/research/tauri-v2.md`

## 맥락

커스텀 IDE 의 데스크톱 셸이 필요하다. 후보는 Electron, Tauri.
사용자 요구: 가볍고 용량이 작을 것, 데이터는 Rust 단이 소유하고 view 는 표시만 하는 누수 없는 구조.

## 결정

**Tauri 2 + TypeScript(view)** 를 사용한다.

## 근거

- Electron 은 Chromium + Node 동봉으로 설치 용량(100MB+)과 상주 메모리가 크다.
  Tauri 는 OS WebView(macOS WKWebView / Windows WebView2 / Linux WebKitGTK)를 사용해 수 MB 급 배포가 가능하다.
- 코어 로직을 Rust 로 두는 아키텍처(ADR-0004)와 자연스럽게 결합된다 — pty, git(libgit2), LSP 프로세스,
  파일 와처 등 시스템 자원은 전부 Rust 가 소유한다.
- Tauri 2 의 capabilities 권한 모델로 렌더러 노출 표면을 최소화할 수 있다(NFR-7, 데스크톱 보안 컨벤션).

## 결과 (트레이드오프)

- OS WebView 간 렌더링·기능 편차가 존재한다 → macOS 우선 개발, 플랫폼 분기는 Rust 에 격리(NFR-6).
- Node 생태계 네이티브 모듈(node-pty 등)을 못 쓴다 → Rust 대체(portable-pty, git2)로 해결(ADR-0005, 0006).
- Electron 대비 레퍼런스가 적다 → `docs/research/` 로 사전 조사를 마치고 진행.
