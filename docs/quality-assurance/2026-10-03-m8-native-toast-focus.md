# M8 토스트 키보드·접근성 목록 후속

## 대상 파일

- `native/taide-native-app/src/{toast,application}.rs`
- 원본: Sonner2.0.7 CJS의 Toaster/Toast, installed Radix Dialog/FocusScope/DismissableLayer와 `src/shared/ui/dialog.tsx`
- 이전 정본: `docs/quality-assurance/2026-10-03-m8-native-toasts.md`

## 리포트

원본의 알림 목록·물리 Alt+T·Escape 접기·Tab 진입과 focus 반환·접근성 이름/description·polite live를 연결했습니다. 표시3개 밖의 알림도 원본처럼 타이머·focus·AX 항목을 보존하되 pointer hit에서는 제외합니다. 같은 입력 frame의 keyboard 부작용은 한 번 처리합니다. 기본 소유/입력/AX tree 검사이며 실제 VoiceOver/전체 제품 modal/모든 브라우저 픽셀/원본 animation·M8 완료는 아닙니다. 별개의 키맵 Tab clipping RED는 그대로 남습니다.

## 상세

1. 여러 개의 독립 Area를 단일 알림 Area로 합쳤습니다. 배경은 뒤에서 앞으로 그리며 실제 card/close registration과 AX 항목은 최신 entry부터 순서대로 구성합니다. 원본 `Notifications altKey+T`·`Close toast`는 원본 영어 기본 label이며 임의 번역 문자열로 바꾸지 않았습니다. 목록은 일반 Tab 후보에 넣지 않고 공개 `Ui::interact`/`Memory::request_focus`로 직접 focus합니다. 상태 ID는 공식 `Context::check_for_id_clash`로 수명 검사에 등록합니다. 비공개 API나 vendor 변경은 사용하지 않습니다.
2. physical T+Alt를 사용하고 Ctrl/Shift 등 추가 modifier를 금지하지 않습니다. 원본처럼 raw hotkey를 소비하지 않습니다. Escape는 목록/하위 focus를 유지하고 확장만 접으며 dismiss하지 않습니다. 목록에서 Tab은 최신 card, 다음 Tab은 close로 들어가고 ShiftTab/blur/목록 제거는 이전 focus로 돌아갑니다. card/close의 원본 black20%/2px focus ring을 연결했습니다. timer pause는 hover에만 묶지 않고 keyboard-expanded/interacting/최소화에도 적용합니다.
3. `cumulative_frame_nr`로 raw keyboard 처리의 단일 frame 소유를 고정했습니다. egui가 같은 frame을 discard/replay해도 Alt+T/Tab을 중복 실행하지 않습니다. 원본 이벤트가 렌더 pass마다 새 브라우저 이벤트로 발생한다고 가정하지 않습니다. 모든 App command/다중 창의 exactly-once 완료가 아니라 이 목록 keyboard 경로의 근거입니다.
4. AX named List·polite/non-atomic live와 ListItem의 명시적 이름/description·close Button을 연결했습니다. Label의 value는 ListItem 이름으로 바꾸고 value를 제거해 중복 값 노출을 막습니다. 모든 entry의 semantic 순서를 보존하며 화면 밖 entry는 빈 clip으로 pointer만 차단합니다. AX Focus/Click도 원본처럼 별도 입력 경로입니다. live relevant additions/text·플랫폼별 announce timing/verbosity와 실제 VoiceOver는 아직 확인하지 않았습니다.
5. 원본 Radix modal은 FocusScope trap·outside pointer 차단·hideOthers를 사용합니다. native의 키맵/닫기/삭제/종료 상태 flag와 직전 egui modal layer로 toast focus/pointer/AX gate를 연결했습니다. inert 상태의 색을 dim하지 않고 AX subtree만 숨깁니다. headless false gate가 확인됐으며 모든 first-frame/nested/full App/aux/OS modality 완료가 아닙니다.

## 실제 검사

Cargo는 app manifest·기존 target·`--locked --offline`·`CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`의 serial입니다. 입력·UI·AX는 합성 fixture입니다.

