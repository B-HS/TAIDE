# M8 native TerminalCore의 단일 parser Outcome

## 대상·상태

`native/taide-native-terminal/src/{lib,stream}.rs`, 신규 `tests/outcome.rs`, `experiments/terminal-core-spike/src/lib.rs`입니다. 새 `Outcome`이 effects·safe text·이전 chunk의 128B overlap을 같이 소유합니다. 실제 Core의 같은 Processor/StreamObserver에서 print/execute/CSI와 OSC를 소비하며 raw bytes의 두 번째 scanner를 추가하지 않았습니다. PTY/metadata/agent/native GUI consumer와 실제 retained graph/RSS·N4/M8는 미완료입니다.

메인이 workflow·서브에이전트 없이 수행했습니다. 기존 product scanner/PTY·native app graph·제품/root dependency/MSRV·보호 사용자 bundle·앱/설정은 변경하지 않았습니다. 실제 OS shell/PTY·clipboard·GUI를 실행하지 않았습니다.

## 공유 source와 계약

기존 spike의 `normalized-stream.rs`를 native `src/stream.rs`로 옮겼습니다. 새 파일의 실제 내용이 이동 전 전문과 정확히 같음을 확인했습니다. 원본 파일을 중복 보관하지 않고 spike의 path module을 이 source로 연결해 2곳에서 같은 정책을 사용합니다. 기존 spike의 실행 동작을 임의로 바꾸지 않습니다. 원래 경로의 파일은 공유 위치로 이동됐으며 새 파일에서 전체 내용을 복구할 수 있습니다.

- Core Pending이 NormalizedStream을 함께 소유합니다. 관찰 callback은 ANSI handler와 같은 parser에서 실행되며 OSC/DCS/SOS/APC 등의 숨은 payload를 text로 노출하지 않습니다. callback별 mutex 비용은 아직 실제 PTY throughput으로 측정하지 않았습니다.
- `advance_outcome`/`flush_sync_outcome`은 text/overlap/effects를 함께 반환합니다. 기존 effects-only `advance`/`flush_sync`도 같은 경계를 호출해 호환하며 반환하지 않은 text는 소비된 것으로 처리합니다. 실제 text consumer는 Outcome API를 사용해야 합니다.
- UTF8 조각은 parser의 상태로 복원하고 overlap은 UTF8 문자 경계에서 128B 이하입니다. synchronized bytes의 text/OSC는 동일 flush에서 나옵니다. 기존 prototype의 chunk별 whitespace 정규화 정책을 유지하며 모든 입력/chunk의 정규화 문자열이 항상 whole feed와 동일하다는 새 주장은 하지 않습니다.
- 정규화 text는 기존 65,536B 상한입니다. 동기화 flush에서 text가 초과하면 pending effects·text와 Term을 retire하고 명시적 오류를 반환합니다. 잘린 문자열이나 부분 effects를 성공으로 전달하지 않습니다. feed 64KiB와 normalized/sync 상한은 서로 다른 경계입니다.
- effect overflow에서도 공유 stream을 초기화해 그곳의 pending text를 보관하지 않습니다. 실제 PTY consumer의 pause/error/close policy·원본 private 그래프 전체 회수/CPU peak는 아직 연결하지 않았습니다.

## 실행 증거

Cargo는 직렬, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] 신규 `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --test outcome -- --nocapture`: 1 PASS, compile 0.71초/suite 0.03초입니다. actual TerminalCore의 cursor/TUI CSI·숨은 escape·OSC title과 legacy ASCII outcome의 전체 split 경계, byte-split CJK/결합 문자/supplementary·UTF8 overlap, synchronized text/OSC 동시 flush, 여러 허용 feed를 합친 flush text overflow·core retire/재개 거절을 하나의 연속 검사로 확인했습니다. synthetic 문자열만 사용했습니다.
- [x] `cargo clippy --manifest-path native/taide-native-terminal/Cargo.toml … --lib --tests -- -D warnings`: exit 0(0.55초)입니다. 기존 성공 core/title runtime body는 반복하지 않았고 새 text 소비 경계만 검사했습니다.
- [x] 공유 source 이동의 기존 spike 소비자 `cargo check --manifest-path experiments/terminal-core-spike/Cargo.toml … --lib --tests`: exit 0(5.23초)입니다. 실제 source 비교는 동일이며 module 경로/모든 test target compile을 확인했습니다. 같은 기존 text/OSC runtime 성공은 재실행하지 않았습니다.
- [x] native lib/stream/new test·spike lib의 exact `rustfmt --edition 2024 --config skip_children=true --check`: exit 0입니다.

## 실제 retained 조사와 다음 경계

설치 Alacritty 0.26.0 `term/{mod,cell,color}.rs`, `grid/{mod,storage,row}.rs`의 실제 field를 읽었습니다. public active grid만 순회해서는 전체 소유 비용을 계산할 수 없습니다.

1. Term은 private active/inactive grid·title/title_stack·두 keyboard mode stack·TabStops·damage Vec·Config 문자열을 보유합니다. Cursor/saved_cursor template Cell도 extra Arc를 가질 수 있습니다. native raw trace/title log의 개인정보 경계도 아직 처리하지 않았습니다.
2. Grid의 private Storage는 Vec<Row<Cell>>이며 활성 len 이외 resize/cache row를 보유합니다. 원본은 MAX_CACHE_SIZE 1,000행을 남길 수 있어 `history+rows`만으로 실제 row 수를 설명할 수 없습니다. 각 Row의 Vec capacity와 CellExtra의 private zerowidth Vec/Hyperlink Arc/String capacity·공유 identity가 필요합니다.
3. Processor의 private state/parser raw OSC·sync buffer·observer owner, Core의 Arc<Mutex<Pending>>·effect Vec capacity·formatter callback·text/tail과 방문 queue까지 구분해야 합니다. 제목/질의/observer의 unknown owner를 0으로 계산하지 않습니다. 기존 taide-native-retained의 bounded visitor·Arc dedup·Opaque/Locked 실패 API를 직접 확인했으며 실제 source의 선택적 typed adapter가 다음 구현입니다.

현재 결과는 정규화 소비 경계만 완료입니다. 실제 retained admission·총 CPU/RSS/allocator/GPU·PTY attach/snapshot/live·writer/query·input/IME·selection/search/link/native terminal surface·다중 session/window·전체 N4/N1~N8을 완료 처리하지 않습니다. TypeScript 제거·commit/push는 전체 M8 완료 뒤입니다.
