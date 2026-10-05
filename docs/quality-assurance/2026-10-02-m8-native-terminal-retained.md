# M8 native terminal의 실제 retained graph

## 대상·상태

`native/taide-native-terminal/{Cargo.toml,Cargo.lock,src/lib.rs,src/stream.rs,tests/retained.rs,vendor/alacritty-terminal}`, `native/taide-native-retained/src/lib.rs`, `experiments/terminal-core-spike/{Cargo.toml,Cargo.lock,vendor/vte}`입니다. 메인이 workflow·서브에이전트 없이 직접 구현했습니다. N4-A/N4·M8 전체는 미완료이며 기존 TS 화면·제품 PTY/scanner·native app 의존성·root/MSRV·보호 실기 bundle과 OS 설정은 유지했습니다.

## 실제 소유 경계

- 설치 Alacritty 0.26.0 source를 Apache-2.0 license/metadata와 함께 격리 fork에 보존했습니다. 선택적 기본 비활성 `native-retained` feature가 private Term/grid/row/storage/cursor/template·selection/vi·damage/tabs·Config/title/title stack/keyboard stack, CellExtra의 zerowidth Vec·Hyperlink의 Arc/String에 field별 derive 계약을 연결합니다. public 활성 행만 복제해 계산하지 않습니다. Storage의 활성 len 밖 resize/cache 행과 모든 Vec/String capacity를 포함하며 공유 Arc는 identity별 한 번만 계산합니다.
- 같은 기본 비활성 vte feature가 실제 ProcessorState/SyncState/Parser/Params의 모든 field를 연결합니다. 처음부터 확보하는 2MiB sync buffer, 미완성 OSC raw/payload Vec, timeout과 observer Box/Arc도 포함됩니다. 알 수 없는 observer는 materialize하거나 비용 0으로 통과시키지 않고 `Opaque`로 거절합니다. 기존 helper에 Range·Instant 계약을 추가했습니다.
- Core/Pending/Listener/Observer/Outcome은 실제 소유 field의 typed 계약을 사용합니다. ScanEvent는 원본 variant를 exhaustive match하며 문자열 capacity와 command marker의 scalar field를 방문합니다. 공유 normalizer의 derive 계약을 양 소비자가 사용하도록 spike에도 기존 local helper를 직접 연결했습니다. source 정책을 다시 복사하지 않았고 새 외부 package는 없습니다.
- 원본 색상 formatter closure가 prefix·terminator **문자열을 소유함**을 source에서 확인했습니다. 이 fork의 native feature에서는 `NativeColorRequest(ColorQuery)`·`NativeTextAreaSizeRequest(TextAreaSizeQuery)`로 전달하고 같은 색상 reply/terminator와 pixel reply를 유지합니다. pixel 곱은 u32로 계산해 큰 u16 크기에서 overflow하지 않습니다. 일반 feature의 기존 callback API는 유지하며 unknown callback/ChildExit는 retained adapter에서 `Opaque`입니다. GUI query consumer 연결은 다음 단계입니다.
- 기존 grid base-cell 산술 admission과 별도로 caller 선택형 retained byte/visit quota를 추가했습니다. 기본 논리 byte quota는 128MiB, visit quota는 1,000,000입니다. 생성 뒤·feed/sync 처리 뒤·resize 뒤 실제 Core를 검사하고, 반환 Outcome과 남은 Core의 합산 비용도 handoff 전에 검사합니다. query 문자열은 기존 effect byte quota에도 포함합니다. 계산 실패/초과는 명시적 오류·Term 폐기·pending 해제를 수행하고 부분 Outcome을 성공으로 반환하지 않습니다. 반환 뒤 caller가 계속 보유하는 과거 Outcome은 caller 책임이며 aggregate session quota는 아직 없습니다.

이 비용은 논리 retained payload입니다. post-operation 검사이므로 할당 전 peak/RSS·allocator overhead·visitor 임시 작업 공간·OS/PTy/GPU cap이 아닙니다. 전체 grid를 순회하는 비용과 방문 한도의 실제 대형 history 정책은 성능 gate에서 검증해야 합니다. retire 후 parser의 기본 sync buffer는 남으며 모든 allocation 0이라는 주장은 하지 않습니다.

## 원문 로그·upstream stack 오류

fork의 title/title stack·hyperlink·색상 prefix 로그와 Event Debug의 title/PTY/clipboard 텍스트를 redaction했습니다. vte의 native feature에서는 DCS put/hook·잘못된 cursor shape의 raw payload를 기록하지 않으며 feature가 꺼진 기존 vte 로그 동작은 유지합니다. 전체 OS/product 로거 감사가 아니라 실제 Core가 호출하는 해당 경계의 검사입니다.

