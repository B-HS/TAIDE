# M7 격리 debug 앱 기능 부분 실기

## 범위

전용 identifier의 기존 debug 번들을 격리된 `ZDOTDIR`·`CLAUDE_CONFIG_DIR`·`TMPDIR`로 실행했습니다. `/private/tmp/taide-m6-isolated.byLYaMWs/project`에 만든 비시크릿 파일만 사용했습니다. 사용자 프로젝트와 IDE lockfile의 내용은 읽지 않았습니다. 이 검사는 release 성능·전체 TS 화면·전체 Phase 0 기능 gate를 대신하지 않습니다.

## 관찰

- 새로 만든 `m7-preview.html`을 Explorer에서 열자 `about:srcdoc`의 `M7 HTML preview` 제목과 본문이 화면·접근성 트리에 나타났습니다.
- `m7-preview.svg`는 색상 배경과 `M7 SVG preview` 문구가 이미지로 렌더링됐고, 접근성 트리에도 문구가 나타났습니다.
- `m7-preview.csv`는 `Sheet1` 탭의 표로 열렸고 `name,quantity`, `alpha,3`, `beta,5`가 화면·접근성 셀에 일치했습니다.
- 기존 Terminal 탭에서 격리 프로젝트의 셸 프롬프트에 `echo M7_TERMINAL_OK`를 붙여넣고 실행하자 같은 문자열이 출력되고 프롬프트가 돌아왔습니다. 처음 `typeText` 입력은 따옴표가 잘못 전달되어 `Ctrl+C`로 취소했고, 제품 입력 오류로 판정하지 않았습니다.
- `m7-diagnostics.ts`의 `number = 'invalid'`를 열자 vtsls가 연결 상태 `1/1 LSP`와 오류 1건을 표시했습니다. Problems 패널의 메시지는 `Type 'string' is not assignable to type 'number'.`와 `ts 1:7`이었습니다. 앱에서 값을 `7`로 고쳐 저장하자 `No problems have been detected`와 오류 0건으로 바뀌었고, 디스크 내용도 수정된 원문과 일치했습니다.
- `⌘Q` 뒤 앱 프로세스는 exit 0이었고 로그에 `ExitRequested → drain 완료 → ExitRequested → Exit → 직접 drain 완료`가 남았습니다. 이 실행의 IDE 포트 `12258` listener와 격리 lockfile `12258.lock`, vtsls 자식 PID `21419`는 종료 뒤 없었습니다. 기존 M6 직접 Exit 단독 검사를 대체하지 않습니다.

## 판정과 남은 범위

HTML·SVG·CSV의 표시, 셸 명령 왕복, TypeScript 진단 표시·해소의 좁은 debug 앱 경로는 통과했습니다. HTML sandbox 실행 차단, 다른 미디어·문서 형식, terminal 재부착·flood·CJK, LSP completion/hover/rename·crash/restart, 편집기 IME·undo, 다중 창·테마/로케일·접근성 전수, release 성능·활성 원격 WebSocket 동시 종료는 검증하지 않았습니다. 따라서 M7-C4와 Phase 0 전체 gate는 미완료입니다.
