# M8 native PPTX worker·cache·화면 연결

## 대상과 결과

대상은 `native/taide-native-app/src/{preview,preview_status,preview_pdf,preview_pdf_surface,preview_presentation,preview_presentation_cache,preview_presentation_surface,host,application,lib}.rs`와 `tests/preview-presentation-host.rs`입니다. 기존 parser 성공은 `2026-10-02-m8-native-pptx-outline-parser.md`에서 재사용했습니다. 실제 File surface의 Presentation 분기를 연결했고 lib/bin compile을 확인했습니다. 전체 PPTX fidelity·실제 OS/GPU/AX·N5/M8 완료는 아닙니다.

1. typed Request·SourceReady·Result를 기존 bounded host와 approved blocking worker에 연결했습니다. bytes 읽기 이전에는 빈 배경, 읽기 성공 이후 처음 parse 중에는 공통 loading입니다. Read는 unsupported/filename/외부 열기, Decode는 presentation loadFailed/외부 열기입니다. raw read와 parse 뒤 모두 기존 canonical root 승인·shutdown을 재검사합니다. 외부 entity/relation은 parser에서 접근하지 않습니다.
2. 같은 path의 Arc outline은 공유하며 tab 선택은 독립적입니다. bytes invalidation은 기존 ready outline·선택·scroll ID를 유지하고, 새 parse 성공 뒤 선택을 첫 slide로 초기화합니다. parse 실패는 ready outline을 회수합니다. Decode error는 다음 parse 성공까지 유지하지만 Read error는 새 bytes가 준비되면 inner loading으로 바뀝니다. 첫 ready/error→ready는 새 scroll generation이고 ready→ready는 기존 scroll generation입니다. 실제 OS scroll offset까지 확인한 것은 아닙니다.
3. 한 window에서 한 parse만 inflight이며 checked monotonic token·invalidation·path 이동·tab/project close·queue 실패·reconnect/shutdown 경계를 연결했습니다. 닫힌/변경된 path의 progress/result와 이전 token은 cache에 다시 반영하지 않습니다. 같은 outline은 중복 합산하지 않고 image/PDF/PPTX cache의 128MiB 합산 admission을 유지합니다. outline 자체는 기존 64MiB 상한이며 raw/decoder 임시 버퍼와 UI layout/Arc 참조를 포함한 실제 전체 RSS 상한은 아닙니다.
4. 원본 outline disclaimer·192px sidebar·28px slide button·선택/hover token·본문 14px/20px와 paragraph 간 8px·empty 문구를 연결했습니다. 실제 key Enter로 slide를 바꾸며 WidgetInfo는 원본의 button 역할입니다. HTML처럼 ASCII 공백을 표시 시에만 collapse하고 NBSP는 보존합니다. 문자열을 script/markup로 실행하지 않습니다. en/ko/ja 메시지와 원본 app.border/selected/hover/warning/editor theme token을 사용합니다. CJK shaping·CSS 줄바꿈·icon stroke·스크롤바·모든 theme의 픽셀 동등성은 별도 gate입니다.
5. PDF와 PPTX에서 중복되는 loading/Read/Decode/외부 열기 화면을 `preview_status.rs`로 옮겼습니다. 기존 PDF 호출 경계와 action ID helper를 유지하고 decode 메시지 key만 provider별로 전달합니다. PDF decoder·worker·geometry는 변경하지 않았습니다. OS 외부 앱을 실제 열지 않고 action intent만 검사했습니다.

## 실제 검사

- [x] `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app` — exit 0, 1.28초. worker/cache와 실제 application 분기의 compile입니다.
- 최초 test 호출은 세션 반환 metadata를 기록하지 못해 완료 로그를 회수하지 못했습니다. 성공 증거로 세지 않았습니다. 다음 출력 확보에서는 합성 Project fixture의 필수 필드 누락으로 compile 실패(0.25초), 잘못된 Default 가정의 수정에서도 compile 실패(0.23초)였습니다. 실제 Project 정의에 Default가 없음을 확인해 기존 fixture와 같은 네 필드를 명시했습니다. 제품 코드의 runtime 실패로 해석하지 않습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-presentation-host -- --nocapture` — 수정 뒤 2건 PASS, compile 0.92초·suite 0.07초. 첫 성공 뒤 동일 검사는 반복하지 않았습니다.
    - actual host: approved raw read→단일 progress→parse 결과, 같은 path 공유/독립 선택, ready 유지/갱신 성공의 초기화, root component 경계, invalidate 중 stale result/progress, cancel/reset_pending, 실제 파일 path 이동, 상대/outside/닫힌 project 거절, broken ZIP의 Decode 분리, close 회수·disconnect/shutdown 뒤 tracked_count 0입니다.
    - actual egui frame: blank/loading·disclaimer·본문 공백/NBSP/14px/20px과 sidebar 우측 위치, Enter slide 선택·empty, 재해석 중 old ready 유지, loadFailed/unsupported/filename·외부 열기 intent, Decode 유지/Read 해제, 합산 admission 및 잘못된 index metadata 거절·회수·회복을 en/ko/ja에서 확인했습니다. FullOutput textures_delta를 clear하여 fixture의 egui 수명을 닫습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-pdf pdf_control -- --nocapture` — 공통 status 추출에 영향받은 기존 PDF UI 1건 PASS, compile 0.88초·suite 0.07초. PDF 픽셀/geometry와 ImageIO/parser 성공은 반복하지 않았습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview-presentation-host -- -D warnings` — exit 0, 1.31초.

## 남은 gate

- [ ] 실제 window/auxiliary window의 mount·활성/비활성 전환·scroll offset·focus·DPI/GPU/AX와 원본 실파일 corpus를 검증합니다. headless Enter 검사는 실제 OS 입력기/VoiceOver 검사가 아닙니다.
- [ ] 모든 theme/locale의 긴 문구·좁은 창·CJK/RTL와 font fallback·CSS whitespace/line-break·icon·색/scrollbar/클리핑 및 전체 원본 시각 동등성을 확인합니다.
- [ ] parser QA의 ZIP central directory/metadata·임시 문자열/decoder/UI layout·large outline의 전체 RSS·cancel/crash isolation·duplicate/CRC/ZIP64/encrypted·lone surrogate 차이를 해결합니다. cache retained estimate를 전체 메모리 상한으로 주장하지 않습니다.
- [ ] 전체 PPTX provider·다른 preview provider·terminal·N1부터 N8/M8·Rust99%·TS 제거·배포를 완료합니다. 제품 TS/사용자 실기 bundle·OS 설정/clipboard를 변경하지 않았고 Git은 전체 M8 완료 뒤만 수행합니다.
