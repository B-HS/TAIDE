# M8 원격 미러 쓰기 receipt·정확한 조건부 정리

## 대상 파일

- `crates/taide-model/src/ids.rs`, `crates/taide-model/src/file.rs`
- `crates/taide-file/src/service.rs`, `crates/taide-runtime/src/file_actions.rs`
- `native/taide-native-app/src/remote-files.rs`, `native/taide-native-app/src/remote-files-tests.rs`

## 리포트

기존 `file_mirror_dirty`의 `Option<f64>`는 디스크 baseline이며 쓰기 식별자가 아닙니다. 쓰기 뒤 목록을 읽거나 클라이언트가 timestamp를 만들어 `MirrorEntry`를 비교하면 정확한 쓰기를 식별하지 못합니다. 내용·baseline·시각이 모두 같은 별도 쓰기까지 구별하는 서버 발급 `MirrorWriteId`와 실제 write receipt를 추가했습니다. 이 경계는 실제 native 원격 dispatcher·runtime·파일 서비스이며 browser 쓰기 owner/epoch/500ms timer/flush 연결 완료가 아닙니다.

## 계약

1. `file_mirror_dirty`의 optional `receipt:true`만 `{writeId,entry}`를 반환합니다. 없거나 false이면 기존 baseline number/null입니다. 잘못된 bool은 쓰기 전에 거부합니다. 새 command/allowlist/타우리 signature/생성 TS binding은 추가하지 않았습니다.
2. 기존 UUID v4 의존성을 가진 model의 ID 패턴으로 `MirrorWriteId`를 발급합니다. 모든 legacy/receipt 쓰기는 같은 service mutex 안에서 새로운 ID·실제 storage metadata baseline·서버 `savedAtMs`를 만든 뒤 한 번 persist합니다. receipt는 해당 persist의 값이며 unlock 후 목록 재조회가 아닙니다. `entry.conflict=false`, `sourceMissing=baseline 없음`은 기존 TS의 쓰기 직후 cache patch와 같습니다. display path는 원본 그대로, 저장 키는 권한을 검증한 canonical path입니다.
3. 내부 `MirrorFile.write_id`는 default None/None 직렬화 생략으로 이전 JSON을 읽습니다. `file_list_mirrors`의 기존 `MirrorEntry` 필드/resolve 정책은 유지하며 ID를 목록에 노출하지 않습니다. 새 legacy 쓰기도 ID를 발급하므로 이전 writer의 receipt가 새 legacy 초안을 지우지 않습니다. 이전 버전 앱이 새 파일을 덮어쓰면 ID가 없어져 receipt 비교는 false입니다.
4. `file_clear_mirror`의 optional `expectedReceipt`는 `MirrorWriteReceipt`로 검증합니다. `expected`와 동시 제공 또는 null/잘못된 모양은 거부하며 ordinary clear로 fallback하지 않습니다. open project/root/canonical/mutation guard와 service mutex 아래 저장된 ID·원본 path·내용·서버 시각·write baseline을 함께 비교하고 같을 때만 삭제합니다. 현재 미러가 없으면 true로 멱등 완료합니다. expected 내부 path는 파일시스템 target이 아닙니다.
5. receipt 정리는 목록용 현재 디스크 conflict/sourceMissing으로 비교하지 않습니다. 원본 파일이 이후 삭제·변경되어도 자신의 정확한 쓰기를 정리할 수 있으며 다른 ID의 새 쓰기는 보존합니다. 기존 `expected:MirrorEntry|null` 모드·ordinary null 반환은 유지합니다. 기존 타우리 remote gateway에는 이 optional mode가 없으므로 Rust browser/native mandatory paired bundle에서만 사용합니다.
6. 기존 `file_mirror_dirty`의 async global mutation guard 미취득·TaskSupervisor operation/동일 blocking worker·shutdown 거부를 유지합니다. cross-process 앱/수동 FS 변경을 service mutex로 동기화한다고 주장하지 않습니다. 최종 성능 게이트에서 lock의 실제 비용을 확인합니다.

## 검증

