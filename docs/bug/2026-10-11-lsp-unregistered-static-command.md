# 정적으로 등록하지 않은 LSP 명령 요청 허용

## 대상 파일

- crates/taide-lsp/src/native.rs의 LspCoordinator::request와 request_id_tests

## 원인과 수정

workspace/executeCommand는 정적 executeCommandProvider 객체가 있으면 commands에 없는 ID도 서버로 보냈습니다. Code Action의 클라이언트 명령을 미지원으로 구분하지 못하는 문제이기도 합니다. 정적 명령은 실제 commands 목록의 ID만 허용하며 기존 동적 등록·해제 경로는 유지합니다. 새 의존성이나 문서별 구문 엔진을 추가하지 않았습니다.

## 검증

/private/tmp/taide-batch23-code-actions-command-red-actual.log는 정적 미등록 명령이 UnsupportedCapability 대신 요청 ID로 반환되는 0통과/1실패·exit 101입니다. command-red.log는 없는 native feature의 CLI 오류·0검사이므로 재현 근거가 아닙니다.

수정 뒤 /private/tmp/taide-batch23-code-actions-command-sdk-all.log에서 cargo test --manifest-path crates/taide-lsp/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --all-targets --no-fail-fast를 1회 실행해 전체 90통과/0실패/0ignored·exit 0·검사 0.22초를 확인했습니다. 새 검사는 정적 등록 ID/미등록 ID·동적 등록/해제와 해제 뒤 정적 명령 보존을 함께 확인합니다. SDK의 동일 성공은 재실행하지 않습니다. 관련 실제 App 적용/클라이언트 참조 명령의 검증은 배치 23 QA에 기록합니다.
