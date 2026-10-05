# native LSP 미바인딩 raw 진단 폐기

## 대상·관찰

native/taide-native-app/src/lsp.rs·lsp-diagnostics.rs·lsp-diagnostics-tests.rs와 experiments/lsp-coordinator-spike/src/bin/mock-server.rs입니다. 원본 src/shared/lib/lsp/adapters/diagnostics.ts는 normalized URI별 raw를 먼저 저장하고 모델이 있을 때만 marker를 붙입니다. 이전 native는 Session.documents 조회가 실패하면 저장 전에 반환했고 code action은 Document의 중복 diagnostics를 읽었습니다. 이는 코드 대조로 확인했으며 실행 전 RED를 주장하지 않습니다.

## 해결

세션별 raw Store를 단일 출처로 연결했습니다. normalized URI 비교로 wire/model 괄호·쉼표의 인코딩 차이를 처리하고 code/data/source를 그대로 보존합니다. marker는 기존 generation/version/binding gate를 유지합니다. code action은 raw Store를 읽고 clone snapshot은 Arc batch를 공유합니다. URI key는 Monaco component 계약을 따르며 path traversal 해석이나 파일 열기는 하지 않습니다.

문서 didClose/unbind는 모델 폐기가 아니므로 raw를 바로 지우지 않습니다. 같은 세션의 다른 문서가 살아 있으면 raw도 유지합니다. 세션 폐기·빈 batch·명시 URI remove는 회수하고 URI 변경은 이전 key를 지웁니다. 이후 model-disposal·inactive-grace 후속으로 App 폐기 알림과 기본5초 유예/비활성 marker를 연결했으며 최신 결과는 native-problems QA가 정본입니다.

## 검증

Store 신규1 PASS(compile12.86초/suite0.00초), 실제 합성 child 신규1 PASS(compile12.13초/suite0.63초), lifetime 변경 뒤 두 문서 영향 확장1 PASS(compile10.17초/suite0.42초), 저장 code action 영향1 PASS(compile2.19초/suite0.34초)입니다. 같은 성공 상태의 반복 검사는 하지 않았습니다. 실제 child는 unbound 원본·동등 URI bound marker·두 문서 unbind·새 owner·감독 task0을 확인했고 저장 검사는 실제 서버 요청·편집·포맷·파일 저장을 확인했습니다.

환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, --manifest-path native/taide-native-app/Cargo.toml, --locked --offline --target-dir experiments/native-shell-spike/target이며 Cargo는 직렬입니다. 원본 Monaco URI 비교5개 출력은 확인했지만 비교 Bun은 event loop 때문에 Ctrl-C exit130으로 종료했으므로 PASS 명령으로 세지 않습니다. 정적 검사와 현재 전체 미완료 경계는 native-problems QA가 정본입니다.

다중 provider/retarget·전체 diagnostics consumer·누적 raw heap/URI quota·실제 App GUI/OS와 M8은 미완료입니다. 기본 폐기/비활성/grace 구현이 이 범위의 완료 증거는 아닙니다. manifest/lock·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.