Alacritty 원본 `push_keyboard_mode`는 keyboard stack 포화 시 `title_stack.remove(0)`을 실행했습니다. title stack이 비면 panic, 비지 않으면 무관한 title 이력을 잃습니다. source 확인 후 최소 검사에서 원본 한 줄로 RED를 재현하고 `keyboard_mode_stack.remove(0)`으로 수정했습니다. 외부 registry package·기존 제품 parser는 변경하지 않았습니다. 원본과 의도적으로 다른 이 수정과 log 정책은 fork의 UPSTREAM 문서에 명시합니다.

## 단일 실행 증거

Cargo는 직렬이며 `--offline --target-dir experiments/native-shell-spike/target`입니다. 최초 dependency edge 해석만 unlocked, 이후 core 검사는 `--locked`입니다.

1. [x] 새 Alacritty path/retained graph `cargo check … --lib`: exit 0(1.96초). registry Alacritty 대신 같은 버전의 path record와 기존 local retained/derive 2 record를 해석했습니다. parser field 계약 추가 뒤 해당 compile exit 0(0.99초)입니다.
2. [x] `cargo test … --test retained 실제_graph -- --nocapture`: 1 PASS, compile 0.97초/suite 0.00초. private resize cache/inactive grid·config capacity·title stack·CellExtra/Hyperlink dedup·raw OSC/sync capacity·unknown observer/callback Opaque·실제 Core의 Arc 1회 계산·typed 색상/pixel reply·생성/방문/증가/resize/query 상한·retire/후속 입력 거절을 한 연속 검사로 확인했습니다. 최초 fixture의 generic Processor 타입 추론 E0283은 `Processor::<StdSyncHandler>` 명시로 수정했습니다. 실행되지 않은 compile 실패는 PASS로 세지 않습니다.
3. [x] `cargo test … --test retained keyboard_stack`: 원본 삭제 한 줄에서 1 FAIL(exit 101, suite 0.00초, empty title stack panic). 정확한 stack 수정 뒤 1 PASS(compile 0.92초/suite 0.00초). 4,097회 push·4,096개 bounded capacity와 후속 pop/push를 확인했습니다. 같은 상태의 성공을 재실행하지 않았습니다.
4. [x] `cargo test … --test retained native_trace`: 1 PASS(compile 0.45초/suite 0.00초). Trace logger를 켠 실제 Core의 synthetic title/push/pop·hyperlink·cursor shape·DCS에서 원문 marker가 로그에 나타나지 않습니다. Event Debug의 제목/PTY marker redaction도 graph 검사에 포함합니다. OS 앱·사용자 파일/클립보드는 조작하지 않았습니다.
5. [x] 변경된 actual admission의 영향 검사 `cargo test … --test core --test outcome`: core 2 PASS(suite 0.01초), outcome 1 PASS(suite 0.33초), compile 0.51초. 새 quota가 기존 Unicode/SGR/title stack·sync text/effect overflow 정책을 깨지 않는지 관련 기존 검사만 한 번 실행했습니다. `cargo clippy … --lib --tests -- -D warnings` exit 0(1.03초), shared source 소비자 spike `cargo check --manifest-path experiments/terminal-core-spike/Cargo.toml … --lib --tests` exit 0(2.35초), authored exact Rust fmt와 `git diff --check` exit 0입니다. spike는 원래 registry Alacritty와 feature-off vte를 그대로 사용합니다.

Core 수동 ScanEvent adapter 첫 compile에서 잘못된 CommandMarker module E0433, module만 교정한 뒤 원본과 다른 variant 이름 E0599가 각각 실패했습니다. 실제 `shell_integration::CommandMarker::{OutputStart,Finished { exit_code }}` 전문을 읽고 수정한 뒤 compile exit 0(0.44초)입니다. 추정 이름을 성공 근거로 기록하지 않습니다.

## 남은 게이트

- [ ] PTY attach/snapshot/live·bounded writer/query·agent metadata consumer·input/IME·selection/search/link·native terminal surface와 session/window 수명.
- [ ] 실제 aggregate admission·큰 history의 visitor/CPU·할당 전 peak·RSS/allocator/GPU·전체 terminal 프로토콜/원본 상태·보안 및 GUI 동등성.
- [ ] upstream ref corpus·실제 OS shell/GUI/codec·비macOS·M8 N1~N8·TS 제거/beta/rollback/배포. 현재 fork는 46MiB corpus/ref target을 포함하지 않으며 corpus PASS를 주장하지 않습니다.

다음 구현은 기존 PTY attach/input/terminal placeholder 연결입니다. 전체 M8 완료 전 commit·push하지 않습니다.
