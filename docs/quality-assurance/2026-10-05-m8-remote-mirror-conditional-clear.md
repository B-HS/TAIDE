# M8 원격 미러 조건부 삭제

## 결과·범위

실제 native 원격 file_clear_mirror에 expected 모드를 연결했습니다. expected가 없으면 기존 unit/null 응답과 일반 삭제를 유지합니다. expected가 있으면 Option<MirrorEntry>를 검증하고 현재 항목과 같을 때만 삭제하며 bool을 반환합니다. null은 현재 미러가 없을 때만 true입니다. 새 명령/allowlist/의존성을 추가하지 않았습니다.

서비스 쓰기·일반 삭제·prune·조건부 비교/삭제가 같은 프로세스의 파일 미러 전용 Mutex를 사용합니다. 기존 root mirror_dirty의 전역 async mutation guard 미사용·blocking worker 계약은 유지합니다. native cleanup의 canonical/root/중복 항목 확인은 유지하고 최종 삭제를 같은 조건부 서비스로 교체했습니다. 브라우저에서 조회 후 일반 삭제를 호출하는 방식으로 원자성을 주장하지 않습니다.

서비스/실제 원격 dispatcher/변경된 native host 고유3검사 PASS입니다. browser 미러 조회·복원·epoch/write owner는 아직 연결하지 않았습니다. 후속47 2/4(50%)·전체363/433(83.83%, 비가중)·최종0/8·전체 ETA 산정 보류·goal active·전체 완료 전 Git 없음·main 직접입니다.

## 대상·계약

- `crates/taide-model/src/file.rs`: 기존 MirrorEntry에서 Deserialize를 유도합니다. expected의 path는 파일 접근에 사용하지 않고 실제 항목과 비교만 합니다.
- `crates/taide-file/src/service.rs`: mirror_dirty/clear_mirror/prune_mirrors/clear_mirror_if_current의 단일 잠금, 공용 resolve_mirror·clear_mirror_file입니다. 목록과 조건부 삭제의 conflict/source_missing/baseline 계산은 동일합니다. 잠금 poison은 오류로 반환하며 검사 억제나 강제 복구를 하지 않습니다.
- `crates/taide-runtime/src/file_actions.rs`, `native/taide-native-app/src/remote-files.rs`: 기존 열린 project/authorized canonical root를 다시 확인하고 expected의 존재 여부로 bool/null 모드를 구분합니다. malformed expected는 기존 삭제로 fallback하지 않습니다.
- `native/taide-native-app/src/file_sync.rs`: 원래 조회/expected 비교 뒤에도 서비스 안에서 재확인합니다. 전역 mutation guard를 쓰지 않는 원격 writer가 두 단계 사이 새 초안을 저장해도 오래된 expected로 삭제하지 않습니다.

기존 TS persistence·native cleanup·runtime mirror_dirty·persist::write_json/read_json을 대조했습니다. [Rust Mutex 공식 문서](https://doc.rust-lang.org/std/sync/struct.Mutex.html)와 [Serde derive](https://serde.rs/derive.html)를 확인했습니다. 기존 원격 프레임/입력 상한·177명령 정책·Tauri signature와 제품 TS는 변경하지 않았습니다.

## 최소 검증

공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, cargo는 같은 경로의 bin/cargo, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`이며 직렬 실행했습니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib remote_files::tests::조건부_mirror_삭제는_새_초안과_입력_권한을_보존한다 -- --exact`: 신규 RED(build7.71초/suite0.06초)의 반환값 null/기대 false를 확인했습니다. 기존 dispatcher는 expected를 소비하지 않고 일반 삭제를 호출했습니다. 구현 후 실패1만 GREEN(build19.59초/suite0.04초)입니다. 새 초안/content 보존·expected null 거절·malformed 거절·outside root 거절·현재 항목 일치 삭제·빈 상태 성공·expected 없는 기존 null·닫힌 project 거절을 연속 확인했습니다.
- [x] `cargo test -p taide-file --lib service::tests::조건부_mirror_삭제와_동시_쓰기는_새_초안을_보존한다 -- --exact`: 신규1 첫 PASS(build5.94초/suite0.03초)입니다. 합성 실제 파일의 조건부 삭제/새 쓰기 두 스레드 한 번, 최종 새 초안 유지·오래된/빈 expected 거절·원본 삭제 뒤 source_missing 비교·일치 삭제·빈 상태를 확인했습니다. 모든 스케줄을 계측한 스트레스 검사가 아니며 단일 critical section의 코드 계약과 구분합니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test disk 실제_host의_충돌_선택은_공유_layout_stale_초안과_mirror_버전을_보호한다 -- --exact`: 변경된 actual cleanup 영향1 PASS(build15.36초/suite0.04초)입니다. 실제 HostBridge·공유 두 view·선택·새 draft 미러 보존/현재 미러 정리를 검사합니다. 다른 기존 성공 검사/Chrome 시나리오는 반복하지 않았습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib --test disk -- -D warnings`: 최종4.58초 exit0입니다. 기존 Wry deprecated/unsafe17개·test/bin linker __eh_frame 경고는 유지했고 억제하지 않았습니다. Cargo/manifest/lock/기존 버전/MSRV·OS/보호 앱/사용자 데이터는 이 변경에서 불변입니다.
- [x] `cargo clippy -p taide-file --lib --tests -- -D warnings`: 새 서비스/동시쓰기 test 자체의 strict4.07초 exit0입니다. root3은 원래 edition2021, native3은 edition2024로 exactfmt를 확인했습니다. tracked 관련 diff와 untracked 관련6파일의 줄 끝 공백/최종 newline 문제0입니다. 초기 no-index diff의 변경 상태 exit1을 공백 오류로 오판하지 않고 실제 파일 검사로 확인했으며 테스트 성공은 재실행하지 않았습니다.

## 남은 경계

- [ ] 실제 BrowserEditor의 project scope·mirror 조회/복원·write epoch/지연 flush·선택/저장 뒤 조건부 정리·오류/명시 retry를 연결합니다. 이 서버 모드만으로 browser mirror 완료라고 세지 않습니다.
- [ ] 실제 Rust public UI와 native 서버의 mandatory bundle을 같이 배포합니다. legacy Tauri 서버에는 expected 모드가 없으므로 기존 TS 호스트를 이 Rust caller의 fallback으로 쓰지 않습니다.
- [ ] 최종 성능 게이트에서 여러 project의 파일 미러 전용 잠금 대기를 측정합니다. 이번 검사는 시간 성능 기준선이 아닙니다. 프로세스 외부의 별도 앱/수동 mirror 파일 수정은 이 in-process Mutex의 원자성 범위가 아니며 해당 환경을 지원할 때 별도 저장소 잠금 계약이 필요합니다.

검사 완료 후 live Cargo/server/browser handle은 없습니다. 전체 App/canvas/GUI/beta/cutover/Rust99% 게이트는 미완료입니다.
