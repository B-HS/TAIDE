# HWP ViewText extended header의 AES 시작 위치

## 대상과 실제 재현

`native/taide-native-app/vendor/rhwp/src/parser/crypto.rs::decrypt_viewtext_section`와 `tests/preview-hwp-distribution.rs`입니다. upstream v0.8.2의 first record reader는 extended size marker를 실제 payload length로 바꿉니다. DISTRIBUTE_DOC_DATA의 payload는 256바이트여야 하므로 `first.size >= 0xfff` 비교는 항상 false입니다. 따라서 실제 8바이트 extended header도 4바이트로 취급해 AES ciphertext 앞의 payload 4바이트를 body에 포함했습니다.

최초 fixture 전사 오류는 259바이트 헤더를 기계적으로 생성한 256바이트로 정정하고 길이 assertion을 추가했습니다. 정정된 합성 ViewText에서 정상 header가 production blank의 paragraph record 본문을 복원하는 반면 extended header는 다른 결과를 반환해 실제 RED를 확인했습니다(extended filter 1 FAIL, suite 0.01초). 이 오류는 사용자 문서/키로 재현하지 않았습니다.

## 수정·검증

먼저 `read_first_record`로 header 길이를 검증한 뒤, 원래 encoded u32 header의 size marker로 4/8바이트를 결정합니다. 기존 payload 256 확인·LCG/key/AES 알고리즘·normal header는 유지합니다. first record payload를 allocation 전에 검사하는 별도 app admission도 적용했습니다.

관련 target의 crypto 비교와 native blank page/render 2건 PASS, suite 0.32초입니다. plain/raw deflate/zlib, normal/extended header의 실제 복호화 바이트와 native page count/size/RGBA가 일치합니다. provider 전체 corpus·큰 문서·전체 memory/CPU/isolation·Rust-native 제품 전체 완료는 별도 gate입니다. source 변경은 vendor UPSTREAM.md에 기록했습니다.
