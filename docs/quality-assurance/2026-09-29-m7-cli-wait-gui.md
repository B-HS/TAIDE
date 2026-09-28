# M7 CLI `--wait` 실앱 수명 검사

## 대상 파일

- `crates/taide-cli/src/main.rs`의 파일 전달·marker 생성·대기
- `src/app/providers/agent-external-open-provider.tsx`의 외부 파일 열기·marker 등록
- `src/entities/layout/tab-path-change.ts`의 마지막 파일 탭 닫기·marker 해제
- `crates/taide-runtime/src/agent_actions.rs`의 marker 경로 검증·삭제

## 리포트

전용 identifier의 debug 앱에 임시 프로젝트만 열고, 빌드된 `taide-cli-aarch64-apple-darwin --wait --timeout 180`으로 임시 텍스트 파일을 전달했습니다. 첫 실행은 CLI에만 전용 `TMPDIR`을 지정하고 이미 실행 중인 앱에는 지정하지 않아 marker가 해제되지 않았습니다. CLI는 180초 뒤 exit 1을 반환하며 자신이 만든 marker를 제거했습니다. 앱의 `agent_release_marker`는 앱의 `std::env::temp_dir()` 안에 있는 marker만 허용하므로 이 결과는 앱·CLI 환경이 다른 fixture 오류입니다. 제품 코드 결함이나 정상 환경 실패로 분류하지 않습니다.

두 번째 실행은 앱과 CLI 모두 `/private/tmp/taide-m6-isolated.byLYaMWs`를 `TMPDIR`로 지정했습니다. CLI가 만든 UUID marker의 존재를 확인했고, 접근성 트리에서 `cli-wait-2.txt` 탭의 내용과 `Save and close this tab to hand the text back to Claude Code` 안내를 확인했습니다. 탭 닫기 뒤 CLI exit 0, marker 부재를 확인했습니다. 이어 앱을 `⌘Q`로 종료해 exit 0을 확인했습니다. marker 내용·사용자 설정·시크릿은 읽지 않았습니다.

실제 CLI 인수 전달과 탭 닫기 수명은 이 격리 fixture에서 통과했습니다. IDE WebSocket 인증·tool handler 요청/응답과 다른 창·dirty tab·파일 이동 경계는 별도 미검증입니다.
