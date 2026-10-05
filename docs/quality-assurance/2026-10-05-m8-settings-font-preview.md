# M8 Settings 글꼴 픽셀·목록 정렬·검색 입력

## 대상 파일

- `native/taide-native-ui/src/font-preview.rs`, `settings-code-view.rs`, `settings-view.rs`, `lib.rs`
- `native/taide-native-app/src/font-preview.rs`, `application.rs`, `lib.rs`, `LICENSE-LUCIDE`
- `native/taide-remote-web/src/font-preview.rs`, `browser-editor.rs`, `lib.rs`, `Cargo.toml`, `tests/settings-resources.rs`
- `tools/m8-remote-rust-file-probe.ts`

## 리포트

원본 FontPicker는 선택 값과 각 글꼴 행에 실제 family를 지정합니다. shared UI의 `Previews`에 플랫폼별 Painter를 연결했습니다. native는 기존 resvg/usvg·system_fonts DB와 TaskSupervisor를 사용해 글꼴 픽셀을 background worker에서 만들고 UI 프레임에서 texture를 받습니다. egui의 전체 FontDefinitions나 터미널 글꼴을 덮어쓰지 않습니다. 준비 중에는 기존 egui 텍스트를 표시합니다. 실행 환경을 우회하는 별도 runtime/스레드·새 의존성·폰트 파일 전송 API는 추가하지 않았습니다.

브라우저는 현재 브라우저가 실제로 사용할 수 있는 CSS family를 Rust/web-sys Canvas2D에서 래스터화합니다. 호스트에만 있는 family가 브라우저에 없으면 원본 CSS처럼 브라우저 fallback을 사용합니다. family는 CSS 인용·이스케이프하고, native SVG의 내부 생성 문자열은 XML 이스케이프·외부/embedded 이미지 resolver 거절을 적용합니다. 새 웹 권한이나 OS 설정 변경은 없습니다. Canvas font·픽셀 경계의 공식 근거는 [MDN font](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/font), [MDN getImageData](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/getImageData)와 설치된 web-sys 0.3.106 generated API입니다. resvg/usvg 0.48.1의 설치된 원본 example/options/tree와 기존 preview_svg.rs도 확인했습니다.

공유 RasterKey는 family/text/실제 pixel size/DPI를 구분하고 유한 양수 geometry·최대 변 8192·8MiB RGBA 예산을 검사합니다. 각 플랫폼의 LRU texture 최대 256개·8MiB이며 native는 pending 예약도 예산에 포함하고 동시에 최대 4개만 실행합니다. 이 값은 texture/pending 픽셀 예산이지 fontdb·SVG shaping·GPU/CPU 순간 복사까지 포함한 전체 프로세스 메모리 상한이 아닙니다. native 실패 key는 자동 재시도하지 않습니다. clear/Drop은 pending receiver와 texture를 회수하고 이전 수명 세대는 새 응답/재그리기를 만들지 않습니다. 전체 Settings가 해제되면 해당 cache를 비웁니다.

browser Painter는 소유한 FontFaceSet loadingdone listener로 늦게 준비된 폰트의 cache를 무효화하고 다시 그립니다. clear/Drop에서 listener와 canvas를 함께 회수합니다. 기존 설정/목록 RPC·인가·서버 정책은 불변입니다.

원본 코드와 실제 캡처를 기준으로 글꼴/문자열/옵션 라벨의 왼쪽 정렬, 목록 행의 왼쪽 텍스트/오른쪽 check·선택 ring, 검색창의 투명 frame/테마 색/검색 아이콘/구분선/36px·좌우 12px, CommandGroup 4px·행 28px/4px radius를 반영했습니다. `add_sized`는 자식 배치를 중앙 정렬하므로 Label.halign만으로 해결되지 않았고 명시적인 왼쪽 배치로 수정했습니다. Search 아이콘은 설치된 Lucide 1.28.0의 실제 circle/path가 근거이며 기존 ISC/MIT 전문에 Search attribution을 추가했습니다. 전체 theme/DPI/원본 CSS 픽셀 일치 완료를 주장하지 않습니다.

