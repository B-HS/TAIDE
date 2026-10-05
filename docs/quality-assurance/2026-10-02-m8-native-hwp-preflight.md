# M8 HWP5/HWPX 입력 admission 기본 경계

## 대상 파일과 범위

`native/taide-native-app/src/preview_hwp_preflight.rs`, `src/preview_spreadsheet_xls.rs`, `src/lib.rs`, app Cargo/lock과 `tests/preview-hwp-{preflight,reference}.rs`입니다. 입력을 검사하는 순수 Rust 함수 `admit`를 추가했습니다. 후속 `preview_hwp::Document::load`가 이 admission을 호출하며 정확한 native engine·기본 page/SVG/raster·표 grid 경계 결과는 `2026-10-02-m8-native-hwp-preview.md`, 기본 승인 worker/cache/File surface 결과는 `2026-10-02-m8-native-hwp-host.md`에 기록했습니다. 전체 HWP provider·N5-P1g·M8는 미완료입니다.

## 구현

1. encoded 20MiB, decoded 논리 스트림 합산 64MiB, container entry 4096개, XML event/HWP record 합산 262144개, 구조 깊이 128, embedded CFB/ZIP 깊이 16, entry 이름 4096바이트/합산 1MiB를 검사합니다. CFB entry 수에는 root/storage도 포함합니다. nested container의 자식 스트림은 다시 합산하므로 중복 논리 데이터에 보수적인 제한입니다.
2. XLS에서 이미 검증한 CFB FAT/DIFAT physical preflight를 두 번째 실제 소비자로 재사용합니다. 함수 visibility만 변경했으며 XLS 동작은 바꾸지 않았습니다. HWP FileHeader·암호/배포 플래그와 DocInfo 존재, stream advertised length/실제 길이·합산을 확인합니다. 일반 CFB open은 설치 cfb 0.15.0의 기본 validation이며 upstream cfb 0.14 strict/lenient와 동일하다고 주장하지 않습니다.
3. 원본 [rhwp v0.8.2 parser](https://github.com/edwardkim/rhwp/tree/v0.8.2/src/parser)의 raw deflate→zlib 순서를 사용합니다. 압축 결과는 remaining+1까지만 읽고 read 오류가 생겨도 이미 상한을 초과했으면 raw fallback으로 숨기지 않습니다. compressed DocInfo/BodyText에는 성공한 압축 해제가 필요하고 BinData는 원본의 압축 시도 후 raw fallback을 따릅니다. record 길이는 payload 복사 없이 checked 범위로 검사하며 extended size·level·count를 확인합니다.
4. ZIP32 EOCD/central header를 먼저 검사해 metadata constructor 전에 entry 수·name 길이·declared decoded 합산을 제한합니다. 이후 실제 CRC/길이·압축 방식과 bounded read를 확인합니다. duplicate/split/ZIP64·비표준 central directory 경계는 명시적으로 거절합니다. XML의 root/태그 짝·깊이/attribute·DTD/unknown entity를 확인하며 문자열은 원본처럼 UTF-8 lossy로 읽습니다. XML 유효성 전체 판정기는 아닙니다.
5. HWPX content.hpf를 archive 순서와 무관하게 먼저 읽고 `application/xml` 참조도 XML 검사 대상으로 포함합니다. 실제 upstream `content.rs::attr_value`는 unescape/공백 정규화 없이 raw lossy 문자열을 사용하므로 admission의 href도 동일한 raw 값을 사용합니다. `.dat`, literal `&amp;` 이름, newline 이름의 참조를 확인했습니다. 외부 href를 파일/네트워크로 읽거나 engine의 external-image 보충 API를 호출하지 않습니다.

이 admission 단위는 기존 native lock의 flate2 1.1.9(MIT OR Apache-2.0, MSRV 1.67) direct edge만 추가했습니다. 설치 upstream crate source에서 Read/decoder·CFB·ZIP spec/API와 quick-xml 0.41을 확인했습니다. docs.rs/crates.io 웹 조회는 도구에서 접근 오류였으므로 조회 성공으로 기록하지 않습니다. 당시 lock의 새 package와 root manifest/lock/MSRV 변경은 없으며 기존 zlib-rs/miniz 선택을 유지했습니다. 후속 rhwp 엔진 단위의 36 package·path dependency는 별도 QA에 기록했습니다. Rust 함수/타입과 bounded streaming buffer mutation은 기존 native Rust 관례와 성능 경계를 따릅니다.

## 실제 검증

공통 명령 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. Cargo는 하나씩 실행했습니다.

- 초기 compile에서 잘못된 XmlVersion import와 BytesText의 모호한 as_ref를 각각 actual crate API/직접 iter로 수정했습니다. 검사 억제는 추가하지 않았습니다.
- 처음 `cargo test ... --test preview-hwp-preflight`: 3건 중 XML/ZIP와 압축 팽창 2건 PASS, CFB fixture 1건 FAIL(compile 6.89초, suite 0.90초). `CompoundFile::create`의 V4/4096바이트 섹터를 512바이트로 계산해 잘못된 위치를 변조한 테스트 문제였습니다. fixture 생성만 V3로 명시한 뒤 CFB filter 1건 PASS(compile 1.14초, suite 0.02초)입니다. 일반/deflate/zlib·record 잘림/extended size/count/깊이·FAT/advertised length·nested OLE 거절을 포함합니다.
- 별도 manifest `.dat` 참조 재현 1건은 처음 admission이 Ok를 반환해 RED(0.00초)였습니다. manifest-first/참조 검사와 raw href 보존을 적용한 뒤 `cargo test ... --test preview-hwp-reference --test preview-hwp-preflight hwpx`: 실제 XML/ZIP 경계 1건 PASS(0.12초), 확장 raw href 참조 1건 PASS(0.00초)입니다.
- V3 fixture 변경 영향을 받는 `hwp_admission은_deflate` filter만 1건 PASS(0.84초)입니다. 실제 64MiB+1 raw-deflate DocInfo/BinData·encoded 제한·두 ZIP entry의 declared 합산 거절을 확인했습니다.
- 신규 `hwpx는` filter 1건 PASS(compile 0.78초, suite 1.04초). EOCD 두 entry count를 함께 4097로 변조한 pre-constructor 거절과 중앙 size를 1로 위조한 실제 64MiB+1 ZIP 압축 팽창 거절을 확인했습니다. 성공 후 테스트 이름의 과도한 “할당전” 표현만 제거했으며 동작은 바꾸지 않았습니다.
- 초기 strict의 fixture collapsible-if 1건을 let-chain으로 정정했습니다. 최종 `cargo clippy ... --lib --bin taide-native-app --test preview-hwp-preflight --test preview-hwp-reference -- -D warnings` exit 0(0.37초)입니다. 이미 통과한 XML/HTML/기존 XLS 동작 검사는 재실행하지 않았습니다.

## 남은 gate

- [x] 후속 단위에서 정확한 revision의 native dependency/compile·production template·synthetic page/SVG/raster와 HWP5/HWPX 합산 table grid metadata cap을 확인했습니다. 새 table target 2건 RED→수정→2 PASS와 실제 native 검증은 `2026-10-02-m8-native-hwp-preview.md`에 기록했습니다. cap은 경계 행·열을 포함한 합산 262144 slot이며 전체 IR/RSS 보호는 아닙니다.
- [x] 후속 기본 approved worker·stale/close/shared cache/File surface 연결은 `2026-10-02-m8-native-hwp-host.md`에 기록했습니다. 기존 source checkout은 `/private/tmp/taide-rhwp-source.O6KKTB`이며 고정 source는 app vendor에 보존했습니다. persistent engine/CPU와 전체 입력/memory gate는 남습니다.
- [x] 후속 배포용 ViewText의 bounded decrypt/decompress·extended header·같은 record/table cap 연결은 `2026-10-02-m8-native-hwp-distribution.md`에 기록했습니다. 기존 password-encrypted 거절은 원본처럼 유지합니다.
- [ ] HWP3/HML admission은 미구현이며 명시적 오류입니다. 원본의 large/lenient/corpus를 이 오류로 대체해 완료 처리하지 않습니다. 원본 75MiB HWPX XML 등은 이 basic 64MiB 제한에서 거절될 수 있습니다.
- [ ] admission의 decoded 크기는 Vec capacity·중간 fallback·UTF-8 lossy/attribute 결과·CFB/ZIP metadata/IR/layout/font/image/SVG/GPU의 peak RSS 상한이 아닙니다. Vec growth와 limit+1의 검출 버퍼도 포함해 전체 메모리 보호를 완료로 주장하지 않습니다. record payload 내부의 allocation count·table/paragraph expansion·ZIP parser 버전 차이·native strict/lenient fallback·종류별 큰 resource/전체 corpus는 후속 엔진 연결 전 검토·검증 대상입니다.
- [ ] 전체 archive 외부 resource/embedded image 렌더, CPU/cancel/crash isolation·실제 OS/GPU/AX·원본 font/줄바꿈/픽셀 동등성과 N1~N8/Rust99%/TS 제거/배포는 미완료입니다. 사용자 실기 bundle·앱·설정·데이터는 조작하지 않았습니다.
