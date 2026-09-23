# Rust-native Phase 0 계약 기준선

> 결정일: 2026-09-23
> 브랜치: `to_rust_native`
> 상위 계약: `docs/acknowledge/2026-09-23-rust-native-transition-contract.md`
> 실행 정본: `docs/roadmap-rust-native.md`

## 1. 조사 결론

Rust-native 실행 경로나 crate 분리를 먼저 시작하지 않습니다. 현행 Tauri 앱과 native 앱이 같은 의미를 유지했는지 판정할 기계 판독 가능한 기준선이 아직 없고, Phase 0의 여섯 항목도 모두 미완료이기 때문입니다.

첫 구현 배치는 제품 동작을 바꾸지 않고 다음 원천을 정적 manifest와 drift test로 고정합니다.

- `src-tauri/src/lib.rs`: Specta command, event, raw channel 등록 집합
- `src-tauri/src/events.rs`: event wire name과 payload 직렬화 계약
- `src-tauri/src/error.rs`: IPC error code 집합
- `src-tauri/src/domain/remote/dispatch.rs`: remote JSON·raw command 구현 및 기본 거부 정책
- `src/shared/api/bindings.ts`: 생성된 command 입력·출력 DTO와 event payload의 현행 wire 계약

## 2. 구현 계약

1. manifest는 schema version과 원천 경로를 포함하고 command, event, raw channel, error code, remote policy를 결정적 순서로 기록합니다.
2. drift test는 원천 코드의 실제 집합과 manifest를 양방향 비교해 새 항목 누락과 폐기 항목 잔존을 모두 실패시킵니다.
3. command signature와 payload 전체는 생성 `bindings.ts`의 digest로 고정해 이름 집합만 맞고 wire 의미가 달라지는 변경도 검출합니다.
4. remote command는 허용·거부·raw 전송 분류가 전체 IPC 집합을 빠짐없이 나누는 기존 기본 거부 계약을 보존합니다.
5. 기존 Tauri command/event signature, 제품 로직, dependency와 workspace member는 변경하지 않습니다.

## 3. 후속 기준선

이번 배치 뒤 다음 Phase 0 작업을 순서대로 진행합니다.

1. remote HTTP·WebSocket 인증, session revoke, binary channel golden fixture
2. IDE/MCP request·response와 CLI `--wait` marker lifecycle fixture
3. settings, session, project, layout, hot-exit의 versioned persistence fixture
4. editor, LSP, terminal, preview, shell 기능 inventory와 기존 시험 근거 연결
5. 동일 macOS Apple Silicon 기기에서 release build를 3회 측정한 성능 중앙값·p95·p99

마지막 성능 항목은 앱 실행이 필요하므로 자동 코드 검사와 구분합니다. 실제 값이 없을 때 임의 수치로 채우거나 완료 처리하지 않습니다.

## 4. 보안 fixture 우선순위

조사에서 다음 경계를 Phase 0 후속 fixture의 우선 대상으로 확정했습니다.

- Host·Origin·cookie·link token·password-only login·session TTL·revoke
- WebSocket 요청·응답·binary channel의 queue와 resource budget
- project root·symlink·CLI 허용 경로와 일반 IPC 경로 분리
- IDE token과 MCP method·tool call의 성공·거부 응답
- CLI marker의 생성·삭제·timeout·소유권

현재 코드에서 확인된 WebSocket 무제한 대기·응답 queue, 전역 password-only 잠금, 빠른 비밀번호 hash, 취약한 spreadsheet parser, CLI marker 소유권 공백은 이번 동작 비변경 배치에서 수정하지 않습니다. 각각 실제 재현과 별도 보안 수정 계약이 필요한 후속 위험으로 유지합니다.

## 5. 구현 기록

첫 자동화 배치로 IPC 계약 manifest와 drift test를 추가했습니다. 제품 동작, command/event signature, dependency, workspace member는 변경하지 않았습니다.

### 산출물

| 파일 | 역할 |
| --- | --- |
| `src-tauri/tests/fixtures/rust-native/ipc-contract-manifest.json` | schema version 1 계약 기준선 |
| `src-tauri/tests/rust_native_phase0_contract.rs` | 원천 코드와 manifest 양방향 drift 검출 |

### manifest 실측값

| 항목 | 값 |
| --- | --- |
| `schemaVersion` | 1 |
| `ordering` | `source-declaration-order` |
| `spectaCommands` | 203 |
| `rawChannelCommands` | 3 (`pty_spawn`, `pty_attach`, `file_read_raw`) |
| `events` | 30 (Rust type + `event_name` wire 쌍) |
| `errorWireCodes` | 6 (`Io`, `NotFound`, `InvalidArgument`, `Forbidden`, `Internal`, `Localized`) |
| `remotePolicy.policies` | 11 |
| `remotePolicy.implementedJsonCommands` | 203 |
| `remotePolicy.allowedCommands` | 177 |
| `remotePolicy.deniedCommands` | 29 |
| `remotePolicy.rawDispatchCommands` | 1 (`file_read_raw`) |
| `remotePolicy.unclassifiedPolicy` | `Unclassified` |
| `generatedBindings` | `src/shared/api/bindings.ts`, `sha256`, 64자리 16진수 digest |

### 검출 계약

- command/raw/error/remote 목록은 원천 선언 순서를 그대로 기록하고 집합과 순서를 모두 비교합니다. event manifest는 `events.rs` 선언 순서를 고정하고 `collect_events!`·생성 bindings에는 등록 집합의 양방향 일치만 요구합니다.
- `bindings.ts` 전체 바이트 SHA-256이 command 입력·출력 DTO와 event payload 변경을 검출합니다.
- remote 분류는 전체 IPC 커맨드(`spectaCommands` ∪ `rawChannelCommands`)를 허용·거부로 중복 없이 나누고, 미분류·유령 분류·raw 겹침을 각각 실패시킵니다.
- 주석 줄을 제거한 뒤 스캔하므로 doc comment 안의 예시 심볼은 집계되지 않습니다.

### 검증

- `cargo test -p taide --test rust_native_phase0_contract`: 7 passed, exit 0.
- `cargo fmt --all --check`: exit 0.
- fixture 변형 10종(누락·중복·raw 겹침·미분류·해시 드리프트·순서 반전·error code 추가·event wire 변경·유령 분류·schema version 상승)에서 해당 테스트가 각각 실패했고, 원복 후 재통과했습니다.

### 남은 한계

- 스캔은 원천 텍스트 기반이므로 macro 우회나 깊은 재수출로 심볼을 숨기면 잡지 못합니다. 기존 `lib.rs`·`dispatch.rs` source-scan 테스트와 같은 한계입니다.
- line comment는 제거하지만 block comment는 제거하지 않으므로 원천 선언 형식을 바꿀 때 scanner도 함께 검토해야 합니다.
- IDE/MCP, CLI `--wait` marker lifecycle, versioned persistence fixture와 실기 성능 측정은 이번 배치 범위가 아니며 §3 후속 순서를 따릅니다.
