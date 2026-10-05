# M8 원격 Git 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-git.rs`, `remote-git-tests.rs`, `lib.rs`입니다. 원본 Git41명령을 실제 root git_actions에 연결했습니다. 선행95와 합쳐 domain adapter136/177·남은41이며 생산용 전체 assembly와 M8 N1~N8은 미완료0/8입니다.

## 구현 경계

- 필수 remaining JSON/channel/raw와 바깥 with_policy를 유지합니다. 원본 typed 인자·GitActionContext·owned mutation/repo guard·감독 blocking worker·status/refs 이벤트를 그대로 사용합니다.
- status는 GitStore의 once-only 설치를 통해 필수 install_status_invalidation 포트를 호출합니다. FsChanged/GitStatusChanged/GitRefsChanged에 실제 cache를 무효화하는 test observer를 사용했지만 생산용 App observer는 아직 조립하지 않았습니다.
- diff의 실제 PluginStore.ensure_loaded/language_overlays를 연결했습니다. status Fresh는 operation admission보다 앞서므로 종료 후 유효 cache 응답을 그대로 보존하며 임의 blanket shutdown gate를 추가하지 않았습니다.
- 부분 줄 unstage는 원본 reverse diff의 old_cursor를 사용합니다. 줄2의 replacement에서 선택2..2는 삭제 줄만 선택하고 복구 추가 줄의 cursor3은 제외해 `one\nthree\n`가 됩니다. 잘못된 전체 복구 fixture 기대값을 이 실제 정책으로 정정하고, 이어 전체 파일 unstage로 원상복구되는지 별도로 확인했습니다. 원본 서비스를 수정하지 않았으며 이 결과를 제품 개선/정상적인 전체 복구라고 주장하지 않습니다.
- manifest/lock/dependency/MSRV/root/Tauri/제품TS·보호bundle·OS 설정을 변경하지 않았습니다.

## 검증

환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, manifest native/taide-native-app/Cargo.toml, locked/offline, target experiments/native-shell-spike/target입니다.

- [x] 최초 `cargo test --lib remote_git::tests:: -- --nocapture`: compile9.82초/suite0.84초,3 PASS/1 FAIL입니다. 실패는 위 부분 unstage 기대값이며 assertion 결과를 읽고 실제 service/select_hunk_lines로 원인을 확인했습니다.
- [x] 실패한 `remote_git::tests::실제_조회_patch`만 재실행: compile5.07초/suite0.21초,1 PASS입니다. 통과한3건은 재실행하지 않았습니다. 고유4검사 모두 PASS이며 이전 domain 성공도 재사용합니다.
- [x] catalog41/actual arm/allowlist·이전 domain 비중복·없는 project/typed 오류/정책/remaining/channel/raw·감독자 종료 뒤 입장 거절과 불변 이력을 확인했습니다.
- [x] 실제 init 재초기화/clean status/사용자/로그/파일/commit-files/file-log/blame·cache hit와 FsChanged invalidation·plugin 언어 overlay·gutter/diff fallback·hunk/line patch·whole-file stage/unstage/discard·외부 경로 거절·종료 뒤 Fresh cache를 확인했습니다. 새 저장소 최초 생성 자체는 CLI fixture setup이며 init backend는 재초기화 경계를 검사했습니다.
- [x] 실제 branch 생성/조회/checkout/delete·stash push/list/apply/drop·stage/commit·tag 생성/조회/delete·revert/soft undo·정확한 원본 status/refs 이벤트를 확인했습니다.
- [x] 새 합성 local bare 저장소에 실제 push/pull/fetch·원격 branch checkout·ahead/behind/remote URL을 확인했습니다. 별도 새 합성 저장소의 충돌을 만들고 base/ours/theirs/workdir과 resolve/index 상태를 확인했습니다.
- [x] `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,16.62초와 authored3 exact rustfmt exit0입니다. 기존 Wry dependency17경고는 별도이며 억제를 추가하지 않았습니다.

## 격리와 미완료

모든 Git 이력/파일/remote는 UUID 이름의 새 임시 fixture 아래 있습니다. setup/probe Git 자식은 GIT_CONFIG_GLOBAL=/dev/null·GIT_CONFIG_NOSYSTEM=1·비대화형과 빈 hook/template을 사용합니다. 실제 root git service에는 repo-local signing/hook/fsmonitor/credential/protocol 설정을 적용했지만 기존 서비스가 process/global config를 전혀 읽지 않는다고 주장하지 않습니다. 사용자 global config를 수정하지 않았습니다. 외부 네트워크·자격증명·사용자 저장소·TAIDE commit/push/force는 사용하지 않았습니다. 각 fixture는 자기 supervisor를 종료하고 자기 임시 디렉터리만 회수했습니다.

- [ ] 실제 production status observer·전체177 backend/socket/Settings/assets/App startup/Exit/HostBridge를 조립합니다.
- [ ] 인증된 외부 Git transport·장기 stall/실제 hook·전체 AppExit/GUI/성능/보안·keybinding RED/PTY remount·TS 제거/Rust99%·배포/rollback은 기존 M8 gate이며 이4검사의 완료 근거가 아닙니다.

## 근거

원본 Tauri gateway Git41 arm/commands와 root git_actions/GitStore/git service/typed model을 대조했습니다. fixture config 격리는 [Git config 공식 문서](https://git-scm.com/docs/git-config), 자식별 환경은 [Rust Command 공식 문서](https://doc.rust-lang.org/std/process/struct.Command.html)를 확인했습니다.
