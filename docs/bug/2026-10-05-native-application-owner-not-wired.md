# 실제 NativeApplication의 서버 owner 미배선

## 대상과 관찰

대상은 `native/taide-native-app/src/application.rs`, `application-ports.rs`, `remote-assets.rs`입니다. 기존 ApplicationPorts factory와 Settings reconcile은 있었지만 실제 NativeApplication은 owner/start를 호출하지 않고 legacy HostBridge constructor를 사용했습니다. Settings AppFile 저장의 필수 callback도 없어 쓰기 전에 Forbidden이었습니다. factory 단독 검사를 실제 App 완료로 확대할 수 없었습니다.

## 해결

실제 constructor가 bootstrap의 같은 GitEvents·서비스·Tabs Hub, 현재 Views palette/context, 실제 terminal environment와 live scrollback 설정을 사용해 ApplicationPorts를 보관합니다. HostBridge 생성/재연결은 이 owner의 reconcile을 받습니다. 모든 fallible App 초기화 뒤 IDE→hooks→remote startup을 await합니다. 정상/오류 direct Exit는 ApplicationPorts와 공유한 remote→hooks→IDE stop 함수를 사용하고, App 폐기 시 owner를 회수합니다.

공개 자산은 executable의 번들/인접 `remote-public` 디렉터리에서 명시적 bundle manifest로 로딩하며 기존 TS dist나 빈 resolver로 대체하지 않습니다. metadata/공개 파일 모두 동일 descriptor Anchor·Stamp, 경로/크기/형식 검사를 거칩니다. actual Rust UI 자산은 아직 생성되지 않았으므로 실제 배포물의 원격 시작은 유효한 자산을 갖출 때까지 실패합니다. 이를 원격 UI 완료로 표시하지 않습니다.

## 검증

actual constructor 검사는 toolkit의 CreationContext/Frame kittest 경계와 실제 App::logic을 사용했습니다. 합성 bundle/project/settings·loopback에서 시작·Settings 파일 저장에 따른 서버 정지·canonical/dirty·정상 Exit·owner/Hub 해제/task0이 고유1 PASS(compile10.22초/suite0.41초)입니다. 기존 direct Exit 정상/오류 검사는 고유1 PASS이며 application-ports QA 맨 위가 상세 정본입니다.

처음에는 sandbox의 EPERM으로 loopback listen이 실패했습니다. 권한 승격 뒤 첫 검사 driver가 내부 poll만 호출해 종료 완료 응답을 처리하지 못했고, 단계 진단에서 shutdown=true·closing=true·tasks=0을 확인했습니다. driver를 실제 App::logic로 정정했습니다. 제품 종료 동작을 바꾸거나 timeout을 늘리지 않았습니다. 실제 창/GPU/CJK/VoiceOver·enabled IDE/hooks 홈 통합·브라우저 UI·beta/cutover는 미완료입니다.
