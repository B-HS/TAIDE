# d-47 — 설치본(GUI 실행) LSP 서버 감지 전멸 수정 계약 (2026-08-28)

> 발견: v0.1.0 설치 실기(사용자). dev(터미널 실행)는 정상, Finder 실행 설치본만 전멸.

## 0. 근본 원인 (메인 확정 — 코드 실물)

- 감지·실행 해석은 `domain/lsp/service.rs::find_in_path` 가 **프로세스 `PATH`**
  (`std::env::var_os("PATH")`)만 스캔한다. macOS 에서 Finder/Dock 실행 GUI 앱은 launchd 의
  최소 PATH(`/usr/bin:/bin:/usr/sbin:/sbin`)만 상속 → `~/.cargo/bin`·bun/npm 글로벌 등
  사용자 도구 경로 전부 불가시. 터미널 실행(dev)만 풀 PATH 라 증상이 prod 전용으로 나타남.
- 파급: LSP 감지뿐 아니라 PATH 의존 전반(태스크 러너 도구 탐지·에이전트 CLI 존재 확인 등)이
  동일 축이다 — 지점별 땜질이 아니라 **프로세스 PATH 자체를 부팅 시 교정**하는 것이 근본.

## 1. 수정 방향 (사용자 확정: 크레이트 채택)

- `fix-path-env`(tauri-apps 공식, Apache-2.0, **crates.io 미배포 — git 의존**)를
  `rev = "c4c45d503ea115a839aae718d02f79e7c7f0f673"` 로 **핀 고정** 추가.
- `lib.rs::run()` 최초입(빌더 생성 전)에서 `fix_path_env::fix()` 호출 — 로그인 셸을 1회
  실행해 사용자 PATH 를 프로세스 env 로 병합. 실패는 치명 아님: `log::warn` 후 계속 부팅.
- 범위 외: 감지 로직 자체 변경(그대로 프로세스 PATH 소비 — 교정만으로 해소), Windows(크레이트가
  no-op 처리하는지 fixer 가 소스 실사 후 필요 시 cfg 가드).
- 검증: cargo fmt/clippy/test + **최종 오라클 = 사용자 실기**(로컬 tauri build 산출 .app 재설치
  후 LSP 감지 목록 확인).

## 2. 실행·검토

- 구현 fixer(sonnet+xhigh, Rust 단일 점유 — d-48 Rust측과 동일 에이전트) → 통합 1렌즈
  (opus+xhigh — 축소 근거: 원인 메인 실증·소형 표면·사용자 실기 후속) → 메인 2차.

## 3. 기록 (구현·검토·검증)

- **구현**(wf_7e4547c6 Rust fixer): rev 핀 의존 추가·`run()` 최초입 호출. 크레이트 소스 실사
  — Windows 도 no-op 이 아니라 별도 목적(Command PATH 상속 버그)의 코드가 있어 계약 범위대로
  `#[cfg(not(windows))]` 가드(방어적 선택 — Windows 지원 재개 시 가드 제거 검토, 렌즈 F9).
- **렌즈**(wf_ad9ea564, opus+xhigh): major 1 — F1 `run()` 초입은 로거 미설치 시점이라
  실패 warn 이 침묵 유실(tauri-plugin-log attach 는 setup 훅) → 오류를 보관했다가 `.setup()`
  에서 기록하도록 수정(wf_bf67d2be). info 2 기록: F9(가드 사유 상기)·F10(fix() 는 로그인 셸
  동기 실행 — 실기에서 기동 지연 체감 시 비인터랙티브 셸/캐시 검토).
- 검증: cargo clippy 0·workspace 1130/0(반영 후 재실행은 메인 2차에서). 최종 오라클 =
  사용자 실기(로컬 build .app 재설치 후 LSP 감지 확인) 대기.
