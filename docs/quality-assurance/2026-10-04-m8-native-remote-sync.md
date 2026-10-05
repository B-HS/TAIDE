# M8 원격 sync 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-sync.rs`, `remote-sync-tests.rs`, `lib.rs`입니다. 원본 허용 sync3명령을 실제 `sync_actions`, AppServices의 필수 SecretStoreState, 필수 SyncGistPort factory와 Settings reconcile 포트에 연결했습니다. 선행174+3=177/177 domain adapter입니다. catalog 합집합 정적 대조는 허용177과 정확히 일치하며 missing/extra/duplicate0입니다. 전체 생산용 dispatch/App/HTTP transport 조립 완료가 아닙니다.

root/Tauri/runtime·manifest/lock·MSRV·제품 TS·보호 bundle·OS·TAIDE Git은 이번 변경에서 불변입니다. 실제 GitHub·Keychain·사용자 자격 증명·외부 네트워크·사용자 앱에는 접근하지 않았습니다.

## 원본 계약과 구현

- `src-tauri/src/domain/sync/commands.rs`, `github.rs`, `settings_port.rs`, `domain/settings/commands.rs`, 실제 `crates/taide-runtime/src/sync_actions.rs`, `settings_actions.rs`, `task_supervisor.rs`, sync service와 모델을 확인했습니다. 새로운 payload/정책/설정 적용 구현 대신 기존 root action을 호출합니다.
- outer with_policy의 admit/owner/remaining JSON·raw를 유지합니다. connect/disconnect는 원본 원격 거부입니다. SecretStoreState는 AppServices::new의 필수 포트를 쓰며 테스트는 임의 UUID 합성 marker만 반환합니다. 사용자 store/토큰을 읽거나 생성하지 않습니다.
- status는 원본 비감독 query이며 credential 부재에는 Gist factory를 실행하지 않습니다. 연결된 gist fetch 오류는 status.remoteNewer null로 원본처럼 흡수합니다. AppState shutdown flag로 새 blanket gate를 만들지 않습니다.
- upload는 원본처럼 전체 run_nonabortable_result입니다. 최초 gist create 동안 mutation guard를 유지하고 기존 gist update의 round-trip만 guard 밖입니다. snapshot 수집·보호 필드 filter·live gist 일치 재검사·live 일반 설정 유지·credential 재조회·bookkeeping persist/이벤트를 root에 위임합니다.
- download는 prepare_sync_download의 abortable fetch 뒤 apply_sync_download만 nonabortable입니다. 원본 force bool typed validation, gist/lastSync 변경 retry·conflict-before-parse·schema gate·설정 sanitize/persist→필수 reconcile→SettingsChanged→theme/locale 적용→SyncStateChanged를 보존합니다. Settings apply는 guard를 다시 잡지 않아 reentrant deadlock을 만들지 않습니다.
- production Reconcile은 preferences와 동일 타입 계약을 재사용합니다. 테스트 reconcile은 실제 live/persisted Settings와 순서/취소 수명을 검증하지만 IDE/hooks/remote OS 통합의 생산용 조립 성공을 대신하지 않습니다. 실제 GitHub HTTP Gist 구현은 현재 Tauri 측에 있어 공유 transport/생산용 factory 연결이 남습니다.

## 최소 검증과 성공 재사용

- [x] 첫 compile은 fixture의 AppError Clone 불가로 중단했습니다. 테스트 reply의 복제 가능한 String 오류를 경계에서 AppError::Internal로 바꿨으며 실제 root 오류 타입/정책은 수정하지 않았습니다.
- [x] `cargo test … --lib remote_sync::tests -- --nocapture`: compile9.66초/suite0.08초, 고유6 PASS입니다. 실제 filesystem/settings/runtime와 합성 Gist만 사용해 network 권한 승격은 필요하지 않았습니다.
- [x] native lib/bin/tests strict 첫 검사는 fixture의 explicit drop(calls)를 await-held-lock로 판정했습니다. 동일 내용 복사/lexical scope로 guard 해제를 명확히 한 뒤 관련 strict만 재실행해 exit0·7.46초입니다. 이미 성공한 동작6은 재사용했습니다. 기존 Wry dependency17 warnings와 authored strict를 구분하고 suppression은 없습니다.
- [x] authored3 exact rustfmt와 tracked `git diff --check`: exit0입니다. 신규 no-index diff 빈 출력/exit1은 정상 신규 diff입니다.
- [x] Bun 읽기 전용 source catalog 대조: adapter177/unique177/allowed177, missing/extra/duplicate0·exit0입니다. 14개 domain catalog와 file_read_raw를 실제 root 허용 목록에 대조했으며 명령을 실행한 검증이 아닙니다.

Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, native manifest, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 전체 suite/GUI/외부 GitHub/keychain 검사는 실행하지 않았습니다.

## 실제 고유6 검사

1. catalog3/실제 arm/허용 목록/선행 domain 비중복·status wire·미연결 factory0·typed force·원격 connect/disconnect 거부·remaining·원본 query shutdown 정책·fetch 실패 remoteNewer null입니다.
2. 실제 upload update 동안 기존 settings_update를 실행해 snapshot 일반 필드와 live 일반 필드가 분리되고 bookkeeping만 갱신됨을 확인했습니다. 실제 theme/locale 파일 수집·보호5필드 제외·UTC payload·원본 Settings/Sync 이벤트·disk/live 일치입니다.
3. malformed payload라도 newer/force false에는 conflict, force true에는 parse 오류·future schema 거부입니다. 유효 강제 적용은 보호5필드가 로컬값을 유지하고 Settings reconcile→settings→sync 순서와 실제 theme/locale 저장을 확인했습니다.
4. 최초 create의 guard 잠금·waiter Drop 후 nonabortable worker1 유지·감독 종료 대기/끝난 create persist와 task0, 기존 update 중 disconnect 결과를 stale bookkeeping이 되살리지 않음을 확인했습니다.
5. fetch future Drop은 실제 합성 port active1→0·task0·파일/이벤트 없음입니다. fetch 동안 live gist 또는 lastSync 변경은 force true여도 각 원본 InvalidArgument retry이며 apply·파일·이벤트를 실행하지 않았습니다.
6. 이미 저장한 apply의 reconcile 대기 중 waiter Drop 뒤 worker1을 유지하고 종료 대기·reconcile 해제 후 theme/locale와 Settings/Sync 이벤트/task0을 완료했습니다. 감독 종료 뒤 upload는 factory0, download는 원본 fetch 후 apply 입장 거절이므로 fetch1·로컬 적용0을 유지했습니다.

## 미완료와 실행 시점

- [ ] 필수 Gist factory의 공유 HTTP client·생산용 자격 증명 store·IDE→hooks→remote reconcile·177명령 full chain·assets/events/App startup/Exit는 기존 production assembly 항목에서 연결/확인합니다. 카탈로그177/177은 조립 완료가 아닙니다.
- [ ] 사용자 credential·실제 GitHub 전송/연결/해제·외부 provider 오류 상세·사용자 theme/locale는 제품 실기 단계에서 확인합니다. 합성 fixture가 라이브 데이터 테스트를 대신하지 않습니다.
- [ ] 전체 N1~N8은0/8입니다. keybinding Tab RED/PTY remount·전체 view/cutover/Rust99%·IME/AX/성능/보안/패키징/rollback gate는 남으며 full M8 완료 전 commit/push하지 않습니다.
