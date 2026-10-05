# M8 native PDF 미리보기

## 대상과 구현

대상은 `native/taide-native-app/{Cargo.toml,src/lib.rs,src/preview.rs,src/preview_macos.rs,src/preview_pdf.rs,src/preview_pdf_macos.rs,src/preview_pdf_surface.rs,src/host.rs,src/application.rs,tests/preview-pdf.rs}`입니다. 실제 File 탭의 PDF 분기를 승인 blocking worker·탭별 cache·egui surface에 연결했습니다. 제품 TS·root MSRV·사용자 실기 bundle은 변경하지 않았습니다.

1. 원본 `src/features/preview/pdf-preview.tsx`와 `src/widgets/preview-pane/preview-pane.tsx`의 상태·버튼·캔버스를 기준으로 합니다. 원본 raw read 중에는 빈 editor 배경이고, bytes 준비 후 PDF 해석 중에는 spinner와 loading 메시지입니다. `PdfSourceReady` typed 진행 응답으로 두 경계를 구분합니다. 읽기 실패는 UnsupportedPreview의 파일명·외부 열기, 해석 실패는 PDF loadFailed·외부 열기입니다. toolbar는 ready 이후만 표시합니다.
2. 탭별 page 1·scale 1에서 시작하며 page는 1부터 total, scale은 0.5부터 3까지 0.25 단계입니다. 같은 파일을 다른 탭에서 열어도 page/zoom은 독립입니다. 페이지·확대 중에는 이전 canvas를 유지합니다. 파일 변경·path 교체는 page/zoom·metadata·snapshot·texture를 초기화하며 새 scroll generation을 사용합니다. page/zoom은 scroll generation을 유지합니다.
3. 실제 toolbar는 localized page indicator·percentage, 이전/다음·축소/확대 24px 버튼과 14px Lucide vector icon입니다. keyboard Enter/Space와 WidgetInfo를 기존 egui interaction에 연결합니다. canvas는 원본처럼 natural pixel size·수평 중앙·상하 16px·양방향 scroll·shadow이며 자동 contain으로 축소하지 않습니다. 전체 CSS/font/icon cap·DPI·실제 픽셀 동등성은 별도 gate입니다.
4. macOS 표준 CoreGraphics를 기존 `objc2-core-graphics` 0.3.2에 PDF feature만 활성화하여 사용합니다. 신규 PDF package·외부 executable은 추가하지 않았습니다. approved bytes에서 CFData/CGDataProvider/CGPDFDocument를 만들며 URL provider를 만들지 않습니다. 실제 API는 설치된 공식 Rust binding과 Xcode SDK의 `CGPDFPage.h`를 읽었습니다. [Apple Quartz 좌표 문서](https://developer.apple.com/library/archive/documentation/GraphicsImaging/Conceptual/drawingwithquartz2d/dq_overview/dq_overview.html), 설치된 `pdfjs-dist/build/{pdf,pdf.worker}.mjs`의 PageViewport·UserUnit·view·rotation·white background를 대조했습니다.
5. PDF.js의 rotation·UserUnit·CropBox/MediaBox 교차·빈 교차 fallback과 소수 canvas dimension의 floor를 적용합니다. PDF.js의 위쪽 원점 viewport를 Quartz bitmap의 좌표로 다시 변환하여 위아래 뒤집힘을 막습니다. box·scale·transform의 유한성을 검사하고 할당 전에 renderer side·RGBA 64MiB를 검사합니다. white 배경과 명시적 sRGB 출력을 사용합니다. 같은 bitmap 출력 함수는 실제 두 사용처인 ImageIO와 PDF가 공유합니다.

## 승인·수명·예산

- 기존 encoded 20MiB와 live opened project/CLI path·absolute path·canonical identity·shutdown·owned mutation guard·tracked blocking worker를 재사용합니다. raw read와 PDF parse 실패 단계를 구분하면서 parse 결과 뒤에도 approval을 재검사합니다. root 밖/relative/닫힌 project와 다른 canonical path에 붙인 snapshot은 거절합니다.
- opaque `Snapshot`은 approved canonical path와 원본 bytes를 Arc로 소유합니다. Debug에는 길이만 노출하며 Eq는 Arc identity입니다. 같은 path의 탭 사이에서는 bytes snapshot을 공유하되 선택 상태와 texture는 공유하지 않습니다. page/zoom은 이 bytes를 재사용합니다. 새 bytes가 관찰되면 기존 snapshot을 무효화하고 다시 읽습니다.
- window cache당 단일 PDF request와 단조 token을 사용합니다. active request·tab·path·loading token·page/zoom을 확인해 늦은 진행 응답/완료 응답을 버립니다. 닫힌 탭·프로젝트·editor 전환은 reconcile에서 제거합니다. invalidate 뒤 진행 중인 요청은 reply를 회수할 때까지 슬롯을 유지하여 중복 parse를 막습니다. disconnect/shutdown 후 결과는 texture에 반영하지 않습니다.
- `PdfSourceReady`는 기존 bounded reply channel에 blocking worker에서 전송합니다. Tokio runtime thread에서 blocking_send하지 않습니다. receiver가 닫히면 전송이 실패하며 별도 unbounded queue를 만들지 않습니다. final result는 기존 async host reply 경로입니다.
- PDF ready pixel estimate와 unique retained raw snapshot을 합산하고 image cache의 frame/texture estimate와 함께 window당 128MiB admission을 확인합니다. source snapshot을 여러 탭에서 중복 계산하지 않습니다. 새 reply의 metadata·pixel length·renderer side를 검사한 뒤 texture를 생성합니다. 이는 parser 내부·CFData 복사·inflight result·egui upload/GPU 전체 RSS의 절대 상한은 아닙니다.
- UserUnit dictionary 읽기는 static NUL-terminated key와 살아 있는 PDF page dictionary, local f64 output pointer를 사용합니다. shared bitmap은 checked Vec 수명 안에서 CGContext가 쓰고 context drop 후 읽습니다. CoreFoundation/CoreGraphics retained wrapper 수명을 유지합니다.

## 실제 실패와 정정

1. 첫 PDF test 실행은 compile 9.41초·suite 0.37초이며 2건 실패했습니다. host의 위 blue/아래 red fixture가 반대로 출력되어 실제 좌표 변환 오류를 재현했습니다. PDF.js의 y flip을 Quartz에 그대로 전달한 원인을 수정했습니다. UI fixture는 flat locale JSON을 LocalePack으로 읽어 missing version에서 실패했으며 실제 flat message 구조로 정정했습니다.
2. 다음 실행은 compile 1.64초·suite 0.37초이며 2건 실패했습니다. 수정된 pixel/crop/UserUnit/zoom/snapshot 검사는 앞서 진행됐지만 전체 PASS로 세지 않았습니다. host assertion은 localized Forbidden variant를 plain Forbidden으로 한정한 오류였으므로 `AppError::kind()`로 정확한 Forbidden 종류를 검사합니다. headless surface의 unapplied TexturesDelta는 실제 render 검사 뒤 명시적으로 clear합니다.
3. 정정 뒤 host는 1건 PASS, UI는 1건 실패였습니다(compile 0.83초·suite 0.37초). UI fixture가 이미 표시된 page 1/scale 1로 돌아와서 불필요한 재요청을 기대한 원인이었습니다. 기존 ready texture를 그대로 재사용하고 request가 없는 것을 먼저 검사한 뒤 다른 zoom에 대한 실제 재요청·stale 처리를 검사하도록 정정했습니다. UI만 다시 실행하여 1건 PASS(compile 0.71초·suite 0.07초)입니다. 기대 픽셀·UI 동작·검사기를 완화하지 않았습니다.
4. PDF.js의 CropBox/MediaBox 교차에 대한 독립 geometry 검사에서 기대 2×2가 실제 4×4인 누락을 재현했습니다(compile 0.78초·suite 0.37초). SDK의 box_rect가 원본 box만 반환하는 계약에 따라 표준화·intersection·빈 intersection 시 media fallback을 구현했습니다. 수정 후 해당 검사 1건 PASS(compile 1.75초·suite 0.38초)입니다. 소수 4.2×2.7 canvas의 floor와 실제 위/아래 픽셀 위치도 확인했습니다.
5. 원본 outer raw read의 blank와 inner PDF parse의 loading 차이를 발견하여 SourceReady 진행 상태를 추가했습니다. 변경된 실제 host·UI 검사만 다시 실행했습니다. malformed PDF fixture에서 CoreGraphics의 일반 진단 한 줄이 출력되지만 result는 Decode/InvalidArgument로 거절하며 성공 픽셀로 세지 않습니다.

## 최종 검증 증거

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-pdf -- --skip pdf_viewbox --nocapture` — 최종 loading/host 변경 뒤 2건 PASS(compile 4.04초·suite 0.57초). 실제 승인 worker의 3-page geometry/RGBA·90도 rotation·UserUnit 2·crop·125% zoom·opaque white, 같은 path 두 탭의 독립 선택/unique source accounting, disk 교체 후 snapshot 재사용, invalidate 후 replacement bytes·초기화, 다른 canonical snapshot·outside/relative/closed project 거절, Read/Decode 분류, 늦은 reply·close·tracked_count 0과 ready surface·keyboard·상한·외부 열기 intent·invalid pixel/cache admission입니다. 외부 앱을 실제로 실행하지 않습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-pdf pdf_viewbox -- --nocapture` — geometry 수정 뒤 1건 PASS(0.38초)를 재사용합니다. 후속 SourceReady/host 변경은 decoder geometry를 바꾸지 않았습니다.
- [x] shared bitmap 변경의 영향 검사로 `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-macos 실제_host -- --nocapture`를 승인된 sandbox 밖에서 1회 실행 — 1건 PASS(compile 1.05초·suite 0.45초). 합성 AVIF·linear ICC PNG/WebP/JPEG·sRGB/alpha/EXIF·malformed/limit·worker 회수입니다. 새 PDF 작업의 shared render 추출 뒤의 결과이며 이전 성공을 무조건 반복한 것이 아닙니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview-pdf -- -D warnings` — 최종 진행 상태/host/source 변경 뒤 exit 0(1.24초). 앞선 단계의 compile/check·strict 성공은 같은 코드의 중복 성공으로 합산하지 않습니다.
- [ ] 실제 OS window·GPU texture 적용/표시·양방향 scroll 위치·theme/3 locale/font/DPI·실제 접근성은 이 headless/worker 검사를 완료 증거로 사용하지 않습니다.

## 남은 PDF와 전체 gate

- [ ] CoreGraphics와 PDF.js의 fonts·embedded images·color/transparency·annotation appearance·encrypted/malformed/inherited/default boxes·모든 page/rotation/zoom fidelity를 대조합니다. 현재 visible canvas만 구현하며 실제 OS/GPU·scroll state·annotation render 동등성은 미검증입니다.
- [ ] parser/encoded copies/retained+inflight/graphics upload 전체 memory, 큰 page의 tiling·cancel·crash isolation, encoded 20MiB/64MiB page/128MiB cache 거절 UX와 원본 큰 파일 parity를 판정합니다. native framework decoder 자체를 sandbox isolation 완료로 주장하지 않습니다.
- [ ] macOS 외 PDF decoder, 재시도·renderer limit 변경·다중 창·tab mount/dirty/editor 전환·file watcher 이벤트의 전체 시나리오와 최종 packaging을 완료합니다.
- [ ] 나머지 6 preview provider·editor/terminal·전체 213 view·N1부터 N8, Rust 99%·제품 TS 제거·beta/rollback·서명/공증/install/upgrade를 완료합니다.

macOS PDF의 기본 실제 worker/cache/surface 연결과 위 좁은 검증만 완료입니다. N5-P1d와 전체 M8은 미완료이며 commit/push하지 않았습니다.
