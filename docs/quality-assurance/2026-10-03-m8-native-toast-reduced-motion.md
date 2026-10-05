# M8 native toast reduced-motion 기본

## 대상 파일

- `native/taide-native-app/src/{toast.rs,toast-motion.rs,toast-swipe.rs,motion-preference.rs,application.rs,lib.rs}`
- `native/taide-native-app/{Cargo.toml,Cargo.lock}`
- 원본 Sonner2.0.7 `node_modules/sonner/dist/index.js`의 prefers-reduced-motion CSS·swipe-out·200ms 삭제

## 리포트

원본 reduced-motion의 transition/animation:none을 native card·height·child·close hover·focus shadow와 swipe-out에 연결했습니다. macOS 공개 getter와 변경 notification으로 현재 창의 repaint를 요청하고 observer를 Drop에서 해제합니다. 합성 renderer·private notification center 결과이며 실제 OS 설정 변경·브라우저 픽셀·다른 플랫폼이나 M8 전체 완료를 주장하지 않습니다.

## 상세

1. 원본 media rule은 toast와 직접 자식의 transition/animation을 끕니다. 정상 타이머4000ms와 제거 지연200ms는 JavaScript 경로이므로 유지합니다. 기존 ease·중간 반전 모델의 duration을0으로 retarget해 진행 중 전환도 즉시 target에 도달합니다. 다시 일반 motion으로 돌아갈 때 이전 entry animation을 재생하지 않습니다.
2. swipe-out은 animation:none일 때 translate100%·opacity0 keyframe을 적용하지 않습니다. 정상 exit selector도 data-swipe-out=false만 대상으로 하므로 마지막 swipe 이동량과 opacity1을200ms 제거까지 유지합니다. 종료 임계값·parent pause·swipe 취소 정책은 변경하지 않았습니다.
3. 첫 headless renderer에서 모델 opacity1이지만 텍스트가 없었습니다. 설치된 egui0.36.2 Area는 첫 sizing pass를 invisible로 생성합니다. reduced-motion에서만 공개 Ui.is_sizing_pass와 Context.request_discard로 같은 run의 bounded layout pass를 요청합니다. 정상 animation 경로는 유지하며 vendor/private state/검사 억제를 사용하지 않습니다. 실제 React mounted/useEffect·WebKit 첫 픽셀 시각은 별도입니다.
4. [NSWorkspace 공개 getter](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayshouldreducemotion?language=objc)는 읽기 전용입니다. 창은 workspace notification center의 accessibility display notification을 받아 egui repaint만 요청하고 매 frame getter를 다시 읽습니다. 별도 cached boolean으로 최신 값이 역전되는 경로를 만들지 않습니다. [objc2 AppKit 계약](https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSWorkspace.html#method.accessibilityDisplayShouldReduceMotion)과 설치된 generated source의 feature/API를 확인했습니다.
5. observer는 retained center·정확한 returned token을 소유합니다. notification 객체를 읽지 않고 Send+Sync+'static callback만 호출합니다. queue/object=None인 Foundation API 경계를 사용하며 typed AsRef<AnyObject>로 token을 해제합니다. 두 unsafe notification 호출과 static 읽기는 해당 객체·수명 계약에 한정합니다. [RcBlock 수명](https://docs.rs/block2/latest/block2/struct.RcBlock.html)과 설치된 Foundation add/remove 계약을 확인했습니다. raw pointer cast나 suppression은 없습니다.
6. Rust 표준 라이브러리에는 AppKit observer API가 없어 이미 잠긴 block2 0.6.2·objc2 0.6.4·AppKit/Foundation0.3.2를 직접 참조합니다. 공식 crate 문서에서 현재 버전도 확인했습니다. 기존 native lock snapshot과 diff는 app dependency array의4줄 추가뿐이며 새 package/version은 없습니다. feature는 NSWorkspace/NSAccessibility/NSNotification/NSOperation/NSObject/NSString/block2로 한정합니다. root lock·MSRV·제품TS는 변경하지 않았습니다. nonmac getter는 None이며 Windows/Linux 연동을 완료로 계산하지 않습니다.

## 실제 검사

기존 manifest의 locked/offline·공유 target·serial Cargo와 합성 Instant/egui를 사용했습니다. 의존성 직접 참조 반영 시에만 한 번 offline lock resolve를 했고 이후 --locked를 유지했습니다.

- [x] `--lib native_toast_reduced_motion` 정책 baseline1 FAIL(suite0.00초, compile3.61초):100ms opacity0.4085106과 기대1의 차이를 재현했습니다. 전환 정책 수정 뒤 model/swipe2 PASS(suite0.00초, compile3.54초)입니다. 즉시 height/negative scale/content/hover/shadow·일반 motion 복귀와 swipe 이동량 보존을 검사했습니다.
- [x] observer 연결 compile E0308과 E0282 각1회:removeObserver의 &AnyObject 계약 및 중첩 AsRef 추론을 설치된 public trait 구현으로 확인한 뒤 명시적 typed AsRef로 수정했습니다. 이 compile 오류를 실행 테스트 실패나 통과로 계산하지 않습니다.
- [x] `--lib native_toast_reduced_motion_render`:first Area sizing으로1 FAIL(suite0.02초, compile7.57초), reduced-motion 전용 discard 뒤1 PASS(suite0.02초, compile2.42초). 첫 text·card bottom·실제 Alt+T/Tab 즉시 ring·raw pointer swipe·100ms 이동량50px·200ms 제거를 검사했습니다.
- [x] `--lib native_motion_preference`:1 PASS(suite0.28초, build0.22초). private center에서 unrelated notification 무시·두 observer·각 Drop 뒤 잔여 호출 수를 검사했습니다. workspace/default center에 합성 notification을 게시하지 않았습니다. App getter·배선은 컴파일 확인이며 실제 OS 변경 검사는 아닙니다.
- [x] authored6파일 exact rustfmt --check exit0입니다. 이미 성공한 pure policy·renderer·observer 및 Settings/catalog/search/layout/AX/motion/parent/focus 검사는 반복하지 않았습니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(23.63초)입니다. 의존 feature 변경으로 관련 native crate를 다시 검사했으며 inherited Wry17 warnings는 authored strict와 분리합니다. full suite 실행이나 별개 Tab RED 통과를 뜻하지 않습니다.

## 남은 gate

- [ ] Windows/Linux reduced-motion 공개 설정 조회·변경 notification·수명·검증을 연결합니다. [portal Settings](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Settings.html)의 reduced-motion 키가 존재하지만 아직 구현하지 않았습니다.
- [ ] 실제 macOS preference 변경·sleep/wake·다중 창/full App/aux·notification 경쟁·browser mounted·OS/GPU/font/subpixel 픽셀과 모든 modality를 검증합니다. 사용자 OS 설정은 유지했고 보호 실기 bundle은 실행·변경하지 않았습니다.
- [ ] selection owner/OS capture/touch·parent position·hot theme/locale/모든Settings·same-frame/late/aux/WebView와 전체 focus-visible/box-shadow gate는 미완료입니다.
- [ ] 별개 keybindings Tab RED 및 PTY remount A/B 응답 대기를 해소한 근거가 아닙니다. 전체213view/41action/Monaco21/palette·N1~N8은0/8이며 cutover/TS제거·성능/보안/배포/rollback은 남습니다.

process/verify/save-docs skill은 체크리스트·위험 검사·성공 재사용·분류 기록에 적용했습니다. 목표는 active이며 전체 M8 완료 뒤에만 commit/push합니다.
