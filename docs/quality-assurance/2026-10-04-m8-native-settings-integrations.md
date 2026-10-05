# M8 Settings integration reconcile

## 대상과 구현

대상은 `native/taide-native-app/src/settings-integrations.rs`, `settings-integrations-tests.rs`, `lib.rs`입니다. 실제 native IDE·hooks·remote apply_toggle을 순서대로 await하는 필수 typed reconcile constructor를 추가했습니다. Settings persist/live와 mutation guard는 기존 shared root action이 소유하며 callback은 잠금을 재획득하지 않습니다.

- IDE는 필수 실제 Ports, hooks는 실제 native apply_toggle, remote는 필수 Weak<Ports>를 사용합니다. 원격 socket dispatch가 같은 preferences reconcile을 보유하므로 역참조를 weak로 두어 순환 소유를 끊습니다. App owner가 strong remote port를 유지해야 합니다.
- 같은 값은 기존 no-change 정책을 유지합니다. remote off는 owner가 없어도 실제 stop이며 on에서 owner가 이미 폐기됐으면 경고만 기록하고 bind/성공 이벤트를 만들지 않습니다. 원본 toggle의 실패 로그·Settings rollback 없음 정책을 유지합니다.
- 실제 App assembly/startup/Exit, HostBridge의 기존 presentation-only no-op callback 대체, Settings AppFile 저장·UI 이벤트 소비자는 아직 미완료입니다. constructor를 제품 전체 조립 완료로 계산하지 않습니다.

## 최소 검증

- [x] 정확한 `--lib settings_integrations::tests -- --nocapture`: compile9.21초/suite0.03초, 고유2 PASS입니다.
- [x] 검사1은 actual settings_update의 파일 persist/live→IDE→대기 hooks→remote→SettingsChanged 순서, 같은 AppServices, mutation guard가 계속 점유된 상태, 명령 waiter 취소 뒤 nonabortable worker의 완료·shutdown/task0을 확인합니다. side-effect observer는 합성 fixture이며 이 검사를 실제 OS 서버3개 시작으로 주장하지 않습니다.
- [x] 검사2는 actual IDE LayoutActions/Hub와 actual preferences dispatch→socket→remote Ports graph를 조립합니다. no-change/off, strong remote port 폐기·Weak upgrade None, callback만 남은 상태에서 on의 bind 없음, 마지막 reconcile 폐기 뒤 integration/Hub 해제·task0을 확인합니다. assets/credentials는 호출 시 panic이며 사용자 home/OS/network를 사용하지 않았습니다.
- [x] 신규 테스트 이름의 non_snake_case 경고를 소문자 이름으로 정정했습니다. 본문 불변으로 동작 성공2는 재사용했습니다.
- [x] native lib/bin/tests strict `cargo clippy --lib --bin taide-native-app --tests ... -- -D warnings`: exit0·13.31초입니다. 기존 Wry dependency17 warnings와 authored strict를 구분합니다. authored3 edition2024 exact rustfmt check·tracked diff check exit0, 신규 no-index check 빈 출력/exit1은 정상 신규 diff입니다.

Cargo 환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, native manifest, --locked --offline --target-dir experiments/native-shell-spike/target입니다. 실제 테스트 handle78778과 strict handle20199는 완료됐습니다. 기존 성공·전체 suite를 반복하지 않았습니다. root/Tauri/manifest/lock/MSRV/제품TS/보호 bundle/OS/TAIDE Git 불변입니다.

## 공식 근거와 미완료

[Rust Arc ownership/cycle](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.new_cyclic), [Weak upgrade](https://doc.rust-lang.org/std/sync/struct.Weak.html#method.upgrade)의 비소유 참조와 폐기 후 None 계약을 확인했습니다. 새 crate나 suppression은 없습니다.

- [ ] 실제 App owner의 IDE/remote 필수 포트·production assets/domain OS/Gist·startup/Exit와 HostBridge/AppFile/화면 처리를 연결합니다.
- [ ] 전체 N1~N8 0/8·keybinding RED·PTY remount 결정·view parity/cutover/Rust99%·IME/AX/perf/security/package/rollback은 남습니다. 전체 M8 완료 전 commit/push하지 않습니다.
