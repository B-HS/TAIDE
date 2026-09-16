# CLI 탐색용으로 `taide <단어>` 를 실행하면 실행 중 앱에 "파일 열기" 요청이 간다 (2026-09-16)

## 무엇을 지적받았나

사용자: "또 간헐적으로 이러한 에러 토스트가 뜨니 확인해 — `File not found: /Users/gkn/taide/help` — claude에서
뭔가 트리거하고있나?"

## 왜 틀렸나

메인이 `taide` CLI 의 사용법을 알아보려고 `taide --help`·`taide help` 를 실행했다. `crates/taide-cli` 는 도움말
플래그가 없고 **모든 인자를 cwd 기준 파일 경로로 정규화**(`normalize_absolute`)해 실행 중 TAIDE 에 열기 요청을
보낸다. 앱은 없는 파일이라 `error.file.notFound` 토스트를 냈다. 사용자 화면에 조사 부작용이 그대로 노출됐다.

## 어떻게 고치나

- CLI 동작은 소스(`crates/taide-cli/src/main.rs`)를 읽어 파악한다. 실행이 필요하면 인자 없이(`taide` → usage 출력 후
  exit 1) 만 실행한다.
- 실행 중 사용자 앱에 요청을 보내는 명령(CLI 파일 열기, IDE 서버 툴 호출 중 diff/저장/탭 닫기 등)은 조사 목적으로
  쓰지 않는다. 읽기 전용 툴(`getDiagnostics`·`getOpenEditors`)만 허용.

## 언제 적용하나

사용자가 TAIDE 를 실행 중인 세션에서 CLI·IDE 서버·원격 제어를 탐색할 때. 앱이 꺼져 있으면 `taide <file>` 은 앱을
새로 띄우므로 역시 금지.
