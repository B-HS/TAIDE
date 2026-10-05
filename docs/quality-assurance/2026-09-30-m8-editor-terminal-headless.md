# M8 editor·terminal headless 실험

## 대상과 범위

`experiments/editor-core-spike`, `experiments/terminal-core-spike`, `experiments/wezterm-core-spike`는 각각 독립 Cargo workspace입니다. 제품 Cargo workspace·TS/Tauri 앱·실제 프로젝트·저장 형식은 변경하지 않습니다. OSC 정책을 복제하지 않기 위해 기존 infra의 classifier를 `classify_osc_payload`로 공개하고 기존 scanner도 같은 함수로 연결했습니다. 분류 본문과 기존 scanner 동작은 그대로입니다. 합성 문자열과 고정된 OS 폰트만 사용합니다. 결과는 후보 조사이며 product DocumentStore·native renderer·IME·LSP·PTY 기능 동등성 통과가 아닙니다.

## 버전·라이선스

| 대상 | 실제 고정 버전 | 공식 MSRV | 라이선스 |
| --- | --- | --- | --- |
| Ropey | 1.6.1 | metadata에 미표시 | MIT |
| COSMIC Text | 0.19.0 | 1.89 | MIT OR Apache-2.0 |
| Tree-sitter | 0.27.0 | 1.90 | MIT |
| Rust grammar | 0.24.2 | metadata에 미표시 | MIT |
| unicode-segmentation | 1.13.3 | 기존 제품과 동일 버전 | MIT OR Apache-2.0 |
| alacritty_terminal | 0.26.0 | 1.85 | Apache-2.0 |
| wezterm-term | 공식 Git `cab25161054c50fd6c705db4ceefef0f1e5a9575`의 0.1.0 | metadata에 미표시 | MIT |
| GPUI 제한 조사 | 등록 package 0.2.2 | metadata에 미표시 | Apache-2.0 |

`cargo info`는 root workspace 환경에서 최신보다 낮은 호환 버전을 반환할 수 있어 Alacritty·COSMIC Text·Tree-sitter는 명시적 최신 버전으로 다시 확인했습니다. Ropey의 최신 prerelease는 2.0.0-beta.1이며 이번 실험은 기존 char·UTF-16 API를 가진 안정 1.6.1입니다. beta 채택이나 제품 의존성 확정은 하지 않았습니다. 제품 MSRV는 아직 올리지 않았습니다.

