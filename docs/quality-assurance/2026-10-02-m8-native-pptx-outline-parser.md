# M8 native PPTX outline parser

## 대상과 범위

대상은 `native/taide-native-app/{Cargo.toml,Cargo.lock,src/lib.rs,src/preview_presentation.rs,tests/preview-presentation.rs}`입니다. 원본 `src/shared/lib/pptx-outline.ts`와 해당 TS test, `src/features/preview/presentation-preview.tsx`를 읽고 outline parser를 구현한 시점의 기록입니다. 후속 approved worker·File surface·탭별 selection/cache 연결과 새 검사 증거는 [PPTX preview 기록](2026-10-02-m8-native-pptx-preview.md)입니다. PPTX provider 전체 완료가 아닙니다.

1. 원본은 실제 슬라이드 그림/레이아웃이 아니라 `ppt/slides/slide(숫자).xml`의 번호순 paragraph outline입니다. 원본처럼 파일명 숫자를 stable sort하고 출력 index는 위치+1입니다. 원본의 정확한 `<a:p>`·`<a:t>` literal 경계와 여러 run의 연결을 유지합니다. XML attribute가 붙은 `<a:t xml:space=...>`와 다른 namespace prefix를 새로 지원한 것으로 처리하지 않습니다. 본문과 공백은 보존하며 공백뿐인 문단만 생략합니다.
2. named entity 다섯 종류와 decimal/hex code point를 한 번만 치환합니다. unknown entity는 문자 그대로 유지합니다. 원본처럼 `&amp;lt;`를 `<`로 재귀 치환하지 않습니다. DTD/외부 entity·relationship·image/URL을 해석하거나 접근하지 않습니다. ZIP을 disk에 추출하지 않습니다. UTF-8는 원본 TextDecoder처럼 replacement 방식으로 처리합니다. lone surrogate는 Rust String이 표현할 수 없어 replacement character를 사용하며 JS UTF-16 내부 문자열과 완전히 같은 표현이라고 주장하지 않습니다.
3. ZIP은 기존 app lock과 taide-infra/taide-vsix가 사용하는 zip 2.4.2를 직접 dependency edge로 재사용합니다. deflate만 활성화합니다. 기존 lock package를 사용했으며 새 package·XML/OOXML library·외부 executable은 추가하지 않았습니다. zip 2.4.2는 MIT·MSRV 1.73이며 설치된 공식 crate의 Cargo metadata와 read/write API를 읽었습니다. root manifest/lock은 이번 변경에서 수정하지 않았습니다. 최신 새 ZIP 엔진 도입이 아니라 기존 프로젝트 dependency 재사용입니다.
4. encoded bytes는 기존 20MiB, 선택된 slide들의 decompressed aggregate는 64MiB, slides는 4096개입니다. declared size를 읽기 전에 검사하고 실제 streaming read에도 remaining+1 상한을 둡니다. Stored/Deflated만 허용합니다. paragraph 문자열 capacity와 Vec 구조 estimate를 합산해 retained outline 64MiB를 검사합니다. slide마다 이전 전체 outline을 재순회하지 않고 새 slide의 비용만 누적합니다. ZIP metadata·decoder scratch·UTF-8/문자열 임시 사본을 포함한 실제 RSS의 절대 상한은 아닙니다.

## 원본 상태 수명에서 보존할 사항

PresentationPreview의 parse effect는 bytes가 교체될 때 status를 loading으로 다시 설정하지 않습니다. 따라서 기존 ready outline은 새 parse 성공/실패까지 유지됩니다. 성공 뒤에만 selectedIndex=0·ready로 교체하고 실패는 error로 바뀝니다. 처음에는 outer raw read의 빈 배경, bytes 준비 뒤 inner loading입니다. 원본 PDF의 data 교체 초기화와 같은 방식으로 무조건 canvas/outline을 즉시 비우면 안 됩니다. cleanup의 cancelled guard·slide index clamp·disclaimer·selected sidebar·noText·외부 열기를 실제 worker/cache/surface에 연결해야 합니다.

## 실제 검사와 정정

- 최초 parser 검사는 1건 PASS(compile 5.01초·suite 0.03초)입니다. 아직 native에서 JavaScript 공백 정의를 정확히 분리하지 않은 초기 결과이며 이를 최종 전체 provider 성공으로 세지 않습니다.
- [ECMAScript White Space/Line Terminator 정의](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-white-space)를 확인했습니다. Rust `is_whitespace()`는 U+0085를 포함하지만 ECMAScript trim의 WhiteSpace/LineTerminator는 포함하지 않습니다. 해당 문단 fixture를 추가해 기대 4문단이 실제 3문단으로 누락되는 RED를 재현했습니다(compile 0.68초·suite 0.00초).
- U+0085를 trim 대상에서 제외하고 U+FEFF를 포함했습니다. 문자열 원문·공백을 지우는 대신 빈 문단 판정에만 사용합니다. 같은 수정에서 retained 비용의 반복 전체 순회를 제거하고 문단별/slide별 capacity 예산을 확인합니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-presentation -- --nocapture` — 수정 뒤 1건 PASS(compile 1.66초·suite 0.03초). Stored/Deflated·번호순/leading zero·CJK/astral code point·복수 run·entity/unknown/nested replacement·빈 문단/NEL·empty slide·무시할 attribute/path/rels·외부 entity의 literal 보존·잘못된 numeric entity/UTF-8 replacement·손상/truncated/CRC/declared oversized/슬라이드 4096 상한입니다. 외부 OS 파일/URL/앱에 접근하지 않습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview-presentation -- -D warnings` — 최종 parser 변경 뒤 exit 0(0.64초). 앞선 strict 성공은 최종 결과에 중복 합산하지 않습니다. PDF/ImageIO/PNG/menu/PTY 성공은 반복하지 않습니다.

## 남은 gate

- [ ] typed request/progress/result·approved read/parse·Read/Decode failure·token/close/shutdown·같은 path의 공유 outline과 독립 tab selection, 전체 image/PDF/cache admission을 연결합니다.
- [ ] 실제 File 탭의 disclaimer·192px slide sidebar·번호/선택·paragraph/noText·외부 열기·loading/error·bytes 교체와 기존 outline 유지·scroll/keyboard/WidgetInfo/theme/3 locale를 연결하고 관련 actual host/headless 검사를 한 번 수행합니다.
- [ ] ZIP metadata/central directory의 할당 전 전체 예산, actual inflate/paragraph aggregate denial·CRC/ZIP64/duplicate entry/encrypted/다른 compression·원본의 CRC 미검사와의 차이·lone surrogate/비정상 숫자·원본 큰 파일 상한 차이·full RSS/cancel/crash isolation을 판정합니다. zip-rs는 동일한 filename entry를 IndexMap에 보관하므로 원본 hand-rolled parser의 duplicate record 의미와 같다고 주장하지 않습니다.
- [ ] 원본 실파일 corpus·실제 OS/GPU/AX·전체 PPTX provider·나머지 preview·전체 N1부터 N8/M8·Rust99%·TS 제거를 완료합니다.

이 기록의 완료 범위는 parser입니다. 실제 Presentation 탭의 후속 연결은 별도 preview 기록을 따르며 전체 PPTX/M8은 미완료입니다.
