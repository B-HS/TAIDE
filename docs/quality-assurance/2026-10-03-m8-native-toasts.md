# M8 native 토스트·Settings 실패 출처

## 대상 파일

- `native/taide-native-app/src/{toast,application,host,presentation,lib}.rs`
- `native/taide-native-app/resources/toasts/{warning,error}.svg`, `LICENSE-SONNER`
- 원본 읽기: `src/widgets/app-toaster/app-toaster.tsx`, `src/shared/constants/toast.ts`, `src/shared/styles/global.css`, `src/entities/settings/settings.query.ts`, `src/shared/lib/{ipc-error-message,unwrap-result}.ts`, installed `node_modules/sonner/dist/index.js`

## 리포트

키맵 캡처 warning과 키맵/글자 크기 Settings 저장 실패를 앱 native 토스트로 연결했습니다. 최신 entry가 앞에 오며 richColors·원본 SVG·문구/description·위치·남은 시간·닫기를 보존합니다. 기본 연결·합성 host·headless 위험 검사이며 Sonner 전체 동작이나 full App/aux/실기·픽셀/AX 완료가 아닙니다. 상위 M8 N1~N8은 0/8이고 전체 완료 전 commit/push하지 않습니다.

## 상세

1. `HostReply::SettingsFailed(AppError)`로 Settings 실패 출처를 보존합니다. `SetKeymapOverrides`/`SetEditorFontSize`의 async 디스크 오류와 App submit의 즉시 queue/disconnect 오류는 동일한 `settings.saveFailed` 제목과 원본 description을 사용합니다. 다른 host 실패의 기존 status 경로는 변경하지 않았습니다. 상태/디스크/이벤트 성공 경로와 실패 시 이전 값 보존도 유지합니다. 모든 Settings 화면의 mutation 연결 완료는 아닙니다.
2. 원본 IPC description은 Rust `Display` 접두사가 아니라 각 오류 variant의 raw 문자열입니다. localized catalog key가 있으면 args를 번역하고 없으면 fallback을 사용합니다. 로그·HTML 실행·외부 SVG URL을 추가하지 않습니다. SVG는 컴파일 시 포함한 정적 데이터이며 두 href resolver를 비활성화했습니다.
3. Sonner 2.0.7/MIT의 warning/error SVG와 상수를 옮겼습니다. installed CJS SHA256은 `02d27b81196b5d6bf04cbff9ae8c8a4fca0cc9e3026d56ac73c9a8c8f24620e2`이며 MIT notice를 보존했습니다. rich palette는 원본 HSL을 sRGB 8-bit로 변환한 값입니다. 아이콘은 20px이며 원본 16px wrapper·-3/4px margin·SVG -1px margin·body gap6, 카드 padding16/border1/radius8·0/4/12 shadow를 사용합니다. 원본 weight500/system font와 전체 GPU raster는 아직 검증 완료가 아닙니다.
4. 수명4000ms·제거200ms·visible3·width356·gap14·viewport24/mobile16·600px 경계를 사용합니다. 숨겨진 네 번째 이후 entry도 타이머가 진행합니다. hover/interaction/최소화 중 남은 시간이 보존되며 확장 카드 사이 원본 pseudo-element gap도 hover 영역에 포함합니다. 닫힘 중 내용은 유지하고 opacity·repaint로 기본 종료를 표시합니다. 원본400ms ease/translate/height·입장/스와이프 동등성은 남습니다.
5. 9개 위치/invalid fallback/추가 segment 무시와 현재 `Settings.toast_position`을 연결했습니다. middle은 원본 CSS가 top50%/translateY(-50%)로 override한 0-height toaster 경계입니다. middle-center의 horizontal translateX도 override되므로 source 계산대로 left50%를 사용합니다. 이 source-level geometry는 실제 browser 픽셀 대조 완료가 아닙니다. native 최소화는 document.hidden의 일부 대응이며 모든 OS occlusion/가시성 전이를 완료로 계산하지 않습니다.

## 실제 검사

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, app manifest, 기존 `experiments/native-shell-spike/target`, `--locked --offline`로 serial 실행했습니다.

