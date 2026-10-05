# M8 실제 App Exit 서버 정리

## 대상과 구현

`native/taide-native-app/src/application.rs`, `application-exit-tests.rs`, `ide-server.rs`가 대상입니다. 정상 shutdown과 on_exit 오류 fallback이 공통 drain_services를 사용합니다. 실제 App의 기존 draft/layout flush 후 begin_shutdown·CLI marker cleanup·remote/hooks/IDE stop·search cancel·ExitDrain을 수행합니다. 서버 owner/startup이 아직 조립되지 않은 사실을 이 종료 검사로 완료 처리하지 않습니다.

기존 on_exit의 shutdown state flag 기반 건너뛰기는 실제 pending/shutdown 결과 기반으로 바꿨습니다. 실패 후 정리 성공에도 원래 오류를 보존하고 actual on_exit에서 보고합니다. 테스트 IDE directory constructor는 cfg(test)로만 제공하며 제품 home/lockfile resolver를 바꾸지 않았습니다. Tauri의 `src-tauri/src/lib.rs` Exit 처리기와 공유 root cleanup/서버 stop API를 근거로 구현했습니다.

## 최소 검사

- [x] `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib application::exit_tests -- --nocapture`: 처음 compilation9.08초/suite0.01초는 sandbox의 IDE bind Operation not permitted로 검사 본문 미진입입니다.
- [x] 동일 단일 검사만 loopback 한정 escalation: compilation0.15초/suite0.02초·RED입니다. 실제 App shutdown 이후 state admission 종료/task0을 통과했으나 IDE running=true로 실패했습니다.
- [x] 수정 후 같은 검사 1회: compilation5.99초/suite0.08초·고유1 PASS, filtered211입니다. 두 서로 다른 경로(정상 Exit, 이미 shutdown=true인 draft 오류 후 직접 Exit fallback)의 IDE/hooks/remote 상태·원격 세션 회수·IDE save=false/diff=rejected·lockfile 삭제·validated marker 삭제·search flag cancel·옛 listener 폐쇄·새 operation 거절/task0을 확인했습니다. 원래 draft 오류는 그대로 반환합니다.
- [x] native lib/bin/tests clippy `-- -D warnings` exit0·13.69초(handle10324 종료), authored3 exact rustfmt 완료입니다. Wry dependency17 warnings는 authored 검사와 별도로 기록하며 성공 동작 검사는 반복하지 않았습니다.
- [x] 대상5문서 Prettier 완료, tracked diff check exit0·새 test/QA/bug no-index whitespace check 출력 없음(exit1은 신규 diff)입니다. live Cargo handle 없음입니다.

합성 UUID 디렉터리/검사 소유 wait marker와 localhost만 사용했습니다. home/hooks 설치·Keychain·사용자 파일·OS 앱/설정·보호 bundle에 접근하지 않았습니다. 검사 fixture의 socket/assets/layout 행동은 의도적 panic이며 제품에 조립하지 않습니다. Cargo는 직렬·locked/offline/기존 target이며 종료 handle82960/1597입니다. 기존 auth/WS/IDE protocol 성공은 재사용합니다. manifest/lock/root/Tauri/MSRV/제품TS/Git 불변입니다.

## 미완료

- [ ] 실제 NativeApplication GUI의 OS on_exit 이벤트·활성 WS/IDE 편집 중 Exit·draft 실패 후 retry/GPU/AX는 이 headless 검사로 완료 처리하지 않습니다.
- [ ] 필수 assets/remote terminal effects·actual App ports owner/startup·HostBridge Settings/AppFile·IDE 화면·keybinding RED/PTY remount·N1~N8 0/8이 남습니다. M8 전체 완료 뒤에만 commit/push합니다.
