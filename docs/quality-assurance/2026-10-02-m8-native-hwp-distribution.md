# M8 배포용 HWP ViewText admission

## 대상과 구현

`native/taide-native-app/src/preview_hwp_preflight.rs`, `vendor/rhwp/src/parser/crypto.rs`, `tests/preview-hwp-distribution.rs`, `tests/fixtures/hwp-distribution-reference.json`입니다. N5-P1g의 HWP5 배포용 확장 입력 경계를 기존 Document/load/host에 연결했습니다. 전체 HWP 형식/corpus/성능 gate 완료는 아닙니다.

1. FileHeader의 distribution flag를 무조건 거절하던 경로를 교체했습니다. 원본 CfbReader와 같이 BodyText/SectionN·ViewText/SectionN·루트 SectionN의 연속 section 수를 확인하고 distribution에는 대응 ViewText가 필요합니다. 기존 password-encrypted flag는 원본처럼 계속 거절합니다. 사용자 보호 문서·키·시크릿을 사용하지 않았습니다.
2. 첫 DISTRIBUTE_DOC_DATA record의 tag·payload 256바이트·normal/extended header와 body 존재를 allocation 전에 확인합니다. 원본 LCG/AES decoder의 `compressed=false`만 재사용해 실제 ciphertext를 복호화하고, 압축은 앱의 기존 bounded deflate→zlib 순서/합산 remaining+1 검출로 해제합니다. 무압축 결과도 decoded remaining을 검사합니다. ciphertext가 물리 입력의 상한 안에 있어도 decoded logical cap과 전체 RSS cap은 구분합니다.
3. 복호화 후 실제 본문은 같은 record count/depth/길이·합산 table grid slot 검사를 거칩니다. 원본 Record::read_all은 마지막 4바이트 미만을 무시하므로 distribution 본문만 같은 short-tail 처리를 사용합니다. 전체 payload의 끝까지 trim하지 않고 실제 record 길이를 유지합니다. 일반 DocInfo/BodyText의 기존 strict tail 동작은 바꾸지 않았습니다.
4. 원본 crypto의 extended first record가 AES 시작을 4바이트 잘못 계산하는 오류를 재현했습니다. parsed payload length 256이 아니라 original encoded size marker를 사용해 header 4/8바이트를 결정하도록 vendor를 국소 수정했습니다. source 차이는 vendor UPSTREAM.md와 별도 bug에 기록했습니다.

## Fixture 출처

Production blank2010.hwp의 351바이트 Section0을 기존 설치 CFB/SheetJS 라이브러리로 읽었습니다. synthetic LCG seed 0·공개 all-zero AES test key·manual zero block padding, Bun의 node:crypto AES-128-ECB와 node:zlib raw deflate/zlib로 reference 암호문을 생성했습니다. 실제 키/사용자 문서가 아니며 runtime에는 Bun/JS 암호화 경로가 없습니다. 작은 4096바이트 압축 팽창·거대 TABLE metadata도 별도 합성 reference입니다.

CFB의 상대 slash 경로 조회는 최초 missing entry였고 실제 FullPaths의 Root Entry 접두사로 정정했습니다. 첫 reference keyHeader의 수동 전사에 3바이트가 추가돼 259바이트가 됐습니다. 생성 출력의 JSON을 기계적으로 적용해 256바이트로 정정하고 테스트에 길이 assertion을 추가했습니다. 잘못된 fixture의 decrypt 결과를 엔진 결함 증거로 세지 않았습니다.

## 실제 검증

Cargo 공통 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`이며 직렬 실행했습니다.

- [x] 최초 distribution target 2 FAIL(compile 3.93초/suite 0.21초): native admission은 미구현 거절이었고 crypto 비교에는 잘못된 fixture가 포함됐습니다. fixture 정정 후 extended filter만 실제 RED(compile 0.78초/suite 0.01초)입니다. 정상 header는 실제 paragraph record 본문을 복원했지만 extended header는 다른 바이트를 반환했습니다.
- [x] source offset 수정·bounded admission 연결 뒤 `cargo test ... --test preview-hwp-distribution`: 서로 다른 2건 PASS(compile 6.51초/suite 0.32초)입니다. normal/extended header의 plain/deflate/zlib 본문·native page count·size·RGBA가 production blank 기준과 일치하고, malformed tag/header·확장 size truncation을 거절했습니다. 단일 blank 문서이며 전체 corpus/실문서 렌더 동등성은 아닙니다.
- [x] 새 위험만 `cargo test ... --lib --test preview-hwp-distribution distribution_경계`: lib의 실제 AES→4096바이트 expansion/16바이트 decoded cap/무압축 cap 1 PASS, native admission의 복호화한 65535×65535 TABLE 거절 1 PASS입니다(compile 3.28초, 두 suite 각각 0.00초). 거대 layout을 실제 할당하지 않았습니다. 이전 성공 두 검사는 재실행하지 않았습니다.
- [x] `cargo clippy ... --lib --bin taide-native-app --test preview-hwp-distribution -- -D warnings` exit 0(3.62초)입니다. 검사 억제·dependency 추가·root MSRV 변경·사용자 앱/bundle 변경·commit/push는 없습니다.

## 남은 gate

- [ ] HWP3/HML content/encoding/count/resource/table admission과 native engine 연결입니다.
- [ ] AES ciphertext/decoded Vec capacity·engine의 두 번째 decrypt/decompress·metadata/전체 IR/layout/font/image/GPU/in-flight/RSS·CPU/cancel/crash isolation·persistent document 비용은 이 논리 admission cap으로 완료 처리하지 않습니다. 기존 unused stream까지 보수적으로 검증하는 정책과 strict/lenient 차이·큰 정상 문서·전체 corpus를 검증해야 합니다.
- [ ] 실제 OS/GPU/AX·원본 font/줄바꿈/pixel·다른 provider/terminal·N1~N8/Rust99%/TS 제거/배포는 남습니다. 성공한 기존 host/page/admission/XML/HTML 검사는 재사용하며 전체 M8 완료 뒤만 commit/push합니다.