공식 자료: [Ropey 1.6.1](https://docs.rs/ropey/1.6.1/ropey/struct.Rope.html), [COSMIC Buffer](https://docs.rs/cosmic-text/0.19.0/cosmic_text/struct.Buffer.html), [Tree-sitter](https://tree-sitter.github.io/tree-sitter/), [Alacritty Term](https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/struct.Term.html), [WezTerm 고정 source](https://github.com/wezterm/wezterm/tree/cab25161054c50fd6c705db4ceefef0f1e5a9575/term), [등록 GPUI 0.2.2](https://docs.rs/crate/gpui/0.2.2/source/).

## editor 검사: 4건 통과

```sh
cargo test --manifest-path experiments/editor-core-spike/Cargo.toml
```

실제 결과 4 passed / 0 failed, test 실행 0.17초입니다. 이 시간은 UI 성능 기준선이 아닙니다.

- byte·scalar·UTF-16 왕복, 한글·일본어·결합 악센트·supplementary scalar·CRLF grapheme와 줄 인덱스. UTF-8 중간 byte·UTF-16 surrogate 중간·범위 밖 위치를 거부합니다.
- 한 transaction의 서로 다른 두 edit, Unicode 결과, stale revision·잘못된 경계의 원자적 거부, undo/redo의 증가하는 revision, 동일 삽입점 충돌, 새 편집 후 redo 폐기를 검사합니다.
- Rust grammar의 incremental edit 결과를 전체 parse의 tree 구조와 비교하고 syntax error 부재를 확인합니다.
- COSMIC Text advanced shaping에서 CJK·결합 문자·supplementary scalar·Hebrew bidi의 실제 glyph 존재·cluster byte 경계·RTL level·hit-test 경계를 검사합니다. 50,000줄의 처음·중간·끝에서 visible layout run 상한과 viewport 위치를 확인합니다.

`DocumentProbe`는 후보 실험용입니다. canonical URI, encoding·read-only·dirty baseline, view selection transform, IME transaction, save/hot exit, history 메모리 상한과 indexed grapheme 접근은 미완료입니다. 아래 추가 연결에서 제품 LSP coordinator의 typed formatting/revision 경계를 재사용했지만 전체 native editor/LSP 기능 동등성은 아닙니다. 위 테스트만으로 native editor 완료나 키 입력·스크롤 p95를 주장하지 않습니다. 시스템 폰트가 없는 환경에서는 shaping 테스트의 환경 조건을 충족해야 합니다.

## editor의 LSP 좌표·revision 연결: 추가 3건 통과

대상: `experiments/editor-core-spike/src/lsp_coordinates.rs`, module export, `tests/lsp-coordinates.rs`와 실험 Cargo manifest/lock입니다. 별도의 손으로 작성한 Position/Range/TextEdit DTO 대신 제품 `taide-lsp::native::protocol::lsp_types`를 재사용합니다. 제품 LSP path와 이미 쓰는 serde_json을 실험에 추가했고 transitive package는 독립 editor lock에만 추가됐습니다. 제품 Cargo dependency·root lock·MSRV·실행 경로는 이 연결로 바꾸지 않습니다.

- [공식 Text Documents](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/textDocuments.md)의 CR/LF/CRLF만 줄바꿈으로 해석합니다. Ropey 1.6.1 기본 feature인 unicode_lines는 NEL/U+2028 등을 줄로 세므로 실험에서 default features를 끄고 cr_lines/simd만 켰습니다. 이 설정에서 Unicode 구분 문자는 같은 줄의 문자입니다. 미래 제품 buffer 선정 때도 같은 계약을 유지해야 합니다.
- [Position](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/position.md)의 큰 character는 줄 내용 끝으로 clamp합니다. signed/uinteger 범위를 벗어난 값·존재하지 않는 줄·UTF-8 중간 byte·UTF-16 surrogate 중간·CRLF의 CR과 LF 사이 byte는 거절합니다. 빈 문서와 trailing CR/LF/CRLF의 마지막 빈 줄도 왕복합니다. CRLF 자체의 위치를 문자 column으로 노출하지 않습니다.
- Ropey의 line/byte/scalar/UTF-16 indexed API를 한 곳에서 사용하며 문자열 전체를 매번 scan하는 별도 line map을 만들지 않습니다. 좌표 변환과 typed edit는 요청 revision을 먼저 확인하고 모든 edit를 현재 buffer에서 변환한 뒤 기존 transaction으로 한 번 반영합니다.
- [TextEdit 배열](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/textEditArray.md)은 같은 위치의 여러 삽입 및 뒤따르는 한 replacement를 허용합니다. stable sort로 서로 다른 위치를 정렬하고 같은 위치는 원본 배열 순서로 합칩니다. replacement 뒤 같은 위치 삽입·겹친 range·reversed range·잘못된 좌표·stale revision은 buffer/revision/undo를 바꾸지 않습니다. 기존 raw byte Edit API의 동일 위치 충돌 거절 계약은 유지합니다.
- 실제 제품 pure coordinator의 formatting reply를 TypedReply로 검사하여 CJK/supplementary UTF-16 위치에 적용하고 증가한 revision/text를 mirror에 반영합니다. 요청 뒤 타이핑하면 coordinator가 StaleRevision으로 끝내고 editor도 오래된 revision의 edit를 거절합니다. undo는 새 revision으로 원본 mirror에 반영합니다. mock reply 기반 headless 경계이며 실제 process/UI의 전체 formatting 기능 완료로 표시하지 않습니다.

```sh
cargo test --manifest-path experiments/editor-core-spike/Cargo.toml --offline --test lsp-coordinates
cargo test --manifest-path experiments/editor-core-spike/Cargo.toml --locked --offline
cargo clippy --manifest-path experiments/editor-core-spike/Cargo.toml --locked --offline --all-targets -- -D warnings
```

새 API 부재 E0599 red 뒤 신규 3건이 0.00초에 통과했고 줄바꿈 feature 변경 영향이 있는 기존 4건은 같은 한 번의 cargo 검사에서 0.17초에 통과했습니다. strict clippy 4.47초·exit 0입니다. API 구현 후 실패한 assertion은 없으며 unchanged 제품 session·transport 검사는 재실행하지 않았습니다. 이 실험의 누적 자동 검사는 7건이며 UI 성능값이 아닙니다.

canonical DocumentId/URI·live allowed roots·closed/reopened 문서의 identity, private buffer/revision ownership, multi-document WorkspaceEdit 원자성·annotation 확인·resource operation·save race, syntax/decoration/selection transform·bounded history·grapheme 인덱스와 실제 IME는 미완료입니다. 현 DocumentProbe의 공개 rope/revision은 후보 검사 편의용이며 제품 DocumentStore의 단일 소유 경계로 인정하지 않습니다. large one-line conversion/edit 메모리와 p95, 실제 server의 clamping/CRLF 대응은 제품 채택 및 native 활성화 전에 확인합니다.

## 대형 문서 후보의 release 비용 관측

대상: `experiments/editor-core-spike/src/bin/document-cost.rs`입니다. 기존 DocumentProbe·indexed 좌표·typed TextEdit·undo/redo를 그대로 호출하는 독립 측정 도구이며 제품 코드·실험 dependency·lockfile은 이 단계에서 수정하지 않았습니다. 합성 텍스트만 만들고 파일·폰트·GUI·사용자 설정에 접근하지 않습니다. 실제 검사는 설치된 Rust 1.98.1과 macOS Apple Silicon에서 수행했습니다.

[RopeSlice의 공식 좌표 API](https://docs.rs/ropey/1.6.1/ropey/struct.RopeSlice.html#method.char_to_utf16_cu)는 indexed 변환을 제공합니다. 별도 전체 문자열 scan이나 line map을 추가하지 않았습니다. [Instant](https://doc.rust-lang.org/std/time/struct.Instant.html)와 [black_box](https://doc.rust-lang.org/std/hint/fn.black_box.html)의 공식 계약을 확인해 경과 시간과 최적화 장벽을 사용했습니다. OS 시간 관측과 black_box는 통계적 안정성 또는 성능 보증이 아닙니다.

```sh
cargo build --manifest-path experiments/editor-core-spike/Cargo.toml --bin document-cost --release --locked --offline
/usr/bin/time -l experiments/editor-core-spike/target/release/document-cost
cargo clippy --manifest-path experiments/editor-core-spike/Cargo.toml --bin document-cost --locked --offline -- -D warnings
```

release build는 22.53초·exit 0, 대상 strict clippy는 0.50초·exit 0입니다. 첫 실제 도구 실행은 아래 JSON 관측값과 내용 검사 성공을 출력했지만 `/usr/bin/time -l`이 `sysctl kern.clockrate: Operation not permitted`로 exit 1이어서 peak RSS는 얻지 못했습니다. 사용자 파일이나 앱을 읽는 권한이 아닌 해당 측정 wrapper의 시스템 시계 읽기만 escalation한 뒤 같은 실행을 한 번 보완해 wrapper exit 0과 RSS를 얻었습니다. 실제 실행은 총 2회이며 첫 실행의 성공한 시간값은 바꾸지 않습니다. 이를 단일 실행에서 모든 관측이 성공했다고 표기하지 않습니다.

| 합성 문서 | 생성 | 좌표 왕복 p50 / p95 / 최대 | 단일 삽입 | undo / redo |
| --- | --- | --- | --- | --- |
| CRLF 50,000행, 600,000 bytes | 186,292ns | 625 / 875 / 16,375ns | 3,209ns | 41 / 0ns |
| 단일 행, 40,000,000 bytes | 7,965,208ns | 1,125 / 1,333 / 3,708ns | 6,625ns | 42 / 0ns |

각 fixture는 `한𐐀e`와 NFD 결합 악센트의 반복이며 다중 행에만 CRLF를 붙입니다. trailing CRLF 때문에 첫 rope의 줄 수는 50,001입니다. 좌표 왕복은 서로 다른 분산된 유효 경계 128개에서 byte→LSP Position→byte를 측정합니다. p50·p95는 정렬한 표본의 nearest-rank이고, 준비용 char→byte 계산·assertion은 구간 밖입니다. 생성 시간은 이미 만든 String을 rope로 변환하는 구간이며 초기 String 반복 생성은 제외합니다. 삽입 구간에는 edit vector 할당과 apply가 포함되며 동일 fixture의 첫 삽입 한 건입니다. undo/redo에는 성공 assertion을 포함합니다. `0ns`는 그 관측의 시계 해상도 아래 결과이며 비용이 0이라는 의미가 아닙니다.

후속 RSS 보완 실행의 process 전체 maximum resident set size는 92,471,296 bytes(88.1875MiB), peak memory footprint는 87,966,128 bytes입니다. 두 fixture는 순서대로 처리하며 동시에 보유하지 않습니다. peak에는 생성 중 원본 String+rope, 임시 할당과 원본/편집 후 snapshot 및 런타임이 포함되므로 순수 rope의 메모리 크기 또는 fixture별 RSS로 분해하지 않습니다. wrapper의 0.02초 real·0.01초 user·0.00초 sys도 해당 보완 실행 전체이며 첫 실행의 operation 시간과 같은 실행의 값으로 합치지 않습니다.

두 실제 실행에서 좌표 왕복·UTF-8/UTF-16 길이 증가·정확한 삽입 내용, 원본 snapshot 길이 보존, undo/redo의 전체 rope 내용 일치와 최종 revision 3을 확인했습니다. unchanged editor 단위 검사 7건의 성공 증거는 재사용하고 반복하지 않았습니다. 새 도구는 probe의 동작을 수정하지 않습니다.

이 관측은 균일한 headless 합성 문서의 buffer 비용입니다. 실제 코드의 비균일 줄·대량 편집·긴 undo history·다중 문서·syntax/shaping·LSP serialize/copy·IME/렌더·CPU soak·제품 RSS 상한은 미검증입니다. M7 UI/terminal 기준선과 같은 경계가 아니므로 성능 동등성, UI input-to-paint p95, 제품 MSRV 1.89 실행 검증 또는 후보 hard gate 통과 근거로 계산하지 않습니다. 동일 성공값을 반복 수집하지 않고 native 소비 경계가 연결되면 그 별도 비용을 측정합니다.

## Alacritty 검사: 3건 통과

```sh
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml
```

실제 결과 3 passed / 0 failed, test 실행 0.07초입니다. 80열·24행·history 상한 128행의 합성 fixture입니다.

- CJK·NFD 결합 문자·CR overwrite·CSI clear/cursor·alternate 복구·bracketed paste/application cursor·OSC title/hyperlink 입력·OSC52 입력을 모든 byte 분할 경계로 나눠 grid text·cursor·history·mode·event snapshot 동일성을 검사했습니다.
- 별도 확정 기대 문자열·cursor·alternate 복구·mode·title를 검사했습니다. `Osc52::Disabled`로 copy/query 모두 clipboard event 0이고 정상 출력이 유지됩니다.
- 1,000줄 burst 뒤 history 상한 128행과 마지막 출력 보존을 검사했습니다.

현재 후보 간 snapshot은 텍스트·cursor·history·일부 mode·title/clipboard/PTY event만 비교합니다. 추가 단위 검사에서는 SGR truecolor·bold/underline·wide-cell flags·명시적 OSC8 링크 target의 전체 입력/one-byte 입력 동일성, Unicode selection/search와 partial damage를 확인했습니다. 80 → 40 → 80열 resize에서도 history와 화면을 합친 전체 출력과 유효한 cursor를 보존했습니다. 추가 검사 3건이 통과해 Alacritty 단위 검사는 총 6건입니다.

resize 검사의 최초 실패는 화면의 177개 문자가 모두 같은 visible 영역에 남아야 한다고 가정한 assertion입니다. 실제 visible 출력은 97개 문자였으며 upstream `grid/resize.rs`가 reflow 중 일부 행을 history로 보내는 것을 확인했습니다. 실패한 검사만 전체 history+화면의 무손실 조건으로 수정해 1회 재검증했고 통과했습니다. 삭제된 출력으로 간주하거나 화면 문자열만 비교해 회귀를 숨기지 않습니다.

native render/input/IME, PTY stream·성능, 제품 전용 OSC consumer와 안전한 normalized text의 shadow parity는 남았습니다. 기존 OutputScanner는 제거하지 않았습니다.

## 단일 parser OSC·메모리 상한: 계약 3건·기존 54건 통과

독립 실험의 `vendor/vte` 0.15.0에만 path patch를 연결했습니다. 기존 source·license를 보존하며 ANSI 모듈의 Apache-2.0 고지를 유지합니다. 제품 및 WezTerm Cargo root는 이 patch를 상속하지 않습니다.

- 표준 vte는 `std`에서 OSC raw buffer가 Vec이며 끝나지 않은 OSC에 상한이 없음을 source에서 확인했습니다. 수정은 const buffer 상한(기본 4096 bytes), 초과 명령 전체 폐기, 정상 명령 복구를 추가합니다. 잘린 title·clipboard 명령을 실행하지 않습니다.
- `OscObserver`는 기존 ANSI parser의 dispatch에서 structured params를 받습니다. 별도 escape scanner로 raw bytes를 다시 파싱하지 않습니다. 미지원 OSC의 debug 로그도 외부 payload를 출력하지 않도록 수정했습니다.
- 먼저 callback 계약의 API 부재 E0432 red를 확인했고, 구현 뒤 기본 grid 검사 3건과 계약 검사 2건이 통과했습니다. 추가로 synchronized update의 observer 재사용과 해제 후 호출 부재 1건이 통과했습니다.
- `cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml -p vte --all-features --offline`의 기존 parser 검사 54건이 통과했습니다. oversized OSC의 기존 unit 기대값은 무제한/잘린 명령 dispatch 대신 명령 전체 폐기라는 강화된 계약으로 변경했습니다.
- `cargo clippy --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --all-targets -- -D warnings`은 exit 0입니다. 성공 결과는 수정 위험이 겹치지 않으면 재사용합니다.

이 검사는 callback과 raw parser 상한의 증거입니다. 실제 effect consumer의 event·문자열 상한, OSC 7·9·133·777 정책, 악성 URL 거부·정규화·정확한 scanner shadow parity·제품 history 메모리·native terminal surface 완료를 주장하지 않습니다.

## OSC 원형 payload·effect shadow 추가 검증

기존 structured params는 최대 16개이며 세미콜론이 많은 본문을 온전히 복원할 수 없었습니다. 기존 params 처리를 유지하면서 같은 parser에 상한이 있는 원형 payload buffer와 observer 경계를 추가했습니다. 별도 escape parser는 추가하지 않았습니다. 원형에는 구분자·C0가 포함돼 공격자가 무시되는 문자로 상한을 우회하지 못합니다. 두 OSC buffer는 각각 최대 4096 bytes이며 전체 원형 payload가 초과하면 명령 전체를 폐기합니다.

원형 observer는 BEL 또는 완전한 ST 뒤에만 호출합니다. ST를 시작하는 ESC만 받은 시점·CAN/SUB 취소·ST가 아닌 ESC 전환은 제품 effect로 채택하지 않습니다. 기존 params dispatch의 upstream 종료 동작은 그대로이며, 새 consumer는 안전한 원형 observer만 사용합니다.

1. `observe_payload` API 부재 E0407 red 뒤 `--test osc-contract`의 5건이 통과했습니다. 32개 추가 구분자·C0 원형 보존, 분할 ST와 취소, 구분자/C0까지 포함하는 상한을 검사했습니다.
2. parser 변경 뒤 기존 `vte --all-features` 54건과 Alacritty `--lib` 6건이 통과했습니다. 이전 성공 상태가 바뀐 parser·constructor에만 재검증했으며 이후 결과는 재사용합니다.
3. `--test osc-shadow` 3건이 통과했습니다. 모든 byte 분할 위치에서 OSC 7·133·0/2·777·9의 순서와 별도 확정 기대값을 기존 실제 `OutputScanner`와 대조했습니다. file URI와 progress 제외, raw 문자열 상한, 잘못된 UTF-8 거부, 초과 OSC 뒤 정상 복구·OSC52 차단을 확인했습니다.
4. synchronized update는 flush 후 순서를 유지합니다. 한 advance의 보류 effect는 256개까지이며 포화 시 `EffectOverflow`를 지속 반환해 유실을 성공으로 보고하지 않습니다. OSC probe는 기존 진단용 title/PTY 응답 누적을 사용하지 않으며 clipboard event 계수는 유지합니다. 제품의 backpressure·회복 정책·PTY 응답 전달 구현은 아직 아닙니다.
5. strict clippy의 byte array 표기 1건을 수정한 뒤 같은 대상 검사 1회 재실행이 exit 0입니다. 검사 억제는 추가하지 않았습니다.

```sh
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --test osc-contract
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline -p vte --all-features
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --test osc-shadow --lib
cargo clippy --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --all-targets -- -D warnings
```

실험은 기존 `taide-infra`의 타입·상수·classifier를 path dependency로 재사용해 정책 복제를 피합니다. 그 transitive 의존성은 실험 Cargo.lock에만 추가했으며 제품 root lock·MSRV는 변경하지 않았습니다. 일반 텍스트·overlap·TUI 정규화 shadow, 실제 PTY 연결·effect 소비처·URL 보안·렌더 입력 게이트는 남았습니다. OSC 비교만으로 scanner 전체 제거 조건을 통과 처리하지 않습니다.

## 단일 parser의 텍스트·overlap 정규화: 추가 3건 통과

`StreamObserver`는 원형 OSC뿐 아니라 같은 ANSI parser의 print·execute·CSI dispatch를 받습니다. `OscObserver`와 기존 setter는 호환 경계로 유지합니다. `normalized-stream.rs`는 escape bytes를 다시 파싱하지 않고 구조화된 cursor 이동·screen switch와 Unicode 문자만으로 텍스트를 만듭니다. 기존 `ScanOutcome` 타입을 재사용합니다.

```sh
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --test text-shadow
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --test text-shadow tui_커서
```

첫 실행은 2 passed / 1 failed이며 DEL이 새 결과에만 남았습니다. observer의 print 입력에서 C0·DEL을 제거하도록 구현을 수정했고 실패한 TUI 검사만 1회 재실행해 1 passed / 0 failed입니다. 통과한 Unicode·상한 검사는 같은 상태에서 반복하지 않았습니다.

- 합성 TUI의 같은 row CUP·다른 row CUP·forward column·상대 row·alternate 복구, OSC/DCS/SOS/PM/APC·charset/SGR 제외, Tab·C0·DEL에서 모든 byte 분할 지점별 text·events·overlap이 기존 실제 scanner와 일치합니다.
- CJK·NFD·supplementary scalar를 한 byte씩 보내도 native 결과를 합치면 원문이 보존되고 U+FFFD가 없습니다. overlap은 UTF-8 문자 경계와 128-byte 상한을 유지합니다.
- synchronized update에서 text·OSC가 flush 후 같은 결과에 나오며 pending text는 64KiB까지입니다. 초과 뒤에는 지속 오류를 반환해 유실을 성공으로 취급하지 않습니다.

같은 byte 분할에서 기존 `OutputScanner`는 chunk마다 `from_utf8_lossy`를 적용해 CJK가 U+FFFD로 바뀌는 것을 재현했습니다. 새 parser는 UTF-8 partial state로 보존합니다. 이는 legacy 결함과 native 개선 차이이며 “모든 scanner 결과가 완전히 같다”는 완료 주장은 하지 않습니다. 기존 제품 scanner는 교체하지 않았습니다.

현재 callback은 문자별 Mutex를 사용하므로 성능 통과 근거가 아닙니다. u16 parser 범위를 넘는 CSI·잘못된/취소된 escape의 legacy 차이, decoded 문자와 실제 charset/repeat 렌더 관계, payload 경계에서 event+text 순서, 큰 stream의 backpressure·복구·PTY 응답 전달은 제품 통합 전에 검증합니다. 합성 테스트의 64KiB는 실험 버퍼 정책이며 대형 PTY 출력을 제품에서 거부하도록 확정한 계약이 아닙니다.

## 기존 실제 PTY의 단일 parser·snapshot/live 연결: 1건 통과

대상: `experiments/terminal-core-spike/src/bin/pty-fixture.rs`, `tests/pty-stream.rs`와 실험 manifest/lock의 기존 Tokio dev-dependency입니다. `taide-infra::pty::spawn`, public write·completion_handle·wait_for_completion을 직접 재사용합니다. 제품 코드·dependency·root lock·종료 정책은 바꾸지 않았습니다.

- Rust 합성 fixture는 추가 argument를 거절하고 사용자 shell/profile/설정 파일을 읽지 않습니다. 자신의 PTY에 상속된 `/bin/stty -echo -onlcr`만 실행해 input echo와 kernel newline 변환을 끄며 해당 짧은 child도 status로 회수합니다. 이는 합성 PTY의 line discipline이며 macOS 입력기·VoiceOver·시스템 설정을 바꾸는 명령이 아닙니다. GUI도 실행하거나 조작하지 않았습니다.
- 실제 PTY의 output callback은 같은 OscEffectProbe를 한 번 호출해 grid·normalized text·OSC effect를 함께 만듭니다. 별도 OutputScanner 또는 parser를 두지 않습니다. title 준비 event를 관찰한 뒤 같은 owner에서 snapshot을 복사하고 고정 continue 명령을 실제 PTY write로 보내 후속 1,000행을 생성합니다. sleep으로 준비 시각을 추정하지 않습니다.
- 초기 CJK/NFD/supplementary 화면과 normalized text, live 전체 출력의 정확한 원문·중복/누락 부재, 준비 title→command output marker→완료 title의 순서, 128행 history 상한과 마지막 row, OSC52 clipboard event 0을 확인했습니다. fixture의 합성 text capture는 64KiB 상한이며 제품 memory 정책을 대신하지 않습니다. 이 검사는 임의 byte 분할을 강제한 테스트가 아니며 앞서 통과한 byte-fragment 검사는 재사용합니다.
- exit status 0을 받은 뒤에도 같은 3초 공통 deadline 안에서 completion.wait_for_completion을 기다리고 is_finished를 확인합니다. child exit만으로 reader/flusher/callback 완료라고 처리하지 않습니다. test 종료 때의 session Drop은 같은 회수된 자원을 닫으며 사용자 앱·프로세스를 종료하지 않습니다.

```sh
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --offline --test pty-stream
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --test pty-stream
cargo clippy --manifest-path experiments/terminal-core-spike/Cargo.toml --locked --offline --all-targets -- -D warnings
```

최초 검사 파일이 없는 writer() accessor를 사용해 E0599가 발생했습니다. 실제 public write API로 바꾼 뒤 유일한 실제 PTY 검사 1건이 0.34초에 통과했습니다. Copy result의 drop 경고와 collapsible-if 정적 경고를 결과 무시 방식·let chain으로 수정한 뒤 strict clippy만 다시 실행해 0.15초·exit 0을 확인했습니다. 변경은 제어 흐름이나 PTY 입력을 바꾸지 않아 성공한 실제 process 검사를 반복하지 않았습니다. source가 같은 core·OSC·text·legacy infra 성공 검사도 재사용합니다.

이 경계는 snapshot 이후 같은 parser가 계속 받는 live stream의 증거입니다. 실제 native subscriber의 snapshot/replay cursor·attach/detach race·input key/IME·mouse/mode-aware encoder·PTY 응답 소비·GPU surface·remote snapshot/live 전달·backpressure와 CPU/RSS/p95 검사는 미완료입니다. xterm/legacy scanner·Tauri 경로는 유지하며 제품 terminal 의존성 선정과 cutover도 하지 않았습니다.

## Terminal 후보의 live mode 입력 계약: 1건 통과

대상: `experiments/terminal-core-spike/src/input.rs`, 기존 lib의 module export, `tests/input-contract.rs`입니다. 제품 source·dependency·lockfile·기존 PTY fixture와 출력 parser는 바꾸지 않았습니다. API는 같은 AlacrittyProbe가 소유한 Term의 live mode를 직접 읽고 별도 mode mirror나 escape scanner를 만들지 않습니다.

현재 `src/features/terminal/terminal-view.tsx`와 설치된 xterm 6.0.0의 `src/common/input/Keyboard.ts`, `src/browser/Clipboard.ts`를 대조했습니다. TAIDE는 macOptionIsMeta를 켜며 조합 중이 아닌 plain Shift+Enter를 LF로 보냅니다. xterm의 paste는 CRLF/LF를 CR로 정규화한 뒤 현재 bracketed paste 설정을 사용합니다. [공식 xterm control sequence](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)의 cursor normal/application, 수정 키·기능 키와 FocusIn/Out 계약도 확인했습니다. Alacritty 고정 source의 kitty_keyboard 기본값은 false이며 실험은 이를 그대로 유지합니다.

```sh
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --test input-contract --locked --offline
cargo clippy --manifest-path experiments/terminal-core-spike/Cargo.toml --lib --test input-contract --locked --offline -- -D warnings
rustfmt --edition 2024 --check --config skip_children=true experiments/terminal-core-spike/src/input.rs experiments/terminal-core-spike/src/lib.rs experiments/terminal-core-spike/tests/input-contract.rs
git diff --check
```

입력 API 부재 E0432/E0599 red 뒤 신규 검사 `1 passed / 0 failed / 0 filtered out`, 0.00초를 확인했습니다. 새 미사용 상수를 제거하고 modifier bit 연산의 괄호를 명확하게 수정한 뒤 대상 strict clippy만 한 번 재실행해 0.24초·exit 0을 확인했습니다. 같은 값의 연산 순서와 실행 동작은 바뀌지 않아 성공한 입력 검사는 반복하지 않았습니다. 대상 format·diff는 exit 0이며 기존 출력/OSC/text/PTY 성공 증거를 재사용합니다.

- [x] parser에 실제 `?1h/l`을 입력한 뒤 normal/application cursor와 Home/End, 수정 키 CSI, command-arrow 미전송을 확인했습니다. F1~F12, 수정 기능 키, PageUp/Down의 local scroll action, Shift/Ctrl Insert의 host clipboard action, Delete/Tab/Backspace/Enter/Escape와 ASCII control 조합을 확인했습니다.
- [x] plain Shift+Enter는 LF, Alt+Enter는 ESC+CR이며 command-A는 local SelectAll action입니다. encoder는 native host가 이미 분류한 Key와 modifier를 입력받으며 app keymap이 소비할 이벤트를 대신 처리하지 않습니다.
- [x] preedit는 PTY byte를 만들지 않고 확정 text는 CJK·NFD·supplementary UTF-8를 그대로 유지합니다. 이는 OS IME 조합/후보 창/취소/중복 commit을 실기한 증거가 아닙니다.
- [x] parser의 실제 `?2004h/l`에 따라 Unicode paste의 줄바꿈 정규화·한 쌍의 bracket이 바뀝니다. 정규화 후 길이와 bracket 길이를 합산해 byte 할당 전에 caller 상한을 검사하며 정확한 경계는 허용, 한 byte 부족 및 UTF-8 한글 크기 부족은 Capacity입니다. 64KiB는 test caller의 값이고 제품 paste 제한 계약이 아닙니다.
- [x] parser의 실제 `?1004h/l`에 따라 focus 상태만 CSI I/O로 전달하거나 무시합니다. 예상하지 못한 kitty mode의 key 입력은 UnsupportedMode로 거절하도록 구조를 두었으며 kitty protocol 자체를 구현하거나 광고하지 않습니다. 이 분기는 별도 실행 증거가 없습니다.

현재 API는 macOS의 기존 meta 정책을 기준으로 한 후보 인코딩 경계입니다. 실제 native keydown/text/IME event adapter의 단일 전달·repeat/release·physical key/다른 keyboard layout·dead key·CapsLock·NumLock·application keypad·mouse/alternate scroll·modifyOtherKeys/kitty·platform별 keymap·composition 중 Shift+Enter guard는 다음 게이트입니다. ClipboardShortcut·SelectAll·PageUp/Down action을 실제 host에 연결하지 않았고 focus event 중복 억제도 아직 host 책임으로 구현하지 않았습니다.

paste는 기존 xterm처럼 내부 control byte를 보존하므로 embedded `ESC[201~` 또는 shell command를 sanitize한 보안 경계가 아닙니다. 불신 clipboard·멀티라인 확인·embedded bracket 정책은 native 제품 연결 전 합의/검증해야 합니다. 당시 남았던 OscEffectProbe consumer의 단일 owner 입력 port와 제한된 실제 PTY 왕복은 아래 별도 검사에서 연결했습니다. 제품 입력 큐·backpressure·취소된 write·GUI 화면과 latency는 미완료입니다. 이 1건을 native terminal 전체 입력/IME parity 또는 제품 cutover 완료로 계산하지 않습니다.

## 동일 terminal owner의 실제 PTY 입력 왕복: 1건 통과

대상: `src/osc-effects.rs`의 thin 입력 port, `src/bin/pty-input-fixture.rs`, `tests/pty-input.rs`입니다. 입력 port는 기존 private terminal의 encode_input에 그대로 위임하고 출력과 같은 parser/mode owner를 유지합니다. 두 번째 terminal/parser·mode mirror·writer를 만들지 않았습니다. 기존 제품 source·dependency·lockfile·출력 fixture는 바꾸지 않았습니다.

새 합성 Rust child는 argument를 거절하고 사용자 shell/profile/파일을 읽지 않습니다. 상속된 자기 PTY에서만 `/bin/stty raw -echo -onlcr`를 실행하고 짧은 stty child의 status를 기다립니다. raw는 정확한 Ctrl-C·CR/LF·ESC 수신 검사를 위한 fixture line discipline이며 사용자 앱·OS 입력기·VoiceOver·시스템 설정을 바꾸지 않습니다. 사용자 shell의 cooked 동작이나 실제 native keyboard event를 검증하는 우회로 사용하지 않습니다.

```sh
cargo test --manifest-path experiments/terminal-core-spike/Cargo.toml --test pty-input --locked --offline
cargo clippy --manifest-path experiments/terminal-core-spike/Cargo.toml --lib --bin pty-input-fixture --test pty-input --locked --offline -- -D warnings
rustfmt --edition 2024 --check --config skip_children=true experiments/terminal-core-spike/src/osc-effects.rs experiments/terminal-core-spike/src/bin/pty-input-fixture.rs experiments/terminal-core-spike/tests/pty-input.rs
git diff --check
```

OscEffectProbe의 입력 port 부재 E0599 red 뒤 신규 실제 검사 `1 passed / 0 failed / 0 filtered out`, 0.36초에 통과했습니다. 대상 strict clippy는 0.27초·exit 0이며 대상 format·diff도 exit 0입니다. 신규 성공 검사는 반복하지 않고 unchanged input unit·출력/OSC/text/PTY·infra 성공 증거를 재사용합니다.

- [x] 하나의 실제 session에서 normal → application → reset을 연속 실행했습니다. child는 각 mode sequence를 쓰고 OSC title ready event를 보내며 test는 같은 출력 parser에서 ready를 관찰한 뒤 입력을 인코딩합니다. sleep·snapshot boolean으로 준비 상태를 추정하지 않습니다.
- [x] normal 단계의 cursor CSI·Ctrl-C byte·CR Enter·LF Shift+Enter·CJK/NFD/supplementary 확정 text·줄바꿈 정규화 paste를 실제 child가 읽었습니다. preedit와 비활성 focus가 추가 byte를 보내지 않는 것도 뒤따르는 독립 literal 기대 stream 대조에 포함합니다.
- [x] application 단계는 SS3 cursor·Alt Backspace·FocusIn/Out·bracketed Unicode paste를 읽습니다. mode reset 뒤 normal cursor·unbracketed paste로 복구하고 비활성 focus는 전송하지 않습니다. child의 기대 byte는 encoder를 import하지 않는 literal이며 각 read_exact 결과가 일치한 뒤에만 received acknowledgement를 출력합니다.
- [x] ready/received 세 쌍과 final title의 순서, exit status 0, 같은 3초 공통 deadline 내 completion.wait_for_completion·is_finished와 OSC52 clipboard event 0을 확인했습니다. child exit만으로 reader/flusher/callback join을 완료 처리하지 않습니다.

이는 raw 합성 receiver와 기존 직접 write API에 연결한 native 후보 입력의 byte 증거입니다. 실제 shell/TUI별 line discipline·native window focus/key/IME event·input 큐와 backpressure·쓰기 취소·attach/replay/remote cursor·renderer·다중 창·성능·보안 paste 정책과 제품 composition root는 미검증입니다. PTY write는 fixture의 작은 fixed stream이며 UI/event loop의 비차단 writer 계약을 대신하지 않습니다. 제품 terminal crate 채택·native terminal 기능 동등성 또는 xterm 제거 gate를 완료 처리하지 않습니다.

## WezTerm 비교: 1건 통과·2건 실패, 현재 No-Go

crates.io에서 `wezterm-term`을 찾지 못한 실제 결과를 확인하고 공식 Git revision에 고정했습니다. Cargo.lock은 transitive Git 의존성도 고정합니다. upstream 소스와 submodule을 다운로드했으며 제품 의존성에는 연결하지 않습니다. `cargo check --locked --offline`은 성공했습니다.

```sh
cargo test --manifest-path experiments/wezterm-core-spike/Cargo.toml --locked --offline
```

실제 결과 1 passed / 2 failed입니다. 성공은 1,000줄 burst history의 두 후보 동일성입니다. 실패 검사를 같은 상태로 재실행하지 않았고 assertion을 낮추거나 fixture를 바꿔 숨기지 않았습니다.

1. `cursor` fixture에서 화면과 cursor는 같지만 CSI `2J` 뒤 Alacritty history는 `["old"]`, WezTerm history는 빈 배열입니다. 화면 지우기에서 history를 보존하는 정책 차이이며 동일 digest gate는 실패입니다.
2. `unicode` fixture의 split 31은 `e` 바로 뒤이며 결합 악센트 U+0301 이전입니다. 한 번에 보내면 `e + U+0301`이 보존되지만 별도 조각으로 보내면 WezTerm 출력은 `e`만 남습니다. 이는 PTY의 임의 조각 경계에서 실제 Unicode 유실이므로 무손실 gate No-Go입니다.

원인 근거는 고정 source의 `term/src/terminal.rs::advance_bytes`와 `term/src/terminalstate/performer.rs::Drop/flush_print`입니다. 각 advance마다 새 Performer가 만들어져 종료 시 print를 flush하고, 다음 조각의 독립된 zero-width grapheme를 elide합니다. NFD 결합 문자 유실을 숨기는 buffering 우회·NFC 설정 강제·실패 검사 무시는 하지 않습니다. 현재 후보는 탈락이며 국소 upstream 수정과 증거가 없으면 제품에 채택하지 않습니다.

WezTerm 실험에는 clipboard handler를 등록하지 않았으므로 OSC52가 실제 OS clipboard에 도달할 경로가 없습니다. parser가 해당 OSC를 인식하지 않는다는 주장은 하지 않습니다. 색·이미지·OSC 전용 효과·배포 부하·유지보수 비교는 완료하지 않았습니다.

## GPUI 제한 source 조사

등록 GPUI 0.2.2의 `src/platform.rs::InputHandler`는 selected/marked text와 replace/unmark, UTF-16 range, 후보 창용 bounds·point 변환을 제공합니다. `src/app.rs::open_window`는 GPUI 자체 Window·platform 경로를 만듭니다. 이 API 존재만으로 eframe pane에 embed 가능하다고 가정하지 않습니다.

등록 package의 Cargo·macOS platform source에서 AccessKit·accessibility 구현을 확인하지 못했습니다. upstream main의 Cargo에는 AccessKit 의존성이 있으나 같은 0.2.2 표기만으로 등록 package의 지원과 같다고 판단하지 않습니다. 이번 제한 조사는 source 확인까지이며 GPUI 실행·VoiceOver·IME·외부 DnD·pane host 실기는 미검증입니다. shell 우선 후보가 아닌 editor 연구 후보로 유지합니다.

## 남은 판정

- [ ] shell hard gate와 사용자 실제 IME·VoiceOver 결과
- [ ] Alacritty의 실제 제품 consumer·PTY 연결·native terminal surface·성능. 단일 parser의 OSC 및 제한된 text/overlap shadow·raw/event/text 상한·색/링크/resize/selection/search의 초기 headless 증거는 확보했으며 legacy Unicode 분할 결함은 명시적 차이로 남습니다.
- [ ] Ropey prerelease 비교 여부·실제 editor surface/IME/accessibility·incremental decoration·LSP generation/revision/replay
- [ ] GUI·syntax 후보의 MSRV 상향 및 제품 채택 계약, release signing·패키징·성능 기준선

제품 TS/Tauri·Monaco·xterm 제거와 M8 전체 완료 조건은 변경하지 않습니다.
