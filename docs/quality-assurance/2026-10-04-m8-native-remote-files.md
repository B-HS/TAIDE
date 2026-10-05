# M8 원격 file/tree 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-files.rs`, `remote-files-tests.rs`, `lib.rs`입니다. 원본 파일 JSON14명령·트리 JSON5명령과 유일 raw `file_read_raw`를 실제 runtime에 연결했습니다. preferences19+IDE7+file/tree20은46/177명령의 실제 domain adapter이며 남은131명령과 생산용 앱 assembly는 미완료입니다. 전체 M8 N1~N8은0/8입니다.

## 구현 경계

- `remote_files::extend_backend(remaining)`은 선언한 JSON19와 raw1만 처리합니다. 나머지는 필수 remaining backend를 유지하며 전체 assembly 바깥의 `with_policy`가 default-deny·owner·JSON/raw 모드를 먼저 검사합니다. fixture remaining은 제품 handler가 아니며 미구현 성공으로 등록하지 않았습니다.
- file_open은 원본과 같은 root guard 뒤 plugin read-through cache/실제 language overlay를 사용합니다. 저장·복사·dirty mirror는 기존 TaskSupervisor/worker/owned mutation, 생성·rename·삭제·untitled는 각 원본 runtime 정책을 그대로 사용합니다. 원본 raw의 project/CLI path guard와 byte 응답도 동일합니다.
- mirror/untitled read/write/clear/prune는 기존 프로젝트·component gate와 persistence를 사용합니다. 원본에 없는 event·새 quota·worker/취소 정책을 임의 추가하지 않았습니다. file_delete는 기존 macOS `NsFileManager` 휴지통 구현이며 직접 영구삭제로 우회하지 않았습니다.
- 트리5명령은 실제 TreeStore·기존 prefetch/mutation/page·offset/optional limit을 사용합니다. Tauri의 TreeToggle/TreeReveal perf span도 유지했습니다. remote shared TreeStore 정책을 바꾸거나 새 UI 설계를 추가하지 않았습니다.

## 검증

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, native manifest `native/taide-native-app/Cargo.toml`, `--locked --offline`, target `experiments/native-shell-spike/target`입니다. 모든 파일은 고유 임시 합성 프로젝트/data 아래 생성했습니다. 사용자 데이터·앱·키체인·OS 설정은 사용하지 않았습니다.

- [x] `cargo test --lib remote_files::tests:: -- --nocapture`: compile6.74초/suite1.88초, 실제 트리5동작/페이지/감독자 거절1 PASS입니다. 다른2검사는 identifier 오류 기대값 및 sandbox 휴지통 거절 때문에 실패했습니다.
- [x] unsafe tab component는 원본 `error.path.invalidIdentifier`의 localized InvalidArgument이며 Forbidden이 아닙니다. fixture 기대값만 정정했습니다. 실제 휴지통 실패는 `error.file.trashFailed`이고 sandbox 제한이며 성공으로 간주하거나 trash 구현을 우회하지 않았습니다.
- [x] 실패한2검사만 exact domain filter와 `--skip 실제_트리5명령은_페이지_expand_reveal_refresh_collapse와_감독자_거절을_보존한다`로 권한 승격해 재실행했습니다. compile4.18초/suite1.81초,2 PASS입니다. 이미 성공한 트리 검사는 제외했으며 고유3검사 모두 PASS입니다. 삭제 대상은 이 실행에서 새로 생성한 합성 `nested/moved.synremote` 한 개이고 휴지통으로 이동해 복구 가능합니다. 사용자의 파일은 삭제하지 않았습니다.
- [x] catalog/actual routing arm·다른 domain 비중복·remaining 전달·JSON/raw mode·외부 path create/save/open/rename/copy/delete/mirror/raw 거절과 디스크 불변, 접근 거절 전 plugin cache 미로드·argument/identifier 오류를 확인했습니다.
- [x] 실제14파일명령/raw 검사는 합성 plugin manifest를 실제로 로드해 file_open의 languageId를 확인하고 생성·저장·복사·이동·휴지통 삭제·원본 bytes·dirty mirror keep/clear/prune·untitled2개 keep/clear/prune를 호출했습니다. 트리 검사에서는 rows/limit·expand/collapse/reveal/refresh·신규 파일 반영·없는 project·stopped supervisor의 cached tree5와 worker file4 신규 입장 거절·디스크 불변/tracked0을 확인했습니다.
- [x] 후속13과 공유 native lib/bin/tests strict clippy exit0,12.75초·authored7 exactfmt exit0입니다. 기존 Wry 경고17개는 authored 결과와 별도입니다. 새 source4파일 no-index whitespace는 빈 출력이며 exit1은 새 파일 diff 존재입니다.
- [x] PROCESS/HANDOFF/재개/QA5문서의 Bun Prettier 결과는 모두 unchanged입니다. tracked2문서 whitespace exit0, 신규3문서 no-index whitespace는 빈 출력/exit1(새 파일 diff 존재)입니다. 마지막 결과 표기 갱신은 QA2문서 단독 포맷으로 확인합니다.

## 미완료와 부채

- [ ] 남은131허용 명령·production dispatch/Settings/assets/App lifecycle·실제 remote socket 전체 명령 왕복과 장기 channel을 연결합니다. 이 adapter 검사만으로 full gateway/GUI를 완료로 표시하지 않습니다.
- [ ] file/tree의 기존 동기 IO·raw 단일 프레임 크기·전체 network byte quota·대규모 트리/동시 close/공유 캐시·전체 TOCTOU/보안/메모리/성능은 별도 M8 gate입니다. supervisor 입장 검사는 기존 worker 경계만 주장하며 원본 create/rename/delete/untitled까지 일괄 신규 shutdown gate가 있다고 주장하지 않습니다.
- [ ] 모든 플랫폼의 휴지통/사용자 권한·symlink/case/네트워크 파일시스템·실제 plugin grammar와 UI/AX·TS 제거/Rust99%·배포/rollback·keybinding RED/PTY remount 결정은 미완료입니다. 현재 goal active이며 전체 완료 뒤만 commit/push합니다.

## 원본 근거

원본 remote gateway의 file JSON14/tree JSON5/raw1, `src-tauri/src/domain/file/commands.rs`, tree commands·lib의 plugin port, root file/tree actions·plugin service·root_guard·trash delete 구현을 대조했습니다. native의 기존 plugin/store와 runtime 의존성을 그대로 사용하며 새 dependency/package/lock/MSRV·제품TS·실기 bundle 변경은 없습니다.
