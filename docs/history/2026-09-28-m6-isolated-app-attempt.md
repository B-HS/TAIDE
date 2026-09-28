# M6 격리 앱 기동 첫 시도

## 대상

- `src-tauri/src/lib.rs`의 setup·IDE 서버·Exit 배선
- `src-tauri/tauri.conf.json`, `src-tauri/tauri.dev.conf.json`, `scripts/tauri.ts`
- `crates/taide-ide/src/lockfile.rs`, `src-tauri/src/domain/ide/commands.rs`

## 실제 결과

사용자 A 선택에 따라 전용 `net.gumyo.taide.m6test.*` identifier로 `node_modules/.bin/tauri dev`를 실행했습니다. 사전 확인에서 해당 identifier의 앱 데이터·로그 경로는 없고 개발 서버 포트 5173은 비어 있었습니다. 설치된 Tauri CLI 2.11.4의 도움말과 [공식 CLI 문서](https://v2.tauri.app/reference/cli/)는 여러 `--config`가 순서대로 병합돼 마지막 충돌 값이 우선한다고 명시합니다. 기존 개발 설정 뒤에 고유 identifier를 전달했으며 `cargo run`은 약 8분 뒤 빌드 완료·앱 기동까지 진행했습니다.

기동 로그에서 키링 서비스명은 고유 identifier였고 복원 프로젝트 0개·watcher 0개였습니다. 반면 IDE 서버는 포트 45059를 열고 사용자 홈 `.claude/ide`에 lockfile을 썼습니다. `lockfile_dir()`은 identifier가 아닌 `CLAUDE_CONFIG_DIR` 또는 사용자 홈을 사용합니다. 이 부작용을 확인하자 더 이상 GUI 조작·직접 Exit 검사를 하지 않고 개발 세션에 `Ctrl+C`를 보냈습니다. 이후 앱 실행 파일 점유와 두 포트 45059·5173의 리스너는 없었지만 `.claude/ide/45059.lock`은 남았습니다. 토큰이 들어갈 수 있어 파일 내용은 읽지 않았고 삭제도 하지 않았습니다. 전용 identifier의 앱 데이터 경로는 생성되지 않았고 전용 로그 경로만 생겼습니다.

## 판정과 다음 조건

이번 결과는 앱 기동의 관찰값이지 M6의 GUI·직접 Exit 합격 증거가 아닙니다. 앱 디버그 실행은 computer-use 앱 목록에서 식별되지 않아 창을 직접 조작하지도 못했습니다. 정확한 lockfile 하나의 정리 방법에 대한 사용자 답을 기다립니다. 다음 시도는 새 임시 경로를 `CLAUDE_CONFIG_DIR`에 주입해 IDE lockfile·stale cleanup을 사용자 홈에서 분리하고, GUI 접근 수단과 종료 시 비강제 cleanup 경계를 먼저 확인해야 합니다. 기존 사용자 설정·키링·lockfile 내용은 조회하지 않습니다.