검색 입력은 trigger 기반 안정 ID를 사용하고 실제 표시 pass에서만 자동 포커스를 소비합니다. 크기 계산 pass는 포커스 요청을 소모하지 않으며 표시 후 다음 repaint를 요청합니다. 원본의 popup mount 후 입력 포커스를 보존하려는 수정입니다.

## 검증

- [x] 신규 native 픽셀·회수 검사 2건 PASS: 22.02초 빌드/.02초 실행. builtin Hack/Ubuntu만 합성 DB에 넣어 실제 RGBA 차이/불투명 픽셀·없는 family fallback·XML 주입 격리·Unicode 경계 truncate·무한 geometry 거절과 이전 세대 응답 회수를 확인했습니다. 최초 `egui` 테스트 import 누락은 수정한 compile 실패입니다. 실제 OS 글꼴을 스캔한 검사가 아닙니다.
- [x] 별도 신규 native 실제 Painter admission 1건 PASS: 10.11초/.02초. 같은 요청 중복0·pending 최대4/예산 유지·clear/감독자 종료·종료 뒤 단일 오류와 자동 재시도0을 확인했습니다. current-thread runtime을 진행시키기 전에 취소하므로 실제 OS DB 읽기는 없습니다. 앞선 성공 2건은 반복하지 않았습니다.
- [x] 변경된 실제 Views 입력 영향 1건 PASS: 새 backend/행 변경 뒤 2.58초/.15초, 검색창 스타일 변경 뒤 해당 검사만 1.17초/.15초. 원본 ToC/검색/키 선택을 확인하며 같은 소스 성공의 반복이 아닙니다. `TextEdit.frame`이 pinned egui 0.36.2에서 bool이 아니라 Frame인 compile 실패는 실제 builder API로 수정했습니다.
- [x] 신규 실제 Chrome 글꼴 시나리오 `settings-font-visual` GREEN: seq12/write1/font1/shell1·실제 두 family 목록→builtin TTF 늦게 로딩→동일 행 픽셀 변경→선택 trigger→Ready/socket0·loadingdone을 종료 뒤 보내도 상태/요청/래스터 개수 불변·quiet1.1초·page/panic/실패0입니다. Mono hash 3707262640→610171027, Sans 696245328→665071351, 선택 trigger의 별도 폭 hash2819531923이며 기록된 5개 raster 모두 opaque>0입니다. 명시 클릭한 검색 입력을 사용한 검사이며 자동 포커스 검사를 대신하지 않습니다.
- [x] 스타일 읽기 및 브라우저 자동 포커스 수정 영향 `settings-code-style` GREEN: seq11/write0·실제 검색 input 자동 focus·새 스타일 PNG·Drop/socket0/page/panic/실패0입니다. 초기 자동 focus 대기는 실패했고, 중간 capture-only 결과의 focused=[]를 자동 포커스 PASS로 쓰지 않습니다. 안정 ID/실제 표시 pass/repaint 수정 뒤 같은 실패 경계만 확인한 최종 결과의 focused=[font-search]가 근거입니다. 글꼴 로딩·저장 성공을 다시 실행하지 않았습니다.
- [x] 공용 UI 자동 포커스 진단: 빈 목록(.09초)→실제 두 font 목록(.11초)→브라우저처럼 press/release를 다른 프레임으로 보낸 입력(.11초)은 모두 PASS였습니다. 이를 브라우저 자동 포커스 성공으로 대체하지 않았으며, 최종 플랫폼 수정은 위 Chrome 결과로 검증했습니다. 초기 Reply::fonts 추측 compile 실패는 실제 Reply::Fonts variant로 수정했습니다. 이 세 fixture 확장은 기존 입력 검사와 달리 새 브라우저 실패의 플랫폼 분리 진단이지만 중복 범위가 컸으므로 재실행하지 않습니다.
- [x] native lib/bins/tests check8.99초 exit0·Wasm probe 최종 의미 변경 build1.92초 exit0·시험 TS strict/Rust exactfmt/Prettier. native check는 검색 스타일까지의 소스이고 그 뒤 안정 ID/표시 pass 수정의 같은 portable UI는 최종 Wasm build가 컴파일했습니다. 기존 Wry17 경고·테스트 linker unwind 경고를 새 제품 경고0과 구분합니다. 기존 성공을 반복하지 않았습니다.
- [x] optional canvas를 끈 normal production Wasm check5.08초 exit0입니다. glyph의 새 web-sys feature가 optional eframe 활성화에 기대지 않는 별도 컴파일 경계입니다. canvas 소비가 없는 구성의 피드백 오류 payload3·show_feedback 메서드2에 dead_code 경고5가 있으며 검사 억제나 payload 삭제로 숨기지 않습니다. glyph 모듈 경고는 없습니다.
- [ ] 전체 popup/disabled/AX 관계·원본 theme/DPI/다중 viewport·font 목록 대규모 성능·native 실제 시스템 폰트 full GUI·원본 전체 시각/접근성 일치·전체 App/assets/패키징 notice·최종 gate는 남습니다.

