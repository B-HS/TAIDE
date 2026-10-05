# M8 bootstrap event relay

## 대상과 구현

대상은 native `event-relay.rs`, `event-relay-tests.rs`, `bootstrap.rs`, `remote-git.rs`, `lib.rs`, Cargo.toml과 native lock의 app dependency entry입니다.

- 원본 `src-tauri/src/lib.rs` fanout28개와 `src-tauri/src/events.rs` payload 계약을 exhaustive AppEvent match로 옮겼습니다. 이벤트 이름/camelCase DTO와 `{t,event,payload}`를 유지하며 payload는 원본 event.payload()처럼 JSON 문자열입니다. 객체로 바꾸지 않았습니다. AgentExternalOpen/HotExitFlushRequested는 원격에서 제외하되 원래 native sink에는 전달합니다.
- bootstrap Assembly는 같은 services의 RemoteStore/GitStore를 relay에 연결합니다. relay는 AppServices 전체를 캡처하지 않으므로 소유권 순환이 없습니다. 구독자가 있을 때만 remote frame을 생성합니다.
- production Git Ports는 실제 GitEvents를 활성화하고 FsChanged/GitStatusChanged/GitRefsChanged만 UI 전달 전에 무효화합니다. Assembly는 기존 lazy registration을 유지하며 기존 App이 사용하는 bootstrap::services는 즉시 활성화합니다. FsRescanRequired를 임의로 invalidation 대상으로 추가하지 않았습니다.
- 기존 workspace taide-git는 이미 runtime의 transitive dependency이나 직접 GitStore를 사용하는 app에는 direct edge가 필요해 기존 path edge와 native lock entry만 추가했습니다. 새 registry package/version/MSRV는 없습니다.

## 최소 검증

- [x] 최초 compile은 taide-git direct edge 누락2·IdeStore::new fixture 오류1로 exit101입니다. 실제 원본 Default API와 직접 edge로 정정했습니다. 제품 정책이나 검사기를 완화하지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib event_relay::tests --locked --offline --target-dir experiments/native-shell-spike/target -- --nocapture`: compile14.28초/suite0.01초·고유1 PASS, filtered207입니다. 실제 bootstrap/remote broadcast receiver로28 이름/전체 field keyset·null/f64·quote/newline/CJK 문자열, 원격 제외2, UI 앞 cache ordering3·비대상 fresh·lazy/eager activation·services owner 해제·shutdown/task0을 확인했습니다.
- [x] 같은 manifest의 native lib/bin/tests clippy `-- -D warnings`: exit0·15.69초입니다. Wry dependency17 warnings는 authored strict와 구분합니다. authored5 exact rustfmt·tracked diff check exit0·신규2 no-index check 출력 없음(exit1은 신규 diff)을 확인했습니다.

CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo이며 Cargo는 직렬로 실행했습니다. 종료 handles37088/14256/23757, live handle 없음입니다. 같은 상태 성공/전체 suite는 반복하지 않았습니다. 합성 state/경로만 사용하고 사용자 앱/보호 bundle/OS/home/Keychain/제품TS/TAIDE Git은 조작하지 않았습니다.

## 공식 근거와 미완료

[serde_json json macro](https://docs.rs/serde_json/1.0.151/serde_json/macro.json.html)와 [to_string](https://docs.rs/serde_json/1.0.151/serde_json/fn.to_string.html), 원본 Tauri fanout/typed payload를 확인했습니다. 원격 보안 제외 항목을 확장하거나 새 전송 계약을 도입하지 않았습니다.

- [ ] 실제 full production App assembly·assets·server startup/Exit와 IDE diff/save/selection 화면 소비자·HostBridge Settings/AppFile write는 기존 미완료입니다. relay의 headless 성공을 해당 기능 완료로 계산하지 않습니다.
- [ ] native editor LSP recovery·keybinding RED/PTY remount·N1~N8 0/8·실기/성능/최종 cutover gate는 남습니다. 전체 M8 완료 뒤에만 commit/push합니다.
