# M8 HWP3 engine 입력 경계 선행

## 대상과 구현

`native/taide-native-app/vendor/rhwp/src/parser/hwp3/{mod,records,ole,drawing}.rs`, `tests/preview-hwp3-boundary.rs`, vendor `UPSTREAM.md`입니다. HML 연결을 마친 뒤 원본 HWP3 reader의 실제 경계 오류와 초기 과잉 할당을 수정했습니다. 아직 `preview_hwp_preflight::admit`의 HWP3 거절을 제거하지 않았으며 전체 HWP3 native provider 완료로 계산하지 않습니다.

1. 고정 header 뒤 info_block_length가 파일 범위를 넘는 panic과 24~31바이트 image metadata에서 payload offset 32의 panic을 직접 재현·수정했습니다. 정보 블록은 정확한 slice 안에서 읽으며 손상된 전체 범위는 오류입니다. 상세는 `docs/bug/2026-10-02-native-hwp3-input-boundaries.md`입니다.
2. `read_record_buf`는 원본 256MiB record hard cap을 유지하되 최대 64KiB씩 실제 read_exact가 성공한 데이터만 누적합니다. Info/Additional block, cell record, drawing extension/unknown/textbox/line attributes, field와 OLE를 연결했습니다. 거짓 length 전체를 먼저 zero-fill하지 않습니다. Drawing polygon/curve/extended polygon은 실제 point를 읽은 뒤 push하며 거짓 point_count만으로 capacity를 미리 예약하지 않습니다.
3. 공개 `decode_body(data, compressed, max_bytes)`는 raw 데이터를 Borrowed로 반환하고 raw-deflate는 max+1까지만 읽은 뒤 팽창을 거절합니다. 원본 parse_hwp3는 기존 record hard cap인 256MiB를 적용합니다. 후속 앱 admission의 64MiB 합산과 동일한 정책이라고 주장하지 않습니다. 정상 원본 compressed flag/bytes를 다시 쓰지 않습니다.

## 실제 검증

Cargo 공통 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 직렬로 실행했고 기존 사용자 앱/bundle·settings·clipboard·문서·root dependency/MSRV를 바꾸지 않았습니다.

- [x] 최초 boundary target 2 FAIL(compile 0.51초/suite 0.00초): info length와 image offset의 두 slice panic입니다.
- [x] 해당 수정 후 target 2 PASS(compile 6.93초/suite 0.00초)입니다. 이후 unchanged 성공은 반복하지 않았습니다.
- [x] 새 record 위험만 `cargo test ... --test preview-hwp3-boundary hwp3_variable_record` 1 PASS(compile 8.57초/suite 0.00초)입니다. synthetic reader는 요청 buffer가 64KiB 이하인지 확인합니다. 거짓 Additional/textbox/OLE length는 오류이고 64KiB+1의 실제 Additional payload는 정확하게 유지합니다. process RSS/allocator peak 계측은 아닙니다.
- [x] 새 압축 위험만 `cargo test ... --test preview-hwp3-boundary hwp3_decoded_body` 1 PASS(compile 0.18초/suite 0.01초)입니다. 1024바이트 raw-deflate/16바이트 상한 및 무압축 cap과 Borrowed 보존, 합성 font/paragraph/line/ASCII HWP3의 raw/deflate가 원본 DocumentCore에서 같은 page count와 실제 text SVG를 반환합니다. 앱 raster/approved host의 HWP3 연결 통과는 아닙니다.
- [x] `cargo clippy ... --lib --bin taide-native-app --test preview-hwp3-boundary -- -D warnings` exit 0(3.32초)입니다. target Rust format도 exit 0입니다. vendor 전체 재포맷·검사 억제·dependency 추가·commit/push는 없습니다.

## 다음 경계와 미완료

- [ ] HWP3 paragraph/control·drawing group/textbox의 공유 중첩/개수·table/grid 합산을 원본 IR 할당 전에 제한하는 명시적 parse context를 연결합니다. 읽는 physical body의 상한만으로 recursion stack·table Cartesian grid·변환 IR 확대가 제한되지는 않습니다.
- [ ] 앱 header/summary/info/body 20MiB encoded·64MiB 합산, font/style/record·nested OLE preflight와 기존 Document/raster/approved host 연결입니다. 원본 char/line/style·Johab/special/field·table/shape의 실제 content/corpus도 필요합니다.
- [ ] 전체 retained/in-flight/IR/Vec capacity·font/image/GPU/RSS·CPU/cancel/crash·persistent document, 원본 canvas/native font/줄바꿈/픽셀·실제 OS/AX·나머지 provider/terminal·N1~N8/Rust99%/TS 제거/배포입니다. 전체 M8 완료 뒤만 commit·push합니다.
