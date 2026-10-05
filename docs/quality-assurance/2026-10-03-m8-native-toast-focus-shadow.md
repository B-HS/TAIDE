# M8 native toast card focus shadow 기본

## 대상 파일

- `native/taide-native-app/src/{toast.rs,toast-motion.rs}`
- Sonner2.0.7 `node_modules/sonner/dist/index.js`의 card box-shadow .2s 및 focus-visible 추가 shadow, swiping transition:none

## 리포트

기존 keyboard focus target에서 card ring이 즉시 켜지고 꺼지던 native 차이를 재현하고200ms ease로 opacity/spread를 전환했습니다. 기본 shadow는 유지하고 swipe 중에는 즉시 적용합니다. 브라우저 focus-visible 판정 전체나 원본 픽셀·M8 완료를 주장하지 않습니다.

## 상세

1. 원본 기본 shadow는0/4/12px black10%이며 focus-visible은0/0/0/2px black20%를 추가합니다. [CSS Backgrounds3 box-shadow](https://www.w3.org/TR/css-backgrounds-3/#propdef-box-shadow)는 짧은 shadow 목록을 transparent0/0/0/0으로 채운 뒤 보간합니다. 따라서 추가 ring의 spread0→2와 alpha0→20%를 기존 ease/reversal 모델로200ms 전환합니다. 중간 반전 모델의 이전 성공은 재사용합니다.
2. native의 현재 keyboard focus target을 유지하며 ring은 card의 공통 transform/opacity 안에서 그립니다. shadow spread와 outer radius를 늘리고 inside stroke로 border-box 내부를 비웁니다. radius/alpha는 egui의 u8 표현으로 반올림합니다. CSS의 실제 blur/curve/GPU/픽셀 동등성은 아직 검증하지 않았습니다.
3. 원본 data-swiping=true의 transition:none에서 기존 전환도 즉시 target으로 이동합니다. swipe-out도 그 상태를 사용합니다. close 버튼의 별도 shadow/focus 스타일은 이번 card 변경 범위에 포함하지 않습니다.
4. [Selectors4 focus-visible](https://www.w3.org/TR/selectors-4/#the-focus-visible-pseudo)는 사용자 설정·키보드/포인터·script focus 이동에 대한 UA heuristic입니다. egui Response.has_focus를 그 전체 구현이라고 주장하지 않으며 현재 keyboard 연결과 visual transition만 완료했습니다. native 전역 input modality·OS preference·원본 WebKit 대조는 남습니다.

## 실제 검사

동일 app manifest/locked/offline/공유 target의 serial Cargo이며 합성 headless egui/Instant만 사용했습니다.

- [x] 신규 `--lib native_toast_shadow`의 renderer baseline1 FAIL(suite0.02초, compile3.42초). Alt+T→Tab으로 실제 card focus ID를 먼저 assert한 뒤 첫 순간에도 ring이 이미 불투명하게 표시되는 차이를 재현했습니다. parent의 앞선 focus fixture 실패와 다릅니다.
- [x] 모델·renderer 연결 뒤 같은 filter2 PASS(suite0.02초, compile3.70초). 실제 keyboard focus의 처음/100ms/200ms·blur의 처음/100ms/200ms/미삭제를 검사했습니다. 순수 모델 검사는 진행 중 target의 swipe transition:none과 일반 전환 복귀를 덮습니다. 실제 OS swipe 중 focus-visible 완료는 아닙니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(10.45초). authored2파일 exact rustfmt exit0입니다. 이전 parent4·swipe/close hover/motion/Settings/catalog/search/layout/AX 성공을 반복하지 않았습니다. inherited Wry17 warnings와 별개 keybindings Tab RED는 그대로 구분합니다.

## 남은 gate

- [ ] 전체 focus-visible heuristic·close focus·pointer/AX/system preference·hidden/modal/같은 frame/late/aux/WebView·reduced motion을 연결합니다.
- [ ] box-shadow의 모든 radius/blur/clip/color/subpixel·negative scale/font/GPU/OS 픽셀과 성능을 비교합니다. 좁은 renderer stroke 검사를 이 전체 동등성으로 계산하지 않습니다.
- [ ] 전체213view/41action/Monaco21/palette·N1~N8/cutover/TS제거·성능/보안/배포/rollback은 미완료입니다. M8은0/8·목표active·전체 완료 뒤만 commit/push합니다.

root/MSRV·제품TS·보호 실기 bundle·사용자 데이터·OS 설정과 vendor를 변경하지 않았습니다. process/verify/save-docs skill은 관련 체크리스트·위험 검사·성공 재사용·실제 결과 기록에 적용했습니다.
