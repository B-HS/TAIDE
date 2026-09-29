# M7 격리 IDE lockfile 일회성 예외

2026-09-29 사용자는 실행 중인 격리 TAIDE 앱의 IDE `openFile` 실측을 직접 수행하도록 요청했고, 해당 앱의 임시 IDE lockfile 한 개에서 인증 토큰을 메모리로만 읽는 예외를 명시적으로 승인했습니다. 이는 `AGENTS.md`의 키 파일 읽기 금지에 대한 이 검사 한정 예외이며 사용자 홈의 다른 lockfile·키 파일, 토큰 출력·문서 기록·커밋을 승인하지 않습니다.

대상은 `/private/tmp/taide-m6-isolated.byLYaMWs/claude-release/ide/63063.lock` 한 개, localhost listener PID 44412, 합성 프로젝트의 `small.txt` 한 파일입니다. 원격 접근 링크 생성은 별도 보안 결정으로 남으며 이 예외에 포함하지 않습니다. 실행 결과는 [IDE 단일 실측](../quality-assurance/2026-09-29-m7-ide-ws-authenticated-open-file.md)에 기록합니다.