- [x] renderer/lifecycle 변경 영향 `--lib native_toast`: 기존5개 위험 검사 PASS(suite0.02초). 단일 Area/화면 밖 항목·내용·close·gap/minimize·기본 수명입니다. Settings host/disk의 변경 없는 성공은 재사용했습니다.
- [x] 최초 compile의 E0624(`create_widget` private)·E0277(description의 `&String`)를 실패로 기록합니다. 공개 interaction과 state ID registration·`as_str`로 수정했습니다. private API 접근을 검사 억제로 우회하지 않았습니다.
- [x] 신규 `--lib native_toast_focus`: hover-only 목록 ID가 used ID에 없어서 2-pass 후 focus None인 RED(suite0.01초). 공개 state ID registration 뒤 이 오류는 사라졌으나, 검사기가 second pass 뒤 원본 raw event가 남는다고 가정해 별도 fixture FAIL(suite0.01초)입니다. first pass 안에서 미소비 hotkey를 확인하도록 수정한 최종1 PASS(suite0.01초). 물리 T/논리 Y·추가 modifier·raw 보존·pause/Escape/Tab/card ring/Enter close·이전 focus/ShiftTab·강제2-pass를 포함합니다.
- [x] 신규 `--lib native_toast_accessibility`: ListItem 이름 None인 RED(suite0.01초). explicit name 연결 후1 PASS(suite0.02초), 원래 Label value를 제거한 변경 영향의 최종1 PASS(suite0.02초). 5 entry semantic 순서/이름/description·List live/non-atomic·AX Focus/화면 밖 close Click·false modal gate의 focus 보존/AX hidden입니다. label/value 변경 전 PASS를 최종 metadata의 증거로 대체하지 않습니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(12.47초). authored exact rustfmt와 tracked whitespace는 exit0입니다. inherited Wry17 warnings는 authored strict와 구분합니다. 이 정적 검사는 남아 있는 keybindings Tab RED를 실행하거나 해결한 것이 아닙니다.

설치된 egui0.36.2 Context/Memory/UiBuilder/Sense/Response와 AccessKit source의 실제 public signature·parent map·role/value/focus/tree update를 확인했습니다. 새 의존성·root/MSRV·제품 TS·보호 bundle·OS 설정은 변경하지 않았습니다.

## 남은 gate

기본 translate/scale/height/opacity·ease/반전 후속은 `docs/quality-assurance/2026-10-03-m8-native-toast-motion.md`가 최신 정본입니다. 아래 전체 animation/실기 gate는 계속 미완료이며 이전200ms 선형 표시 상태를 현재 구현으로 승계하지 않습니다.

- [ ] 원본 입장/종료400ms easing·translate/height·hover/close transition·swipe·font/CJK/weight·실제 픽셀을 재현합니다. 현재 opacity200ms 기본 표시를 원본 animation 완료로 계산하지 않습니다.
- [ ] 같은frame 새 entry/삭제·pointer↔keyboard·모든 Tab 순환/blur/사라진 origin·nested/첫 modal frame·다중 viewport/late/shutdown·native WebView layer를 full App에 연결합니다. 키맵 offscreen Tab RED는 별도 진단 질문 응답 대기이며 이 toast 성공으로 해소했다고 쓰지 않습니다.
- [ ] theme/locale hot reload·전 Settings mutation·global Observer/aux queue 소유·실제 VoiceOver와 모든 IME/OS 가시성·성능/보안 aggregate를 확인합니다. timer count 변경과 정지 pointer에 따른 원본 React effect/onMouseMove timing도 전체 동등성 gate에 남습니다.
- [ ] 전체41 action/Monaco21/palette·AppFile/Save/rawUnicode·213view와 N1~N8/cutover/TS제거·배포/rollback을 완료합니다. 목표 active·상위0/8이며 전체 완료 후만 commit/push합니다.

## 테스트 부채

실제 시스템 입력기/VoiceOver/사용자 앱/clipboard/browser/보호 bundle은 실행하지 않았습니다. source-level AX tree와 native public API 호출 검사는 사용자 담당 실기를 대체하지 않습니다. extreme queue/초장문·u64 identity·전역 다중 창 origin 회수는 full App/aggregate/최종 실기 gate에서 검사하며 원본에 없는 cap을 임의 도입하지 않았습니다.
