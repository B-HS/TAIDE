# M8 Rust 브라우저 통신 기반

## 결과와 범위

후속 shared renderer/native-host 분리와 실제 BrowserShell의 DTO/snapshot consumer는 `2026-10-05-m8-rust-remote-shell-state.md`가 정본입니다. 아래 통신 실측은 해당 소비자 구현 이전의 불변 transport 근거이며 제품 화면/새 소비자 실측을 대신하지 않습니다.

기존 서버와 Rust 브라우저가 같은 RemoteRequest·JSON/binary frame·상수를 쓰도록 `crates/taide-remote-wire`로 분리했습니다. `taide-model::remote::RemoteRequest`와 `taide-remote::protocol`의 기존 경로는 re-export로 유지했습니다. 플랫폼 중립 Client와 실제 web-sys WebSocket adapter를 구현했고, 순수 코어4건·변경된 실제 서버2건·격리 Chrome/Wasm 연속 검사1건이 각1회 통과했습니다. 이 결과는 원격 화면·최종 제품 bundle·M8 완료가 아닙니다.

대상은 `crates/taide-remote-wire/src/{protocol.rs,client.rs,tests.rs,client-tests.rs}`, root/model/remote의 manifest와 re-export, native `remote-ws.rs`의 공통4001 상수, `native/taide-remote-web`, 그 아래 test-only `tests/browser-probe`, `tools/m8-remote-rust-browser-probe.ts`입니다. main이 process/verify/save-docs 스킬로 직접 수행했고 subagent/workflow/Git 작업은 없었습니다.

## 보존한 계약

- [x] 원본 TypeScript `remote-ws-client.ts`와 shim의 seq1·null args·FIFO·disconnect pending/queue 폐기·끊긴 mutation 재전송 금지·1초 reconnect·recovery 통지·4001 로그인 이동을 대조했습니다. client seq는 u32 범위 끝에서 명시 거절하고 재사용하지 않습니다.
- [x] 기존 서버 frame builder 본문을 같은 순수 모듈로 이동했습니다. response header5/channel header9·big-endian u32·binary byte 내용, chan/chanEnd/index, JSON 문자열 event payload를 유지했습니다. malformed/unknown/truncated frame은 pending을 소비하지 않으며, server failure payload는 원본 Value로 반환합니다.
- [x] 연결별 opaque epoch로 이전 open/close/message를 무시하고, browser error callback도 현재 연결만 닫습니다. 콜백은 Weak owner를 사용하며 Closure.forget나 전역 singleton을 사용하지 않습니다. Socket Drop은 listener 제거→closure 해제→close이고 timer Drop은 clearTimeout입니다. 재연결 timer는 wake 통지 전에 등록해 통지 도중 dispose하면 새 timer도 취소되게 했습니다.
- [x] browser는 현재 HTTP/HTTPS origin의 `/__taide/ws`만 WS/WSS로 바꿉니다. 실패한 send/연결의 미완료 seq는 rejected 이벤트로 반환하고, timer 등록 실패는 명시적으로 수렴합니다. 내부 borrow를 해제한 뒤 wake를 호출합니다. 실제 UI의 repaint/response/channel/event consumer 연결은 다음 경계입니다.

## 실행 근거

공통 Cargo target은 `/private/tmp/taide-m8-menu-build.j6Efnw`, CARGO_HOME은 `/Users/hyunseokbyun/development/rust/cargo`입니다. 실제 server localhost 검사와 headless Chrome에는 이전에 확인된 sandbox bind EPERM을 피하기 위한 좁은 실행 권한을 사용했습니다. 사용자 데이터·인증·Keychain·기존 앱은 사용하지 않았습니다.

- [x] `cargo test --manifest-path Cargo.toml -p taide-remote-wire --lib --offline --target-dir <target>`:4 PASS, build3.93초/suite0.00초입니다. wire/잘린 frame·재연결/no mutation replay·응답 단일 완료/채널/event·만료/dispose/seq exhaustion의 서로 다른 검사입니다.
- [x] native app의 `cargo test --lib remote_ws::tests:: -- --exact`로 `실제_ws_인증과_json_raw_channel_event_왕복_후_정리된다`와 `실제_7일_시계_경과는_http_401과_ws_4001을_보낸다`만 실행했습니다.2 PASS, build27.17초/suite0.03초입니다. 선행 성공 이후 공유 wire/상수 변경의 실제 caller 영향 검사이며 다른 성공은 재실행하지 않았습니다.
- [x] Rust std `wasm32-unknown-unknown` target을 현재 stable1.98.1에 추가했습니다. 공식 wasm-bindgen0.2.129 CLI는 `/private/tmp/taide-m8-wasm-tools.h2xBQB`에만 내려받았습니다. 공식 release API digest와 다운로드 SHA256 `81d4a23d56b3c3eb8187658329116d50e0b228a93b343825fb71f70179051cd1`이 일치했습니다. 전역 CLI 설치와 제품 bundle 교체는 하지 않았습니다.
- [x] test-only browser-probe의 실제 Wasm `cargo build --target wasm32-unknown-unknown --lib`:exit0/5.68초입니다. CLI `--target web --no-typescript --out-name probe` 출력은 위 임시 디렉터리 `built` 아래입니다. 이것은 실제 BrowserClient를 실행하기 위한 검사 자산이지 제품 remote-public이 아닙니다.
- [x] `bun tools/m8-remote-rust-browser-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/built`:첫 실행 PASS입니다. 같은 격리 Chrome/context 안에서 초기503 handshake 실패→seq1 쓰기 폐기→새 조회→binary→JSON/binary/channelEnd/event→1012 재연결→새 조회→dispose→4001 재인증 이동까지 이어서 실행했습니다.2회 recovery·원래 mutation 서버 호출0·순서0/1/2·wake12·마지막 active socket0·upgrade4·JS pageerror0입니다. dispose 및 만료 후에는 각각1.1초 지나도 추가 upgrade가 없었습니다. TTL 자체7일 정책은 위 실제 native server 검사이며 이 browser fixture는4001 처리만 검증합니다.

