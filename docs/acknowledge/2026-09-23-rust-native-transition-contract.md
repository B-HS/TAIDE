# Rust-native 전환 계약

> 결정일: 2026-09-23
> 브랜치: `to_rust_native`
> 기준 커밋: `deb5867` (`dev`)
> 실행 정본: `docs/roadmap-rust-native.md`
> 검증 정본: `docs/quality-assurance/2026-09-23-rust-native-parity-plan.md`

## 1. 사용자 목표

TAIDE의 TypeScript·React·Tauri·Monaco·xterm 의존을 장기적으로 제거하고 현재 기능 전체를 Rust-native 애플리케이션으로 제공한다.

전환의 직접 목적은 다음 세 가지다.

1. Monaco와 xterm의 제약에서 벗어나 편집기와 터미널을 제품 요구에 맞게 직접 최적화한다.
2. Rust와 TypeScript로 나뉜 LSP 소유권을 하나의 Rust 상태기계로 통합해 장시간 `connecting` 상태, 재연결, 문서 replay 문제를 근본적으로 제거한다.
3. Tauri IPC, WebView, React Query 캐시와 Rust 상태 사이의 복제·직렬화·동기화 비용을 제거하고 입력·렌더링·메모리 성능을 개선한다.

## 2. 확정 결정

| 항목 | 결정 |
| --- | --- |
| 제품 기능 | 현재 사용자 기능을 축소하지 않는다. 기존 한계를 개선할 수는 있지만 기능을 제거해 동등성을 맞추지 않는다. |
| 전환 방식 | Big-bang 재작성 금지. 기존 앱과 Rust-native 실행 경로를 병행하는 단계적 전환만 허용한다. |
| 기존 Rust 코어 | 파일·프로젝트·레이아웃·Git·PTY·LSP process·검색·원격·AI·동기화 로직과 테스트를 우선 재사용한다. |
| TypeScript 제거 시점 | 모든 기능·성능·보안·패키징 게이트와 beta 안정화가 끝난 마지막 단계에서만 제거한다. |
| Monaco·xterm 제거 시점 | native editor와 terminal이 각자의 동등성·성능·실기 게이트를 통과한 뒤 한 릴리스의 fallback 기간을 거쳐 제거한다. |
| LSP 소유권 | process, JSON-RPC transport, generation, document mirror, 요청 취소, crash replay와 UI 표시 상태를 Rust `LspCoordinator` 한 곳이 소유한다. |
| 플랫폼 우선순위 | 현재 배포 대상과 같은 macOS Apple Silicon을 첫 완료 대상으로 한다. 기존 Windows·Linux 조건부 코드는 별도 결정 없이 삭제하지 않는다. |
| 의존성 | 이 계약은 GUI·editor·terminal crate를 확정하지 않는다. 공식 자료 검토와 동일 spike를 통과한 뒤 별도 승인한다. |
| Git | 이 계획은 `to_rust_native`에서 관리한다. force push와 AI 트레일러를 금지한다. |

사용자가 앞서 요청한 일반적인 commit·push 금지는 이번 요청의 명시적 `commit push` 지시로 이 계획 문서에 한해 해제됐다. 이후 구현 작업에서는 매 요청의 Git 지시를 다시 따른다.

## 3. 절대 불변식

### 3.1 데이터와 복구

- 기존 `settings.json`, `session.json`, 프로젝트 메타데이터, 레이아웃, hot-exit buffer, 테마·언어팩·snippet·plugin·LSP 경로를 읽을 수 있어야 한다.
- native 전용 schema를 쓰기 전까지 기존 Tauri 앱으로 되돌아갈 수 있어야 한다.
- schema migration은 versioned·forward-only이며 미래 버전을 조용히 downgrade하지 않는다.
- 저장 중 타이핑, 외부 파일 변경, 앱 crash, window close, project close에서 dirty buffer를 잃지 않는다.

### 3.2 수명주기와 보안

- project capability attach/detach 순서, `FlushScope`, watcher·PTY·LSP task 종료, remote session 회수를 보존한다.
- project root guard, remote 기본 거부 dispatch, Host·Origin·인증 검증, keyring namespace, 외부 URL 검증을 완화하지 않는다.
- HTML·SVG·PDF·HWP·XLSX 등 비신뢰 입력은 renderer/helper crash와 권한 경계를 앱 코어에서 격리한다.

### 3.3 편집기와 LSP

- 하나의 문서는 여러 split view에서 공유하되 selection·scroll·fold·IME composition은 view별로 분리한다.
- 모든 변경은 revision을 가진 transaction으로 적용하고 undo, dirty, save, hot-exit, LSP mirror가 같은 transaction을 관찰한다.
- crash/restart 후 initialize와 열린 문서 replay가 완료되기 전에는 `Running`을 표시하지 않는다.
- generation·document revision이 다른 비동기 결과는 UI에 적용하지 않는다.

### 3.4 터미널

- PTY 출력은 하나의 Rust terminal core가 단 한 번 파싱한다.
- terminal screen model과 OSC 7·8·9·133·777 효과가 같은 parser 결과에서 파생돼야 한다.
- attach replay와 live output 사이에 중복·누락이 없어야 하고 backpressure·scrollback은 상한을 유지한다.
- 한글·일본어·중국어 IME composition은 화면에 표시하되 확정 문자열만 PTY에 한 번 전송한다.

## 4. 완료 정의

다음 조건을 모두 만족할 때만 Rust-native 전환을 완료로 본다.

1. `docs/quality-assurance/2026-09-23-rust-native-parity-plan.md`의 필수 항목이 자동 또는 실기 증거로 완료된다.
2. 같은 기기·fixture에서 native 앱의 핵심 성능 지표가 현행 release baseline보다 악화되지 않는다.
3. 기존 앱 데이터로 native 앱을 실행하고 재시작·복구한 뒤 기능과 데이터 손실이 없다.
4. remote, IDE integration, CLI `taide --wait`, Git·PTY·LSP 장기 session이 동등하게 동작한다.
5. 이전 안정 Tauri 릴리스로 되돌리는 복구 절차가 검증된 beta 기간을 통과한다.
6. TS·React·Tauri·Monaco·xterm 참조와 빌드 자산이 0건이며 Rust-native 배포물이 서명·공증된다.

## 5. 이번 계획 작업의 범위

- 장기 목표, 목표 crate 경계, 단계와 의존성, spike, 동등성·성능·보안 게이트를 문서화한다.
- 제품 코드, Cargo dependency, 빌드 설정, 기존 기능 동작은 변경하지 않는다.
- 실제 framework·crate 선정과 구현은 각 phase 착수 시 공식 문서, prototype, 사용자 결정을 거친다.