- [x] baseline `--lib keybinding_host`: generic Failed가 반환되어 SettingsFailed 기대 assertion RED(exit101, suite0.06초). typed reply 수정 후 관련1 PASS(suite0.03초). 실제 합성 디스크·전체 Settings equality·이벤트·실패 이전 값·disconnect/task0를 포함합니다.
- [x] `--lib native_toast`: 컴파일 E0282는 palette 변환 closure의 타입 추론 문제였으며 array map으로 수정했습니다. 첫 실행은 raw/localized/newest ID·시간/hidden entry·9-position 3 PASS(suite0.02초), renderer fixture1 FAIL입니다. 실패 원인은 headless TexturesDelta 미처리 Drop assertion이며 기존 검사 방식처럼 clear했습니다. 제품 렌더러의 검사를 끄지 않았습니다.
- [x] 수정된 `--lib native_toast는_실제`:1 PASS(suite0.02초). 실제 title/description·SVG texture shape·close pointer down/up·닫힘 중 내용·200ms 제거입니다. 이전3개 성공은 반복하지 않았습니다.
- [x] 후속 `--lib native_toast_stack`:1 PASS(suite0.02초). 5 entry/3 visible·collapsed content1/expanded3·실제20px texture 위치·gap hover·minimized/PointerGone pause·resume·숨긴 entry까지 만료입니다. 이전 renderer 성공은 내용/닫기 경로의 동일 부분에 재사용합니다.
- [x] 변경된 failure reply 영향 `--lib font_keymap`:1 PASS(suite0.04초). 원본 font 범위·실제 host/Settings/디스크/오류·editor 표시/Core 보존입니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(11.32초). inherited Wry17 warnings와 authored strict를 구분합니다. exact authored rustfmt/추적 whitespace는 exit0입니다. 새 의존성·root/MSRV·제품 TS·보호 bundle·OS 설정은 변경하지 않았습니다.

사용 API는 installed egui0.36.2의 InputState/ViewportInfo/Ui/Area/TexturesDelta 소스와 [공식 Ui 문서](https://docs.rs/egui/0.36.0/egui/struct.Ui.html)로 확인했습니다.

## 남은 gate

keyboard/접근성 목록 후속 기본은 `docs/quality-assurance/2026-10-03-m8-native-toast-focus.md`가 정본입니다. 아래 focus/AX 잔여는 모든 modal/first-frame/다중 창/실기·전체 동등성을 뜻하며 해당 기본 완료를 취소하지 않습니다.

- [ ] 원본 Alt+T/Escape/focus 반환·toast/close focus ring·live polite/Notifications 접근성·모든 offscreen/disabled/modal/같은frame/multi-pass를 연결합니다.
- [ ] 입장·종료400ms ease/transform/height·hover/close transition·스와이프·정확한 font/weight/wrapping·narrow viewport·pixel/AX/OS를 재현합니다. 기본 fade를 전체 animation 완료로 계산하지 않습니다.
- [ ] Settings theme/locale hot reload·전 mutation·연속 저장/late event·다중 창 원본 Observer 소유·native WebView layer·full App/aux/aggregate를 연결합니다. 현재 toast queue는 App 인스턴스 소유이고 theme는 초기 resolved type입니다.
- [ ] 일반 focus trap·나머지8 action/Monaco21/palette/AppFile/Save/rawUnicode·213view·N1~N8/cutover/TS제거·성능/보안/package/rollback을 완료합니다. 기존 성공은 재사용하며 동일 입력 검사를 반복하지 않습니다.

## 테스트 부채

실제 사용자 앱/OS 입력기·VoiceOver/clipboard/browser는 실행하지 않았습니다. full App/aux와 최종 통합 시 위 gate를 검사합니다. 극단적 u64 identity 고갈·초장문 toast의 aggregate 메모리/CPU는 현재 국소 timer/UI 검사 범위를 벗어나며 전송·수명/성능 보안 gate에서 판정합니다. 임의 count/byte 제한으로 원본 동작을 축소하지 않았습니다.
