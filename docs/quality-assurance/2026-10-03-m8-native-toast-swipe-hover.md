# M8 native toast swipe·닫기 hover·parent 입력 기본

## 대상 파일

- `native/taide-native-app/src/{toast.rs,toast-motion.rs,toast-swipe.rs}`
- 원본 Sonner2.0.7 `node_modules/sonner/dist/index.js`의 Toast pointer handler, Toaster mouse/pointer handler 및 CSS
- 이전 정본 `docs/quality-assurance/2026-10-03-m8-native-toast-motion.md`

## 리포트

원본 축 잠금·방향 감쇠·45px/0.11px/ms 종료·400ms 복귀·200ms swipe-out, 두 theme의 close hover 색 전환과 parent expanded/interacting 수명을 native에 연결했습니다. 원본 전체 toast·브라우저 픽셀·OS 입력 동등성 또는 M8 완료는 아닙니다. 기존 성공은 재사용하며 별개 keybindings Tab RED와 PTY remount 계약 응답 대기를 유지합니다.

## 상세

1. 첫 이동의 절댓값이1px를 넘으면 큰 축을 잠그고 동률은Y입니다. 원본 React handler의 기존 state처럼 그 잠금 이벤트에서는 이동량0을 유지합니다. 다음 이벤트부터 위치 방향은 그대로, 반대 방향은 `delta / (1.5 + abs(delta) / 20)`으로 감쇠합니다. center의 수평 이동은0이며 middle은 제품의 실제 Sonner top 매핑을 사용합니다. 선택 문자열 guard는 순수 모델에만 있으며 renderer는 아직 전역 선택 상태를 전달하지 않습니다.
2. 전체 정수 millisecond의 속도가0.11을 넘거나 이동량이45px 이상이면 종료합니다. 원본의0/0 NaN은 종료하지 않고 양수/0 Infinity는 종료합니다. swipe-out은 [CSS Easing의 ease-out](https://www.w3.org/TR/css-easing-1/#cubic-bezier-easing-functions),200ms, 폭/높이100%와 opacity1→0입니다. swipe 중 parent transform/height/opacity는 transition:none이며 child opacity는 별도입니다. 취소 시 현재 transform과 이동량에서400ms 복귀를 시작합니다.
3. native raw 입력은 프레임당 한 번 처리하고 이전 화면의 최신 card부터 실제 affine hit rect로 owner를 고릅니다. close와 우클릭은 swipe를 시작하지 않습니다. 포인터가 영역 밖으로 나가도 owner를 유지하고 동일 button 해제에서 완료합니다. modal/창 focus 취소는 native의 보호 정책이며 원본 DOM pointer-capture와 모든 다중 버튼·touch·event timestamp 동등성은 아직 검증하지 않았습니다.
4. close hover는200ms ease로 background/border를 전환합니다. light의 gray2/gray5, dark의 normal-bg-hover/normal-border-hover를 사용합니다. 원본 legacy hsl의 보간은 [CSS Color4의 legacy sRGB](https://www.w3.org/TR/css-color-4/#interpolation-space)를 따릅니다. 실제 출력은 u8 반올림이므로 브라우저의 subpixel/색 공간·픽셀 일치를 주장하지 않습니다. egui Response.hovered가 PointerGone 프레임에 남아 있어 실제 hover_pos가 close rect 안에 있는지도 확인합니다.
5. parent expanded는 실제 enter/move에서만 켜고 leave는 interacting=false일 때만 접습니다. pointer-down의 interacting은 expanded와 별도이며 pointer-up 자체는 expanded를 접지 않습니다. Escape 뒤 정지 포인터의 idle/discard pass가 다시 펼치지 않도록 raw event frame을 한 번만 처리합니다. timer pause는 hidden 또는 expanded 또는 interacting입니다. [egui Event](https://docs.rs/egui/0.36.2/egui/enum.Event.html) 및 [PointerState.hover_pos](https://docs.rs/egui/0.36.2/egui/struct.PointerState.html#method.hover_pos)의 공개 계약과 설치 source를 확인했으며 vendor/private API를 변경하지 않았습니다.

## 실제 검사

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, app manifest, `--locked --offline --target-dir experiments/native-shell-spike/target`의 serial입니다. 합성 headless egui/Instant만 사용했습니다.

- [x] 첫 compile E0599(Vec2에 없는 to_array)를 설치된 공개 x/y 필드로 수정했습니다. `--lib native_toast_swipe`:2 PASS(suite0.01초, compile6.67초). 축/감쇠/선택 모델·threshold/속도/NaN/Infinity·center·방향 exit와 raw capture/20px 복귀/50px 종료를 검사했습니다.
- [x] close hover 첫 검사1 FAIL(suite0.01초)은 PointerGone 뒤 egui가 보존한 interaction 위치 때문입니다. 실제 hover_pos 확인 뒤 `--lib native_toast_hover`:1 PASS(suite0.02초, compile3.42초). 두 theme의 초기/중간/200ms 최종 색과 영역 밖 복귀·미삭제를 확인했습니다.
- [x] motion/swipe 공통 변경 영향 `--lib native_toast_motion`:3 PASS(suite0.01초), card Sense 변경 영향 `--lib native_toast_focus`:1 PASS(suite0.01초), 당시 strict exit0(14.50초). 이후 parent 입력 변경 전의 성공이며 현재 전체 suite 성공으로 계산하지 않습니다.
- [x] parent 검사 최초1 FAIL(suite0.01초, compile4.03초), event 상태 변경 뒤1 FAIL(suite0.02초, compile3.84초)은 모두 Escape 전 focus_within이 아직 반영되지 않은 fixture였습니다. 첫 실패를 정지 포인터 제품 재현이라고 보고했던 설명을 정정합니다. 원본 대비 제품 상태 차이는 source 분석으로 확인했고 이 두 실패 자체로 입증했다고 주장하지 않습니다. 공개 focus 요청을 idle frame에 반영하고 실제 focus_within을 assert한 뒤 `--lib native_toast_hover_parent`:1 PASS(suite0.02초, compile1.75초). stationary Escape·move 재확장·drag leave/해제 유지·재진입 뒤 leave/resume·미삭제를 검사했습니다.
- [x] parent 변경 영향만 `--lib native_toast_stack`:1 PASS(suite0.01초), `--lib native_toast_focus`:1 PASS(suite0.01초), `--lib native_toast_swipe_render`:1 PASS(suite0.01초)입니다. 같은 상태의 성공을 세 번 계측하지 않았습니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(12.53초). authored3파일 exact rustfmt exit0입니다. inherited Wry17 warnings와 별개 Tab RED를 구분하며 전체 suite를 실행하지 않았습니다.

## 남은 gate와 테스트 부채

- [ ] renderer의 실제 toast/전역 text selection·touch/cancel/여러 pointer button·원본 DragEnd·batched event/render timestamp·같은 frame timer/키보드·late push/delete/index>=3 이동을 연결하고 해당 위험 검사만 수행합니다. 순수 모델의 selection guard를 실제 화면 완료로 계산하지 않습니다.
- [x] card focus shadow transition 기본은 후속 `docs/quality-assurance/2026-10-03-m8-native-toast-focus-shadow.md`에 기록했습니다. 위 parent strict12.53초 뒤 추가 product 변경의 최종 strict10.45초이며 현재 전체 suite 성공은 아닙니다.
- [ ] 전체 focus-visible/shadow·reduced motion·parent position·전체 font/픽셀/모든9-position·OS GPU/CPU와 aggregate retained/peak는 미완료입니다. 극단 queue/긴 glyph/f32 차이는 그 조건과 성능 gate에서 검사합니다.
- [ ] 모든 modal/auxiliary/WebView·hot theme/locale·전 Settings mutation·실제 IME/VoiceOver·전체41 action/Monaco21/palette/213view와 N1~N8/cutover/TS제거·배포/rollback을 완료합니다. 상위 M8은0/8이며 목표 active·전체 완료 뒤만 commit/push합니다.

root/MSRV·제품 TypeScript·보호 실기 bundle·OS 설정을 변경하지 않았습니다. 프로젝트 `docs/convention`이 없어 AGENTS가 지정한 전역 전문을 process/verify/save-docs skill의 대체 경로로 적용했습니다. Rust fn/enum·상태 mutation과 검증된 숫자 cast는 기존 native 언어 계약을 유지하며 TS 컨벤션의 문법을 Rust로 강제하지 않습니다.
