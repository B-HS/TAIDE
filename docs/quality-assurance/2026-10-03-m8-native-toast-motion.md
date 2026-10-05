# M8 native toast CSS motion 기본 연결

## 대상 파일

- `native/taide-native-app/src/{toast.rs,toast-motion.rs}`
- 원본 `node_modules/sonner/dist/index.js`의 Sonner2.0.7 Toast/CSS와 `src/widgets/app-toaster/app-toaster.tsx`, `src/shared/constants/toast.ts`, `src/shared/styles/global.css`
- 이전 정본 `docs/quality-assurance/2026-10-03-m8-native-toast-focus.md`

## 리포트

입장·종료·접힘/확장의 translate/scale/height/opacity 상태와 중간 반전, 닫기 시점의 전환 시작과 삭제 offset 보존을 Rust renderer에 연결했습니다. 전체 animation·원본 브라우저 픽셀 동등성이나 M8 완료는 아닙니다. 위험 검사의 성공은 변경 없는 입력 경로에 재사용했고 전체 suite를 다시 실행하지 않았습니다. 별개 keybindings Tab RED는 그대로 남습니다.

## 상세

1. 원본 CSS의 기본 ease는 [W3C CSS Easing](https://www.w3.org/TR/css-easing-1/#cubic-bezier-easing-functions)의 cubic-bezier(0.25,0.1,0.25,1)입니다. x를 역산한 y를 사용하며 t=0.5의 x=0.3125/y=0.5375로 결정적 대조합니다. 중간 반전은 [W3C CSS Transitions](https://www.w3.org/TR/css-transitions-1/#reversing)의 reversing-adjusted start/shortening을 유지합니다. 같은 target을 매 프레임 재시작하지 않습니다.
2. transform/opacity/height/content400ms, 접힌 non-front 종료 transform500ms/opacity200ms, close opacity100ms입니다. DOM 제거는 기존200ms이며 더 긴 전환이 완료되기 전에 잘리는 원본 수명을 유지합니다. front/expanded 종료의 -lift×100%, 접힌 non-front 종료40%, 종료 전 expanded offset을 보존합니다. height의 auto↔length는 즉시 바꾸고 length↔length만 보간합니다. 활성 height 목록에서는 dismissed 항목을 제외하되 render/semantic 목록에는200ms 동안 남깁니다.
3. 설치된 원본의 접힌 non-front는 `scale(calc(-1 * (index * 0.05 + 1)))`입니다. 이전 native의 양수0.95 축소로 대체하지 않고 음수1.05 등을 유지합니다. middle은 실제 원본처럼 Sonner top 위치를 사용하고 제품 CSS의50% 기준을 유지합니다. source 수식/설치 버전의 재현이며 실제 WebKit computed style·픽셀 대조 완료 주장은 아닙니다.
4. egui의 일반 Shape transform은 음수 scale에서 radius/stroke/텍스트와 Rect min/max를 직접 음수화합니다. 따라서 음수 상태는 공개 tessellate의 triangle mesh vertex를 변환하고, 양수 상태는 공개 PaintList transform을 사용합니다. zero scale은 paint를 생략하고 Rect는 두 변환 끝점으로 정규화합니다. 그림·card/close hit·AX interaction rect를 같은 transform에서 계산합니다. renderer가 소유하는 toast shape 범위에만 적용하며 vendor/private API/새 의존성을 사용하지 않습니다.
5. opacity0 입장 프레임과 전환 중 glyph/shape 표현을 검사 fixture에 반영했습니다. 닫기는 입력을 받은 시점의 상태에서 전환을 시작합니다. 원본 mounted effect의 브라우저 paint/style-flush 시각, 여러 동시 상태 변화와 timer/modal/position 변경의 모든 조합은 전체 App/픽셀 gate에 남습니다.

## 실제 검사

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, app manifest, `--locked --offline --target-dir experiments/native-shell-spike/target`의 serial입니다. 실제 앱·OS·사용자 데이터 대신 합성 headless egui/Instant를 사용했습니다.

- [x] compile E0599(Painter의 없는 with_opacity), fixture E0618(style factory를 local style이 가림)을 기록하고 설치된 공개 multiply_opacity/명확한 factory 이름으로 수정했습니다. 검사 억제는 사용하지 않았습니다.
- [x] 첫 `--lib native_toast`: 5 PASS/4 FAIL(suite0.03초). Duration f32의215.00001ms와215ms 비교, 원본 -lift 방향을 반대로 둔 fixture, elapsed0의 입장 opacity0를 즉시 표시로 기대한 두 renderer fixture입니다. 시간 정밀도 허용과 원본 방향/실제400ms 경과로 수정했습니다.
- [x] 수정 후 같은 filter는 8 PASS/1 FAIL(suite0.03초). 신규 motion2개와 keyboard/AX 및 내용·수명·위치 성공은 재사용합니다. 남은 stack FAIL은 전환된 glyph를 mesh로 출력하는 경로를 기존 Text shape로 기대한 검사였습니다. 양수 변환은 네이티브 Shape로 보존하고 음수만 mesh로 변환하도록 연결했습니다.
- [x] 양수 경로의 첫 stack 재검사에서 context graphics lock 안의 content_rect 조회가10.02초 뒤 deadlock panic으로 실패했습니다. screen을 lock 밖에서 획득해 수정한 `--lib native_toast_stack` 최종1 PASS(suite0.01초)입니다. 독립 원인의 실패를 성공 계측3회로 계산하지 않습니다.
- [x] 신규 `--lib native_toast_motion_render`는 두 geometry FAIL(suite각0.01초) 뒤1 PASS(suite0.01초)입니다. native bounds는 현재 음수 transform이었으나 frame 뒤 read_response는 이전 pass의356px rect였습니다. 같은 시각의 settled frame으로 실제 이전-pass hit 상태를 갱신한 뒤 width373.8px/close21px·뒤집힌 위치·유한 mesh·pointer close/200ms 제거와 최신 항목 보존을 검사했습니다. 예상 폭을 낮추거나 제품 animation을 비활성화하지 않았습니다.
- [x] 양수 renderer 최종 영향 `--lib native_toast는_실제`:1 PASS(suite0.01초). 내용/아이콘/front pointer close·종료 중 제목/description과200ms 제거입니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(14.28초). authored2파일 exact rustfmt/추적 whitespace exit0입니다. inherited Wry17 warnings는 구분하며 이 정적 검사가 keybindings Tab RED를 실행하거나 해결한 것이 아닙니다.

## 남은 gate와 테스트 부채

- [x] swipe/drag 방향·45px/속도 threshold·close hover 색/background/border transition·parent 입력 기본은 후속 `docs/quality-assurance/2026-10-03-m8-native-toast-swipe-hover.md`에 연결·검사했습니다. 전체 Sonner animation 완료는 아닙니다.
- [x] card focus shadow 전환 기본은 `docs/quality-assurance/2026-10-03-m8-native-toast-focus-shadow.md`에 연결·검사했습니다. 전체 focus-visible/close/픽셀 완료는 아닙니다.
- [ ] 전체 focus-visible/shadow·parent position transform·reduced motion 설정을 연결합니다.
- [ ] height reflow/긴 제목·모든9-position 픽셀·CJK/system font/weight·negative↔positive 중간 glyph/clip·동일 시각 push/delete/escape·frame/viewport별 owner·중첩 modal/WebView/auxiliary·전 Settings mutation/hot reload를 원본과 대조합니다. 겹친 card의 hit 우선순위와 같은 프레임 timer/expanded 변경도 남습니다.
- [ ] negative mesh 경로의 실제 GPU/CPU·aggregate retained/peak·최소화/재시작/late·OS IME/VoiceOver는 마지막 성능/실기 gate에서 검사합니다. 현재 fixture는 위 경우의 완료 근거가 아닙니다. extreme queue·긴 glyph·double-to-f32의 subpixel 차이는 그 조건에서 별도 검사하며 arbitrary cap을 도입하지 않았습니다.
- [ ] 전체41 action/Monaco21/palette·213view와 N1~N8/cutover/TS제거·배포/rollback을 완료합니다. 상위 M8은0/8이며 목표 active·전체 완료 뒤만 commit/push합니다.

root/MSRV·제품 TypeScript·보호 실기 bundle·OS 설정을 변경하지 않았습니다. 전역 convention 전문을 사용했으며 프로젝트 `docs/convention`이 없어 skill의 해당 경로 대신 AGENTS가 지정한 원문을 적용했습니다.