## 첫 Chrome 실패와 수정 구분

처음 글꼴 시나리오의 공통 `settings.fontFamilyMonospaceOnly` 진단 key는 Editor/Terminal에서 중복돼 마지막 Terminal 좌표를 가리켰습니다. 원자료의 popup에 Mono만 있고 Terminal 필터 좌표 y1722.921875가 남은 사실을 확인했습니다. 실제 widget ID는 각 field scope로 구분되어 있었으므로 제품 상태 버그로 주장하지 않습니다. inspection key만 editor-monospace-filter/terminal-monospace-filter로 구분한 뒤 실패 시나리오 한 건을 수정 확인했습니다. 이 실패 캡처에서 발견한 중앙 라벨 배치는 별도의 실제 제품 수정입니다.

스타일의 초기 브라우저 자동 포커스 실패는 Native 같은 동작의 PASS와 분리했습니다. 단순히 명시 클릭을 넣어 통과 처리하지 않았습니다. 실제 표시 pass·안정 input ID·repaint의 수정 뒤 자동 focus가 관찰됐습니다. 크기 계산 pass와 브라우저 DOM의 세부 기여를 각각 분리 계측한 것은 아니므로 한 원인만 단독으로 증명했다고 주장하지 않습니다.

## 명령·원자료

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --lib font_preview::tests -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --lib font_preview::tests::실제_painter는_중복과_동시_작업을_제한하며_종료_후_자동_재시도하지_않는다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources 실제_두_설정섹션은_독립목록_응답수명과_원본_picker_검색_입력을_소비한다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo build --manifest-path native/taide-remote-web/tests/browser-probe/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-font-visual
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-code-style
```

원자료는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-font-visual-result.json`, `settings-font-visual-failure.json`, `settings-code-style-result.json`입니다. 최신 bindings는 검색 자동 포커스 수정 소스입니다. 글꼴 시나리오는 검색 스타일/자동 포커스 추가 전 소스로 재사용하며 두 입력 경계를 구분합니다. 캡처는 `assets/2026-10-05-m8-settings-font-visual.png`, `assets/2026-10-05-m8-settings-code-style.png`입니다. 스타일은 popup 표시 전이 시점의 단일 캡처로 전체 안정 픽셀 비교가 아닙니다. Chrome/server는 합성 localhost·mock Keychain·service workers blocked/download false로 격리하고 종료 후 회수했습니다.

글꼴·정렬·검색 입력 변경은 연결/최소 검증 완료이나 Editor·Terminal의 원본 전체 재현 체크는 계속 pending입니다. 세부3/4(75%)·provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 산정 보류·goal active·main 직접·전체 M8 완료 전 Git 없음입니다. 기존 폰트 목록 캐시/입력 성공을 반복하거나 하위 작업을 부모 완료로 계산하지 않습니다. 실제 사용자 프로젝트/키체인/보호 bundle/OS 설정/제품 TS/의존성 버전/lock/MSRV를 보존했습니다.
