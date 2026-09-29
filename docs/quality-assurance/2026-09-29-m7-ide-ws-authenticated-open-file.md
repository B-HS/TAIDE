# M7 격리 IDE WebSocket 인증·openFile 단일 실측

## 대상과 방법

전용 계측용 release 앱 PID 44412가 `127.0.0.1:63063`에서 LISTEN 중인 것을 확인했습니다. [일회성 예외](../acknowledge/2026-09-29-m7-isolated-ide-lockfile-exception.md)에 따라 정확한 임시 lockfile 한 개만 메모리에서 읽고, 토큰을 출력·저장하지 않은 채 `ws` 클라이언트의 `X-Claude-Code-Ide-Authorization` 헤더와 `mcp` subprotocol로 localhost에 연결했습니다. 프로젝트 안의 합성 `/private/tmp/taide-m7-fixture.csNotl/project/small.txt`만 요청 대상으로 사용했습니다.

첫 probe는 제품 연결 전에 lockfile JSON 필드를 `snake_case`로 검사해 중단됐습니다. 실제 모델의 `camelCase` 키로 바로잡은 다음 한 세션에서 아래 요청을 순서대로 실행했습니다. 실패한 사전 검사를 인증 실패나 제품 장애로 분류하지 않습니다.

## 결과

| 경계 | 관찰 |
| --- | --- |
| 인증 WebSocket handshake | 연결 성공, 협상된 subprotocol `mcp` |
| `initialize` | JSON-RPC `result` 응답 |
| `tools/list` | `openFile` 항목 존재 |
| `tools/call` `openFile` | 합성 파일 요청의 `result` 응답, `error` 없음 |
| `tools/call` `getOpenEditors` | 응답에 동일 합성 파일 경로 포함 |
| 실제 앱 화면·수명 | `small.txt` 파일 탭·본문을 접근성 트리에서 확인하고 탭을 닫아 Settings만 남음 |
| 합성 파일 보존 | 종료 후 1,024바이트, `s` 외 바이트 0개 |

클라이언트 WebSocket은 요청 후 닫았습니다. IDE 토큰 값·인증 헤더 내용·lockfile 내용은 출력하거나 문서에 남기지 않았습니다. 이 검사는 실제 앱의 유효 토큰 인증, 대표 MCP tool handler, 파일 탭 열림·닫힘을 입증하지만 모든 IDE 도구와 장시간 다중 연결 동작을 대표하지는 않습니다. 앞선 [무인증 401 실측](2026-09-29-m7-ide-ws-live-denial.md)과 합쳐 인증 경계의 양쪽을 확인했습니다.
