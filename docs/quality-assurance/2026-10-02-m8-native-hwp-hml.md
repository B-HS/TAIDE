# M8 HML content admission과 native page

## 대상과 구현

`native/taide-native-app/src/preview_hwp_preflight.rs`, `tests/preview-hwp-hml.rs`입니다. 고정 rhwp v0.8.2의 HML signature·strict encoding·reader·adapter를 기존 HWP Document/load 경로에 연결했습니다. 원본 파일 분류와 같이 `.hwp`/`.hwpx`의 content 지원이며 새 `.hml` 확장자 분류나 외부 리소스 fetch는 추가하지 않았습니다.

1. 원본 `hml::detect_hml_signature`와 `encoding::decode`를 재사용합니다. UTF-8·UTF-8 BOM·BOM UTF-16LE/BE를 검사하고 decoded UTF-8 길이를 기존 64MiB 합산에 예약합니다. 원본 bytes를 Cow::Borrowed로 그대로 engine에 전달하며 인코딩·metadata를 다시 쓰지 않습니다.
2. 기존 XML node/depth/attribute/DTD/entity/root 검사를 재사용합니다. HWPX의 local `tbl`·정규화 attribute와 HML의 exact `TABLE`·`RowCount`/`ColCount`·비정규화 unescape를 구분합니다. HML text/CDATA에는 원본 HmlLimits의 text node 한계도 적용합니다. Version 2.9/2.91·HEAD/BODY 등 실제 semantic 검증은 원본 engine이 담당합니다.
3. 7개 language FONT와 BORDERFILL/CHARSHAPE/PARASHAPE/TABDEF/STYLE의 sparse array high-water를 실제 모델 struct size로 합산해 engine resize 전에 검사합니다. 같은 Id의 중복은 이중 예약하지 않고 BORDERFILL은 1-based Id입니다. 원본 max_resource_id 초과는 hard error로 바꾸지 않고 경고·skip 동작을 유지합니다.
4. HML element마다 ancestor name/path 길이를 보수적으로 합산해 짧은 source에서 반복 경고·preserved fragment path가 확대되는 입력을 미리 거절합니다. 미지원 subtree 내부도 보수적으로 계상하므로 완전한 원본 수용 집합과 같다고 주장하지 않습니다. 이 값은 logical 비용이지 Vec capacity·문자열 복사·전체 IR/RSS/CPU cap은 아닙니다.

## 실제 검증

공통 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. Cargo는 직렬로 실행했고 사용자 앱/bundle·설정·clipboard·문서는 조작하지 않았습니다.

- [x] 최초 `cargo test ... --test preview-hwp-hml` 1 FAIL(compile 5.14초/suite 0.00초): 원본 DocumentCore가 synthetic HML을 읽지만 native admission은 미구현 거절이었습니다.
- [x] 연결 후 같은 target 1 PASS(compile 2.69초/suite 1.05초): CJK·entity·보조 평면 문자 본문이 네 encoding에서 동일 page count·size·RGBA와 실제 ink를 반환합니다. 원본 bytes가 borrowed로 보존됩니다.
- [x] 새 독립 위험만 `cargo test ... --test preview-hwp-hml hml_admission` 1 PASS(compile 1.29초/suite 0.37초): 실제 cell/table native page, Version 2.9 admission, 거대/합산 table metadata 거절, sparse Id skip/warning·정상 high-water 중복, 12 resource table 합산 거절, depth/node/path 예산과 DTD/unknown entity/잘못된 root/숫자 공백/UTF-8/UTF-16 잘림·surrogate·missing HEAD를 검사했습니다. 거대 grid/resource를 engine에 할당하지 않았습니다. 이전 성공 encoding 렌더는 반복하지 않았습니다.
- [x] 공유 XML 분기의 영향만 `cargo test ... --test preview-hwp-table-budget hwp_table_dimensions` 1 PASS(compile 0.92초/suite 0.00초)입니다. 기존 HWPX/CFB grid 정책을 유지합니다. 다른 성공 host/UI/crypto 검사는 재사용합니다.
- [x] `cargo clippy ... --lib --bin taide-native-app --test preview-hwp-hml -- -D warnings` exit 0(1.21초)입니다. 새 dependency·lock/root/MSRV·vendor source 변경이나 검사 억제는 없습니다.

## 남은 gate

- [ ] HWP3 admission/native page·HML 전체 corpus·보수적 admission으로 거절되는 큰 정상 문서·reference canvas/native font/line wrap/pixel 차이입니다.
- [ ] 전체 IR/layout/image/font·engine의 두 번째 decode/parse·Vec capacity/in-flight/RSS·CPU/cancel/crash isolation·page별 재파싱/persistent engine 비용입니다. 원본 preserved fragment order의 반복 순회 CPU 비용도 별도 검증이 필요하며 이번 연결만으로 해결됐다고 하지 않습니다.
- [ ] OS/GPU/AX·다른 provider/terminal·N1~N8/Rust99%/TS 제거/배포는 남습니다. 전체 M8 완료 뒤만 commit·push합니다.