- [x] actual native 새 검사 `remote_files::tests::mirror_쓰기_receipt는_동일_초안의_새_쓰기를_구별하고_legacy와_권한을_보존한다`: 구현 전 baseline scalar에 writeId 없음 RED(exit101, build9.05초/suite.04초), 실패1만 구현 뒤 GREEN(exit0, build26.15초/suite.06초)입니다. 동일 초안의 새 ID·old receipt false·current/absent true·malformed/서로 충돌한 기대값 거부·legacy false/누락 number 및 ordinary null·outside write/closed project 거부를 확인했습니다.
- [x] 실제 service 새 검사 `service::tests::mirror_receipt는_동일_내용_시각_디스크_변경과_legacy_쓰기를_구별한다`: 첫 PASS(exit0, build2.67초/suite.05초)입니다. 시험 fixture에서 두 쓰기의 시각까지 같게 고정해 `first.entry==second.entry`인데 ID만 다름을 확인하고 old false/current true, 원본 삭제 후 own receipt 정리, absent 멱등, missing baseline null, 새 legacy 쓰기/ID 없는 이전 JSON 보존을 확인했습니다. 같은 성공을 반복하지 않았습니다.
- [x] 변경된 root worker 영향 검사 `file_actions::tests::등록된_파일_worker는_열기_저장_복사_mirror의_결과를_유지한다`: PASS(exit0, build9.19초/suite.04초)입니다. actual open/save/copy/legacy mirror·감독 count0·shutdown 거부를 확인했습니다. receipt 도입으로 본문이 바뀐 worker 경계이며 이전 unchanged CAS/native GUI 성공의 반복이 아닙니다.
- [x] native caller strict `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --lib -- -D warnings`: exit0, 7.58초입니다. vendor Wry의 기존17개 deprecated/unsafe 경고와 linker 기존 경고를 억제하지 않았습니다.
- [x] root service test strict `cargo clippy -p taide-file --lib --tests -- -D warnings`: exit0, 2.06초입니다. model을 소비하는 production browser `cargo clippy --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib -- -D warnings`: exit0, 1.82초입니다. 위 공통 locked/offline/target 옵션을 사용했습니다. root4/native2 정확한 파일 rustfmt check·관련 tracked diff 검사는 exit0입니다. 최종 문서/미추적7파일 trailing whitespace/final newline 문제0·PROCESS active363/433이며 live Cargo/browser handle은 없습니다. 앞선 미러 조회·복원 문서5개 최종 whitespace도 문제0을 확인했습니다.

명령은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 같은 경로의 `bin/cargo`, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`입니다. 신규 dependency/버전/lock/MSRV·제품 TS·OS 설정·보호 앱·Git 변경은 없습니다. 새 합성 임시 파일만 사용했습니다.

## 남은 실제 경계

- BrowserEditor의 same owned connection으로 lazy draft→500ms write·typed receipt/response·pending epoch·실제 save/choice settle·강제 flush·조건부 정리/오류/Closed/no automatic replay를 연결합니다. 클라이언트 추정 savedAtMs로 CAS하지 않습니다.
- 원본 복원과 adoptUnobservedModelEdit는 pendingMirror를 세우지 않습니다. 실제 handleChange만 pending 쓰기를 예약하며 dirty와 pendingMirror를 구별해야 합니다. 실제 원본 본문을 다시 대조해 앞선 adopt 설명을 정정했습니다. 저장 중 추가 편집/경로·프로젝트 해제·더 최신 writer/원본 삭제 복구와 cache never-seed도 유지합니다.
- 실제 browser timer/blur/unmount/close 및 오류·sourceMissing SaveAs UI·원본 query GC/rescan throttle·전체 App/canvas/consumer/제품 bundle·N1~N8/beta/cutover/Rust99%는 미완료입니다. 이 서버 계약 완료로 상위 체크박스를 체크하지 않습니다.

## 공식 참조

[Serde field attributes](https://serde.rs/field-attrs.html)의 default/skip_serializing_if, [Mutex](https://doc.rust-lang.org/std/sync/struct.Mutex.html)의 guard 수명/poison 오류, [UUID v4](https://docs.rs/uuid/latest/uuid/struct.Uuid.html#method.new_v4)를 확인했습니다. 기존 model ID 생성·JSON float_roundtrip·persist/write/권한 경로를 재사용했으며 새 의존성을 추가하지 않았습니다.
