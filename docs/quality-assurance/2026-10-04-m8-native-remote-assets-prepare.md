# M8 원격 자산 준비·시작 연결

## 대상과 구현

대상은 `native/taide-native-app/src/remote-assets.rs`, `remote-assets-prepare-tests.rs`, `application-ports.rs`, `settings-integrations.rs`입니다. 기존 공개 manifest·Anchor/Stamp·MIME·payload/count quota의 frozen resolver를 lazy Catalog로 준비합니다. 생성 시 IO하지 않으며, 준비가 성공한 뒤에만 Settings/production Ports의 원격 listen을 시작합니다. 실패는 kind만 경고하고 캐시를 게시하지 않아 재시도가 가능합니다.

감독된 nonabortable 작업이 Tokio mutex를 소유해 caller 취소 후에도 worker 완료를 추적합니다. 동시에 준비하는 호출은 직렬화하며 이미 성공한 Catalog는 source를 재읽지 않습니다. resolver는 캐시만 소유하므로 Catalog 소멸 뒤에도 기존 bytes를 제공하고 요청에서 FS에 접근하지 않습니다. off/no-change는 준비하지 않으며 shutdown 뒤 prepare는 이미 cache가 있어도 admission에 실패합니다. 기존 static-ready constructor는 caller가 제공한 resolver 계약을 유지합니다.

실제 ApplicationPorts.start는 Settings와 동일 reconcile을 사용합니다. IDE→hooks→remote 순서와 weak RemotePorts 역참조를 보존합니다. 이는 실제 NativeApplication의 소유자·artifact 선택·caller 연결 완료를 뜻하지 않습니다.

## 최소 검증

- [x] `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib remote_assets::prepare_tests::준비_waiter -- --nocapture`: compile12.68초/suite0.01초·1 PASS(handle89922 종료). 실제 blocking pool 점유 상태에서 waiter 취소 뒤 tracked owner 유지·두 번째 준비 직렬 완료·원본 bytes·자기 source 삭제 후 cache 재사용·shutdown admission·Catalog/standalone resolver 수명을 확인했습니다.
- [x] 같은 옵션의 filter `remote_assets::prepare_tests::production_startup과_settings`: compile0.21초/suite0.02초·1 PASS(exit0). 알려진 sandbox bind 제한 때문에 이 신규 검사만 localhost/UUID 합성 자원 범위로 승격했습니다. all-off 무준비·missing assets 무listen·준비 성공 후 실제 서버 running·실제 static handler body·source 삭제 뒤 off/stop·task0·RemotePorts/Catalog owner 해제를 확인했습니다. auth/WS/terminal을 새로 검사한 결과로 계산하지 않습니다.
- [x] native lib/bin/tests clippy `-- -D warnings` exit0·15.81초(handle20907 종료). authored4 exact rustfmt는 직전 구현에서 완료됐습니다. 기존 Wry dependency17 warnings는 별도이며 검사기를 비활성화하지 않았습니다.
- [x] 대상4문서 Prettier 완료·tracked whitespace check 출력 없음(exit0), 신규 Rust4개/QA의 no-index check 출력 없음(exit1은 신규 diff)입니다. live Cargo handle은 없습니다.

동일 성공 검사와 기존 auth/WS/domain 검사는 재사용했습니다. 두 신규 검사는 서로 다른 취소/동시성 위험과 startup 연결 위험을 각 한 번 검사합니다. UUID 임시 fixture 외 사용자 앱/보호 bundle·home·키 파일·OS 설정에는 접근하지 않았습니다. 의존성/manifest/lock/제품TS/Git 변경은 없습니다.

## 남은 경계

- [ ] 실제 NativeApplication 필수 ports owner/caller·remote terminal effects·Rust 원격 UI 생성/패키징을 연결해야 합니다. legacy TS dist를 읽거나 최종 fallback으로 사용하지 않았습니다.
- [ ] cache quota는 payload/count이고 전체 RSS·HTTP clone peak·WebSocket byte cap 증거가 아닙니다. WebSocket은 합의한 256프레임 정책을 유지합니다.
- [ ] Windows/Linux 자산 Anchor·OS/GUI·전체 기능/성능/보안·cutover와 N1~N8은 미완료입니다. M8 전체 완료 전 commit/push하지 않습니다.
