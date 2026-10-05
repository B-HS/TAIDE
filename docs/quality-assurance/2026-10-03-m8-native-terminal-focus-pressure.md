# M8 native terminal 포화 중 focus 보고

## 대상·재현

`native/taide-native-app/src/{terminal_surface,terminal_host,terminal_writer}.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다. 앞선 focus 수명·query 뒤 일반 Outbox 포화 시 보고가 버려지는 경계를 수정했습니다.

실제 writer count=1에서 최초 FocusIn과 문자 63개로 일반 Outbox 64개를 채우고, UI를 숨김→복원→숨김→복원했습니다. 이후의 Out/In/Out/In 보고가 실제 child에 도달하지 않아 신규 `focus_pressure`가 실패했습니다. 실패 검사에서도 child close/join·TaskSupervisor 회수 후 assertion을 수행했습니다.

## 구현·상한

후속 공통 순서 배선에서 단일 focus slot은 중간 query를 보존하는 최대 64 fragment로 변경됐습니다. 현재 상한·기본 검사·미완료 경계는 `2026-10-03-m8-native-terminal-input-order-queue.md`가 정본이며 아래 단일 slot 설명은 이 checkpoint의 이력입니다.

- [x] 일반 receipt+pending 64개·기존 byte 예산은 유지합니다. 일반 큐가 가득 찼을 때만 focus 전용 pending slot 하나를 추가 사용합니다. slot의 retained payload는 `min(64KiB, writer 전체 byte 예산 - Envelope 크기)` 이하이며 PendingInput inline 크기가 추가됩니다. 기존 64개 전체만으로 메모리를 설명하지 않습니다.
- [x] 이후 focus 보고는 해당 slot에 순서대로 바이트를 붙입니다. Out/In을 최종 상태 하나로 축약하지 않습니다. 서로 다른 session token 또는 사용자 입력 payload를 합치는 것은 거절합니다. 성장 시 새 allocation의 capacity를 검사하고, 거절 시 기존 batch를 보존합니다.
- [x] focus slot이 남아 있으면 새 일반 입력의 admission을 닫습니다. 이미 대기 중인 일반 입력을 먼저 drain하고 focus batch를 동일 Session의 writer에 제출합니다. 포화가 계속되면 원래 encoding을 그대로 유지하여 다음 background tick에 재시도합니다. focus 보고는 user input epoch를 올리지 않습니다.
- [x] shared Outbox가 slot을 소유하므로 View 숨김·viewport 제거 뒤에도 살아 있는 Session의 보고는 남습니다. 기존 clear/cancel·session 제거 경로는 slot도 정리하도록 연결했습니다. 이 clear 경로에 대한 별도 actual PTY cancel 검사는 이번에 실행하지 않았습니다.
- [x] 전체 byte 예산보다 큰 단일 batch가 뒤늦게 writer에서 거절되지 않도록 writer의 payload 상한을 함께 사용합니다. focus slot 자체의 byte 상한 초과는 명시적 오류이며 무한 입력 보존을 보장하지 않습니다. 이 경우 기존 batch는 유지되지만 새 보고가 거절되는 정책과 추가 UX/복구는 전체 입력 gate로 남습니다.

## 실제 검증

Cargo는 직렬 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. app manifest는 `native/taide-native-app/Cargo.toml`입니다.

| 검사 | 실제 결과 |
| --- | --- |
| app `--test terminal-host focus_pressure`, 최초 | exit 101, compile 3.43초 / suite 6.36초. `focus reports were lost at the input budget` |
| 같은 검사, 전용 slot·순서 연결 후 | 1 PASS, compile 8.58초 / suite 1.61초. child literal 검증·최초 query/FocusIn·63개 x·Out/In/Out/In·새 z/Out 순서를 확인했습니다. 포화 중 시도한 `not-admitted` 문자는 수신되지 않았고, epoch+64·exit 0·close/join·task 0입니다. |
| app `--lib control_batch` | 신규 1 PASS, compile 3.12초 / suite 0.00초. 순서 보존·정확한 retained capacity·상한/다른 session/사용자 입력 혼합 거절·거절 후 원본 보존을 확인했습니다. |

기존 lifecycle/query/startup/hidden mouse·일반 writer 성공은 같은 위험의 반복 검사 없이 재사용했습니다. 새 unit 전에 성장 배수 2를 이름 있는 상수로 옮겼으며 값·동작은 바꾸지 않았습니다. 테스트 fixture는 자기 child PTY에만 noncanonical 모드를 설정합니다.

app `clippy --lib --test terminal-host -- -D warnings` exit 0(1.42초), native `clippy --bin native-session-fixture -- -D warnings` exit 0(0.23초), authored 5파일 exact rustfmt·추적 diff check exit 0입니다. 기존 Wry 17 warnings와 authored strict 결과를 구분합니다.

## 남은 경계

- [ ] output query Dispatcher와 UI Outbox는 아직 서로 다른 admission 경로입니다. 후속 `2026-10-03-m8-native-terminal-query-pressure.md`에서 일시적 writer 포화의 query 실패를 대기로 바꿨지만, 먼저 대기한 UI 입력과 새 query의 전체 순서·공정성은 여전히 다음 구현/검증 대상입니다.
- [ ] 작은 writer byte 상한을 쓰는 실제 PTY, focus slot 자체 상한 초과 뒤 UX/복구, 같은 session의 빠른 다중 창 전환·mode 변경/retire/cancel 조합은 아직 전체 검증이 아닙니다. 기본 실제 포화 검사와 token unit 성공을 이 조합의 성공으로 확장하지 않습니다.
- [ ] 일반 captured-wheel 숨김·동일 event 일부 소비의 모호한 순서·UI 주입·mode ABA/forced selection·전체 VT/IME/AX·aggregate/RSS·GUI와 213 TS view/full cutover/TS 제거가 남습니다. retained 상한은 allocator의 작업 중 일시적 복사 peak나 앱 전체 RSS 상한이 아닙니다.

보호 실기 bundle·기존 TS/root/MSRV·사용자 데이터/OS 설정은 유지했습니다. M8 N1~N8 전체는 0/8이며 전체 완료 후 commit·push합니다.
