# Native shell mutation의 remote relay 우회

## 대상 파일

native `application.rs`, `bootstrap.rs`, `bootstrap-shell-tests.rs`, 기존 `native/taide-native-ui/src/controller.rs`입니다.

## 리포트

ShellController.connect가 PaintSink를 forward로 보유한 뒤 AppServices.events에만 relay를 추가해 controller 자체의 desktop ShellMutation이 remote event fanout을 우회했습니다. relay28계약 검사는 services 직접 발행만 덮었으므로 단독 성공이 actual App 모든 발행자의 성공 증거가 아니었습니다.

## 상세

원래 실제 App 순서를 connect_shell로 추출한 단일 합성 actor 검사에서 window chrome Zen 변경은 paint와 controller snapshot까지 반영됐지만 remote receiver는 Empty였습니다. 최초 compile8.45초/suite0.01초 RED입니다.

relay→PaintSink를 먼저 구성하고 controller의 forward로 전달한 뒤, 아직 공유되지 않은 services의 events를 같은 NativeShellEvents로 바꿨습니다. 두 발행자가 NativeShellEvents→relay→PaintSink를 한 번 거치며 Notify가 snapshot을 갱신합니다. relay가 NativeShellEvents로 역참조하지 않으며 AppServices를 캡처하지 않아 재귀·strong cycle이 없습니다. 수정 뒤 동일 검사1회 compile5.84초/suite0.02초 PASS·native lib/bin/tests strict13.72초입니다.

실제 GUI/앱은 실행하지 않았고 synthetic state/경로만 사용했습니다. 구체적인 새 wiring의 통과를 assets/Settings/AppFile/IDE 화면/startup/Exit·전체 M8 완료로 확대하지 않습니다. 앞으로 공통 EventSink wrapper 변경에서는 서비스를 직접 발행하는 경로와 실제 controller가 보유한 sink를 함께 대조합니다.
