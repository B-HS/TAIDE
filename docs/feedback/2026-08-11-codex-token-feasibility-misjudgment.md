# Codex access token auto-tab "기술적 불가" 오판 (2026-08-11)

## 무엇을 지적받았나

리서치(opus)가 "Codex access token 은 인가 범위가 CLI/app-server 한정이라 completion 호출 경로가
없다 — auto-tab 부적합" 으로 판정했고, 메인이 이를 재검증 없이 사용자에게 두 번 단언했다.
사용자 반박: "뭘 자꾸 기술적으로 막혀있다는거지? b-hub 에서도 이미 구현해서 쓰고있는데."

## 왜 틀렸나

- 리서치가 **공식 문서에 없는 경로 = 존재하지 않는 경로**로 치환했다. 실제로는 Codex CLI 가 쓰는
  `https://chatgpt.com/backend-api/codex/responses`(Responses 형태 + SSE)가 실존하며, access token
  (`at-` 접두) + `chatgpt-account-id`(whoami 로 획득) + `originator: codex_cli_rs` 헤더로 호출된다.
- 결정적으로 **사용자의 기존 프로젝트에 동작하는 반례가 있었는데 확인하지 않았다**
  (`~/development/b-hub/service/domain/ai/providers/codex-provider.ts`).
- TAIDE 자체가 이미 같은 성격의 결정 전례(Claude Code IDE 연동 = 비공식 리버스 엔지니어링 프로토콜,
  "깨질 수 있음을 인지하고 진행")를 갖고 있었으므로, "비공식 = 불가"가 아니라 "비공식 = 리스크
  고지 후 사용자 선택"으로 다뤘어야 했다.

## 어떻게 고치나

- 외부 API "가능/불가" 판정은 공식 문서 부재만으로 불가 처리하지 않는다. ① 사용자의 기존
  프로젝트(~/development/*)에 선행 구현이 있는지 먼저 찾고, ② 해당 CLI/클라이언트가 실제로 쓰는
  엔드포인트(리버스 엔지니어링 결과 포함)를 확인한 뒤, ③ 약관·파손 리스크를 명시해 사용자가
  결정하게 한다.
- 사용자가 "이미 쓰고 있다"고 말하면 그 코드가 단일 출처다 — 즉시 열어 실측한다.

## 언제 적용하나

- 서드파티 API·프로토콜의 실현 가능성을 판정할 때. 특히 에이전트 리서치가 "불가" 결론을 낸 경우,
  메인이 사용자 반례·기존 자산을 검색한 뒤에만 사용자에게 단언한다.