원본 결과는 `built/result.json`입니다. browser result의 requests는 `settings_get(seq2)`, `file_read_raw(3)`, `subscribe(4)`, `cut_connection(5)`, `settings_get(6)`, recreate한 client의 `expire(1)`뿐입니다. 성공 뒤 RECOVERY_COUNT와 CHANNEL_END_INDEX의 이름을 분리한 것은 값2/제어 의미가 동일해 browser 성공을 재사용하고 TS 검사만 수행합니다.

## 정적 검사와 변경 경계

- [x] wire lib/tests strict clippy exit0/.94초, 실제 native app lib/bins/tests strict exit0/11.89초입니다.
- [x] browser 실제 Wasm-target strict clippy 최초exit0/9.53초, callback epoch/해제 순서 점검 변경 뒤 관련 최종exit0/.07초입니다. browser-probe Wasm strict는exit0/.20초입니다. 같은 상태의 검사를 반복하지 않았습니다.
- [x] 검사용 TS의 strict `bunx --no-install tsc --noEmit --strict --skipLibCheck --target es2022 --module esnext --moduleResolution bundler --types bun tools/m8-remote-rust-browser-probe.ts`는exit0입니다. authored Rust13 exactfmt·대상 TS/HTML Prettier·tracked diff whitespace를 확인했습니다.
- [x] `cargo tree --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target wasm32-unknown-unknown -e normal`에서 browser→wire/serde/web-sys/js-sys/wasm-bindgen만 확인했습니다. native PTY/runtime/infra/model/OS 의존성을 반입하지 않았습니다. root/native lock은 새 로컬 wire package/edge만 추가하며 기존 버전 변경은 없고 browser/probe는 별도 lock을 생성했습니다. browser의 Web API direct edge는 이미 사용 중인0.2.129/0.3.106을 재사용합니다.

native test link의 기존 `__eh_frame`16MiB 경고와 Wry dependency17 경고는 남으며 검사 억제는 하지 않았습니다. 실제 GUI/제품 크기·CPU/GPU/RSS 성능에 관한 통과 주장이나 시간 추정은 없습니다.

최종 TS 이름 분리 뒤 strict/Prettier는exit0이고 tracked diff check도exit0입니다. 이번 미추적/관련18파일의 no-index whitespace check는 출력 없이exit1(새 diff 존재)이며 whitespace 오류가 아닙니다. browser 실행은 반복하지 않았습니다.

## 남은 작업

- [ ] 기존 renderer의 native host/runtime 의존성 분리와 실제 BrowserShell의 프로젝트·설정·레이아웃 snapshot 공급은 후속 QA에서 구현/정적 검사했습니다. 실제 browser App·모든 원본 화면/상태/consumer는 남습니다. 기존 TerminalCore를 중복 파싱하는 브라우저 parser를 추가하지 않았으며 전체 native/remote query handoff는 미완료입니다.
- [ ] 실제 Rust 제품 UI build·generated binding·strict bundle manifest·패키징·기존 Catalog 준비/actual App 호출을 연결합니다. 테스트 fixture/빈 module/legacy TS dist fallback을 제품으로 넣지 않습니다.
- [ ] 실제 서비스 인증과 이 Rust browser consumer의 통합, failed-close 재연결·enabled IDE/hooks·전체 GUI/성능/beta/cutover/Rust99%·패키지/설치/rollback 검증을 마칩니다. 후속 QA에서 model/editor/UI와 BrowserShell의 정상 Wasm 그래프에 native 실행/FS/PTY/runtime 의존성이0임을 확인했으나 전체 제품 renderer/consumer는 미완료입니다.

브라우저 pending/outbox는 원본처럼 무상한이며 adapter의 이벤트 보관도 전체 RSS 상한으로 검증하지 않았습니다. server의256프레임 정책이나 자산 loader의64MiB 상한을 client 메모리 제한으로 주장하지 않습니다. 숨김/느린 화면에서 실제 consumer의 처리 수명·pressure 정책은 그 consumer를 연결할 때 검증해야 합니다. 별도 byte 정책을 이번 단계에 임의로 추가하지 않았습니다.

후속47은2/4(50%), 전체363/433(83.83%, 비가중·공수비 아님), 최종 N1~N8은0/8입니다. SDK target 준비·순수 통신 기반의 진척을 제품 UI 게이트 완료로 올리지 않았습니다. 전체 ETA는 남은 UI/실기/배포 범위 공수 미확정으로 산정 보류, goal은active입니다. 보호 egui spike bundle·기존 Cargo cache·OS/입력기/VoiceOver/Keychain/사용자 데이터·제품 TS·vendor·기존 MSRV를 보존하며 전체 M8 완료 전 commit/push는 하지 않습니다.

참조한 primary 문서는 Serde enum representations, Rust VecDeque/checked_add, rustup Cross-compilation, wasm-bindgen의 WebSockets/CLI와 공식0.2.129 release, Playwright Browser 및 Bun WebSockets입니다. 새로운 API는 공식 문서와 설치된 web-sys/wasm-bindgen generated source를 확인한 뒤 사용했습니다.
