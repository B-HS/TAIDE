# HWP3 정보 블록·이미지 metadata slice panic

## 대상과 증상

고정 rhwp v0.8.2 `native/taide-native-app/vendor/rhwp/src/parser/hwp3/mod.rs`와 `tests/preview-hwp3-boundary.rs`입니다. 앱 admission 연결에 앞서 원본 parser를 합성 입력으로 검사했습니다.

1. 1166바이트 고정 header에서 info_block_length=65535이면 원본은 시작 index 66701로 body slice를 만들며 panic했습니다. 정보 블록 read 실패 후 cursor 위치만 선언한 끝으로 옮긴 것이 원인입니다. 정확한 info_start..body_start가 파일에 포함되는지 먼저 확인하고 별도의 제한된 Cursor 안에서만 정보 블록을 읽습니다. 잘린 전체 범위는 UnexpectedEof입니다.
2. 추가 image block 길이 24바이트에서 원본은 조건 `len >= 24` 뒤 `data[32..]`를 사용해 panic했습니다. 실제 image payload offset=32를 header 상수와 조건에 함께 적용합니다. 24~31바이트 metadata는 불완전한 image로 건너뛰고 정상 문서 parse를 유지합니다.

## 재현과 결과

공통 Cargo 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- 최초 `cargo test ... --test preview-hwp3-boundary`: 실제 두 panic으로 2 FAIL(compile 0.51초/suite 0.00초)입니다. catch_unwind는 test 재현용일 뿐 제품의 panic 우회가 아닙니다.
- 경계 수정 뒤 같은 target 2 PASS(compile 6.93초/suite 0.00초)입니다. 거대 metadata를 실제 할당하지 않았습니다.
- 후속 가변 record와 압축/body 검사는 별도 QA에 분리합니다. 이 두 panic 수정은 HWP3 전체 안전성·native provider 완료가 아닙니다.
