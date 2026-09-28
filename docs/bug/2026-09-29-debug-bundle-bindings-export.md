# 패키징한 debug 앱의 바인딩 출력 경로 오류

## 대상 파일

- `src-tauri/src/lib.rs`의 `BINDINGS_PATH`와 `run()` 초기 바인딩 내보내기
- `src-tauri/src/lib.rs::typescript_바인딩을_생성한다`

## 리포트

macOS debug `.app`은 Launch Services로 시작하면 앱 창을 표시하기 전에 `src-tauri/src/lib.rs:806`에서 종료됐습니다. 실제 오류는 `../src/shared/api/bindings.ts`의 상위 디렉터리를 읽기 전용 위치에 만들 수 없다는 것이었습니다. 같은 실행 파일을 `src-tauri` 작업 디렉터리에서 직접 실행하면 창이 열렸으므로 바인딩 생성의 상대 경로가 시작 작업 디렉터리에 의존한 것이 원인입니다.

## 상세

`#[cfg(debug_assertions)]`는 패키징한 debug 빌드에도 적용되지만, 바인딩 파일 출력은 소스 트리가 작업 디렉터리로 잡히는 개발 실행에서만 필요합니다. 설치된 Tauri 2.11.5의 [공식 `tauri::is_dev()` 구현](https://docs.rs/tauri/latest/src/tauri/lib.rs.html#317-319)은 `custom-protocol` feature의 반대로 정의되어 있어 개발 실행과 패키징 실행을 구분합니다. 바인딩 내보내기를 debug 컴파일 조건 안에서 `tauri::is_dev()`로 한 번 더 제한했습니다. 공개 IPC 타입이나 생성 파일 내용은 변경하지 않았습니다.

기존 `typescript_바인딩을_생성한다` 대상 1건, `cargo fmt --all -- --check`, `git diff --check`가 exit 0이고 생성 bindings diff는 없습니다. 수정한 debug 번들은 `CARGO_NET_OFFLINE=true bun run tauri build --debug --bundles app --no-sign`으로 exit 0이었습니다. 로컬 산출물에만 ad hoc 서명 후 `codesign --verify --deep --strict`가 exit 0이고, 전용 identifier의 `.app`을 `src-tauri`가 아닌 작업 디렉터리에서 Launch Services로 열자 실제 TAIDE 창과 이전 임시 프로젝트·파일이 나타났습니다. `⌘Q` 뒤 IDE listener 39729와 격리 `39729.lock`이 사라졌습니다. 이 실행에서 상대 바인딩 경로의 panic은 재발하지 않았습니다.

배포용 release 번들은 이 코드 변경 전에도 `CARGO_NET_OFFLINE=true bun run tauri build --bundles app --no-sign`으로 exit 0이었고, 전용 identifier의 창과 기본 `⌘Q` 종료를 확인했습니다. release 결과는 debug 오류 수정의 검증이나 성능 실측으로 대신하지 않습니다.
