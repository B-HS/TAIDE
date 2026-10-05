# M8 actual App shell event relay

## 대상과 구현

대상은 native `bootstrap.rs`, `bootstrap-shell-tests.rs`, `application.rs`입니다. 앞선 event-relay headless 성공만으로는 actual NativeApplication의 controller 발행 경로를 덮지 못했습니다.

- 원래 App은 ShellController를 PaintSink로 연결한 다음 AppServices.events만 relay로 감쌌습니다. services 이벤트는 remote에 전달되지만 controller의 ShellMutation은 자신의 NativeShellEvents→PaintSink로 바로 발행해 relay를 우회했습니다.
- 실제 App 연결을 bootstrap::connect_shell로 옮겨 동일 state/tasks의 기존 순서를 먼저 재현했습니다. PaintSink에 WindowChromeChanged가 도착하고 snapshot.zen=true임에도 remote receiver가 Empty인 RED를 확인했습니다. 임의 timeout 대기나 이벤트 재발행으로 보완하지 않았습니다.
- 수정은 services의 relay→PaintSink를 먼저 만들고 controller를 해당 relay로 연결한 뒤, 아직 공유되지 않은 services의 events를 같은 NativeShellEvents로 바꾸는 순서입니다. controller와 services 모두 NativeShellEvents→relay→PaintSink를 한 번 거치며 기존 refresh Notify/snapshot/paint 수명을 유지합니다. relay의 forward가 ShellEvents를 다시 가리키지 않아 재귀나 소유권 순환이 없습니다.
- Arc::get_mut은 서비스를 다른 owner에 넘기기 전에만 호출하며 공유가 먼저 발생하면 명시적으로 실패합니다. unsafe/get_mut_unchecked·Arc 재복제·전역 mutable pointer를 사용하지 않습니다. GitEvents는 기존 eager App 정책대로 활성화합니다.

## 최소 검증

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib bootstrap::shell_tests --locked --offline --target-dir experiments/native-shell-spike/target -- --nocapture`: 최초 compile8.45초/suite0.01초·1 FAIL, WindowChromeChanged의 remote receiver Empty입니다. 동일 실제 App 순서가 사용되고 paint/snapshot이 먼저 통과한 상태에서 재현했습니다.
- [x] 연결 순서 수정 뒤 같은 검사만1회: compile5.84초/suite0.02초·고유1 PASS, filtered209입니다. 실제 ShellMutation의 native paint·remote JSON 문자열 payload와 state snapshot, services 직접 ThemeChanged의 양쪽 전달·추가 remote frame 없음·paint 정확히2개·controller worker 종료·shutdown/task0·AppServices owner 해제를 확인했습니다.
- [x] native lib/bin/tests clippy `-- -D warnings`: exit0·13.72초입니다. authored3 exactfmt·tracked diff check exit0·신규 test no-index check 출력 없음(exit1은 신규 diff)을 확인했습니다. Wry dependency17 warnings는 authored strict와 구분합니다.

CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, --locked --offline --target-dir experiments/native-shell-spike/target이며 모든 Cargo는 직렬입니다. 종료 handles56856/17193/87611, live handle 없음입니다. event relay28계약/production owner 등 입력이 같은 성공은 재사용했습니다. 합성 UUID data/state/actor만 사용했고 실제 NativeApplication GUI/OS 앱은 실행하지 않았습니다. manifest/lock/root/Tauri/MSRV/제품TS/보호 bundle/사용자 설정/자격 증명/TAIDE Git은 불변입니다.

## 근거와 미완료

[Rust Arc::get_mut](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.get_mut), 실제 `native/taide-native-ui/src/controller.rs`의 worker_events/forward/refresh와 기존 App constructor를 대조했습니다.

- [ ] production ports owner의 실제 App 보유·필수 assets/remote terminal effects·HostBridge Settings no-op 대체/AppFile save·IDE/hooks/remote startup/Exit·IDE diff/save/selection 화면 처리는 다음 기존 gate입니다.
- [ ] native editor LSP recovery·keybinding RED/PTY remount·N1~N8 0/8·실기/성능/최종 cutover는 남습니다. 전체 M8 완료 뒤에만 commit/push합니다.
