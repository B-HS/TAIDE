# ADR-0005 — 터미널: xterm.js(view) + portable-pty(Rust)

- 상태: 승인 (2026-08-06)
- 관련: `docs/PRD.md` FR-G, `docs/features/terminal.md`, `docs/research/xterm-pty.md`

## 맥락

사용자 기본 셸을 그대로 쓰는 내장 터미널이 필요하다(FR-G1). VSCode·많은 웹 기반 터미널이 검증한
표준 조합이 존재하며, Tauri(비 Node) 환경이라 node-pty 는 사용 불가.

## 결정

- 렌더링: **@xterm/xterm** (+ fit, webgl, search, web-links, serialize 등 addon)
- pty: **portable-pty crate** (Rust, wezterm 프로젝트 산하) — macOS/Linux pty, Windows ConPTY 를 단일 API 로 흡수
- 소유권: pty 프로세스·스크롤백 원본은 Rust 소유(ADR-0004). view 의 xterm 은 표시 전용이며
  reload 시 Rust 버퍼에서 재생한다.
- 셸 선택: OS 기본/로그인 셸 자동 감지 + 설정에서 변경 가능(Windows 는 설치된 셸 열거).

## 기각한 대안

- 자체 터미널 렌더러(warp 식 GPU 렌더): 구현 비용이 IDE 본체를 넘어선다.
- WebSocket 로컬 서버 + node-pty: Node 런타임을 다시 들이는 것으로 ADR-0001 취지에 반한다.

## 결과

- Rust↔view 사이 고속 출력 스트림의 전송 방식(이벤트 vs Channel)과 flow control 이 성능 핵심이 된다
  → `docs/research/xterm-pty.md` 확정안을 `features/terminal.md` 에 반영.
- OSC 133 셸 통합(명령 블록 인식) 등 warp 류 기능은 셸 rc 주입이 필요한 선택 기능으로 분리한다.

## 구현 노트 (2026-08-28 현행화)

- addon 실설치: fit·search·web-links·webgl·**unicode11** (`@xterm/addon-serialize` 는 미도입 —
  링버퍼 재생(`pty_attach`)이 직렬화 요구를 대체).
