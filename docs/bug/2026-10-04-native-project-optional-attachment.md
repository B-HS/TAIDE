# Native project의 선택 자원 실패가 열기를 거절한 차이

## 대상·원인

`native/taide-native-app/src/projects.rs`의 guard-free attachment 준비가 File/Git watcher와 IDE lockfile 오류를 `?`로 전파했습니다. 원본 `src-tauri/src/domain/file/capability.rs`, `git/watch.rs`, `ide/commands.rs`는 해당 오류를 경고로 처리하고 가능한 자원을 등록합니다. native 경로만 프로젝트 열기/attach를 거절하는 차이입니다.

## 재현·수정

열기 상태 commit과 실제 watcher 준비 사이에 합성 root가 사라진 조건을 최소 재현했습니다. native attachment가 `error.watcher.registerFailed`로 실패한 RED(compile6.04초/suite0.01초)입니다. watcher 준비 실패는 None으로 처리하고, lockfile 갱신 실패도 후속 hooks/commit을 막지 않도록 원본 정책을 복원했습니다. layout/state live 확인·mutation guard·감독 작업·shutdown admission·root guard는 유지합니다.

수정 후 고유1 PASS(compile8.19초/suite0.12초)입니다. root 소실 경로의 layout commit과 실제 open에서 IDE directory가 파일이라 lockfile 쓰기에 실패한 경로의 project/layout/watcher commit·guard 해제·watcher/task 종료를 확인했습니다. 경고는 사용자 경로/원문 오류 대신 AppErrorKind만 기록합니다. root가 없어도 사전 canonical/open 검증을 우회하도록 변경한 것이 아닙니다.

source capability registry의 serialized kind는 Git/Terminal뿐입니다. LSP 등 임의 capability를 추가해 동등성을 바꾸지 않았습니다. 실제 App startup/OS UI와 `.git` 소실 경합은 별도 gate입니다.
