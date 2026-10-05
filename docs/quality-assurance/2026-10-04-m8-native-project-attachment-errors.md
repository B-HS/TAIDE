# M8 project attachment 오류 parity

## 대상·원본

대상은 `native/taide-native-app/src/projects.rs`, `projects-tests.rs`입니다. 원본 FileWatcherCapability/build_watcher_handle·GitWatcherCapability/build_git_watcher_handle·IDE refresh_lockfile은 watcher/lockfile 실패 시 경고 후 계속합니다. 기존 native 준비의 `?` 전파를 해당 정책으로 맞췄습니다. 두 watcher 호출은 같은 optional_watcher로 처리합니다.

프로젝트 canonical/open 허용 경계·source detected Git/Terminal 순서·guard-free expensive build→live/root 확인→guarded registration·감독 task·hooks 순서·shutdown 거절은 그대로입니다. 정상 watcher도 선택 실패로 무조건 제거하지 않으며 이미 등록된 watcher는 유지합니다. 신규 경고는 kind만 출력하고 경로/원문 오류는 로그에 남기지 않습니다. 실제 App on_exit의 이번 신규 로그도 kind 출력으로 한정했습니다.

## 최소 검사

- [x] `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib projects::tests::원본처럼_watcher와_lockfile -- --nocapture`: compilation6.04초/suite0.01초·RED, 합성 root 소실 후 watcher.registerFailed가 attachment에서 전파됐습니다.
- [x] 수정 후 관련 단일 검사1회: compilation8.19초/suite0.12초·고유1 PASS, filtered212입니다. root 소실의 부분 자원/layout commit과 실제 open의 IDE lockfile 준비 실패 후 project/layout/정상 file watcher commit·Terminal kind·mutation guard 해제·watcher stop 완료/task0을 확인했습니다. hooks는 off이며 home/OS/소켓/시크릿에 접근하지 않습니다. source 전체 registered detected-kind 검색 결과는 Git/Terminal 두 종류입니다.
- [x] native lib/bin/tests clippy `-- -D warnings` exit0·13.89초(handle82847 종료), authored2 exact rustfmt 완료입니다. Wry dependency17 warnings는 authored 검사와 구분합니다. 직전 실제 Exit와 기존 domain의 성공 동작은 반복하지 않았습니다.
- [x] 대상5문서 Prettier 완료·tracked whitespace check 출력 없음(exit0), 새 Rust2개/QA/bug의 no-index check 출력 없음(exit1은 신규 diff)입니다. live Cargo handle 없음입니다.

UUID 합성 root만 삭제/생성했고 IDE running store의 합성 owner에는 실제 소켓을 연결하지 않았습니다. Cargo는 직렬·locked/offline/기존 target, handles54906/97794 종료입니다. 선행 프로젝트/hooks/domain 성공은 재사용하며 제품TS/manifest/lock/root/Tauri/MSRV/Git·보호 bundle 불변입니다.

## 남은 게이트

- [ ] `.git` directory가 is_dir 확인 직후 사라지거나 시스템 watcher quota가 실제 포화되는 경합은 이번 fixture가 직접 재현하지 않았습니다. 두 watcher의 동일 실패 정책을 연결했지만 전체 OS watcher stress 증거로 주장하지 않으며 최종 watcher/보안 gate에 남깁니다.
- [ ] App ports owner/startup·Rust remote UI/자산 생성·terminal effects·HostBridge Settings/AppFile·IDE 화면·keybinding RED/PTY remount·N1~N8 0/8이 남습니다. M8 전체 완료 전 commit/push하지 않습니다.
