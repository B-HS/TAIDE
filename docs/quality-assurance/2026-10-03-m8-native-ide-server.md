# M8 native IDE WebSocket·lockfile 수명

## 대상 파일

- `native/taide-native-app/src/{ide-server,ide-server-tests,lib}.rs`
- `native/taide-native-app/{Cargo.toml,Cargo.lock}`
- `crates/taide-ide/src/store.rs`
- 원본: `src-tauri/src/domain/ide/{commands,server}.rs`
- 재사용: `crates/taide-ide/src/{lockfile,service,protocol,store}.rs`, native IDE tool dispatcher·TaskSupervisor

## 리포트

Tauri-free native IDE 서버의 실제 loopback WebSocket·인증·MCP subprotocol·JSON-RPC/notification·시작/중지/재시작과 lockfile 경계를 구현했습니다. 서로 다른4검사의 성공 근거를 확보했습니다. 원본 transport를 재사용하며 아직 NativeApplication startup/Exit·production layout callbacks·diff/save 화면 처리기·Settings 전체 reconcile에는 조립하지 않았습니다. 전체 IDE/Settings/M8 완료가 아닙니다.

## 상세

1. Ports는 실제 layout open/close callbacks와 제품 버전을 필수 입력으로 받습니다. 정상 경로는 기존 lockfile_dir·원본20회 random port·127.0.0.1 bind·UUID 인증과 private atomic lockfile을 사용합니다. 현재0.1.0 실험 버전을 제품 버전이라고 자동 응답하지 않습니다. 테스트만 별도의 합성 data/synthetic-ide 디렉터리를 반환하며 사용자 홈·실제 Claude 설정/lockfile을 접근하지 않았습니다.
2. 시작 작업 lease·ready oneshot을 보유하고 TaskSupervisor 등록 또는 shutdown 거절에는 자기 candidate lockfile과 listener/task를 회수합니다. 이미 실행 중이면 status를 반환하고 unchanged toggle은 lazy입니다. 종료는 store에서 server/connection handles·pending diff/save를 가져와 abort·자기 lockfile 제거·응답 해소·default status 발행을 수행합니다. 작업 중인 자원의 최종 drain은 감독자의 shutdown이 담당하며 stop 반환을 즉시 모든 future Drop 완료라고 주장하지 않습니다.
3. 인증은 원본 헤더의 constant_time_eq·401을 사용하고 모든 Sec-WebSocket-Protocol 헤더 값과 comma offer에서 mcp를 선택합니다.10초 handshake deadline을 유지합니다. Invalid JSON을 무시하며 ping/pong·원본 dispatcher의12tool을 사용합니다. 연결 이벤트보다 notification 구독을 먼저 수행해 동기 observer의 첫 broadcast도 유지합니다.
4. accept/connection은 감독자 handle로 등록하고 connection의 JoinSet writer/notify/request는 spawn 전 별도의 operation lease를 얻습니다. 부모 취소 시 JoinSet은 자식을 abort하고 자식 Drop까지 lease가 남습니다. 정상 종료에는 children.shutdown을 await합니다. 재시작의 token을 내부 server identity로 사용해 이전 연결의 늦은 Drop이 새 서버의 client_count를 감소시키지 않도록 store의 동일 mutex 안에서 running/token을 확인합니다. 이 token 비교는 외부 인증 대신이 아니며 기존 legacy client counter API는 변경하지 않았습니다.
5. workspace lockfile refresh와 missing-project pending 해소는 기존 service/store 계약을 사용합니다. 프로젝트가 사라진 diff의 결과는 source처럼 TAB_CLOSED입니다. 부모/자식 작업 회수와 연결 재시작을 실제 loopback에서 확인했지만 production lifecycle의 mutation 직렬화·동시 start/stop/refresh 전체를 검사한 것은 아닙니다.

## 의존성과 공식 API 근거

root/Tauri가 이미 사용하는 tokio-tungstenite0.30.0을 native의 직접 dependency로 재사용했습니다. native lock에는 이 package와 기존 root lock의 같은 버전/checksum을 가진 tungstenite0.30.0·sha1 0.11.0·digest0.11.3·block-buffer0.12.1·crypto-common0.2.2·const-oid0.10.2·hybrid-array0.4.14·data-encoding2.11.1,9개 항목이 추가됐습니다. 새로운 프로젝트 전체 버전 업그레이드나 root lock 변경을 이 작업에서 수행하지 않았습니다. native Tokio는1.53.1이며 root MSRV1.89/edition2021·native MSRV1.95/edition2024는 유지했습니다.

공식 [WebSocketStream0.30.0](https://docs.rs/tokio-tungstenite/0.30.0/tokio_tungstenite/struct.WebSocketStream.html), [accept_hdr_async](https://docs.rs/tokio-tungstenite/latest/tokio_tungstenite/fn.accept_hdr_async.html), [JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html)와 설치된0.30.0 Callback/handshake·Tokio1.53.1 JoinSet source를 확인했습니다. latest Tokio 웹 문서의1.53.2를 실제 native dependency라고 기록하지 않습니다. Callback의 외부 trait 계약을 그대로 구현하고 lint suppression을 추가하지 않았습니다.

## 실제 최소 검증

Cargo는 모두 직렬·locked/offline·`CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`·공유 target `experiments/native-shell-spike/target`로 실행했습니다. 앞선 loopback bind 권한 거절이 확인된 환경이므로 해당 합성 소켓 검사만 escalation했습니다. 보호된 실기 bundle·OS 설정·실제 사용자 clipboard·CLI는 실행하지 않았습니다.

- 최초 `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib native_ide_server`는 fixture에서 u32 client_count와 usize task count를 비교해 E0308/E0277·exit101로 컴파일에서 중단됐습니다. CLIENT_COUNT 별도 상수로 고쳤으며 검사는 실행되지 않았습니다.
- 수정 후4건 실행은 compile9.16초/suite5.01초,2 PASS·2 FAIL입니다. 시작 거절/lockfile 실패와 늦은 connection owner는 성공했습니다. 연결 이벤트의 동기 notification은5초 timeout RED이고 broad wire 검사는 테스트 클라이언트의 repeated subprotocol header 제한에 의해 InvalidSubProtocol로 실패했습니다. 새 코드의 Copy send 결과 drop warning도 발견해 `let _`로 수정했습니다.
- 구독 순서를 수정하고 실제 client offer를 단일 comma header로 바꿨습니다. 서버의 repeated header 처리 자체는 동일 Authentication Callback을 직접 호출해 별도 assertion으로 유지했습니다. repeated header를 실제 wire client로 통과했다고 주장하지 않습니다. 설치된 tungstenite client source는 반환 subprotocol을 client가 보관한 목록에 대조하며 source server의 전체 header 순회를 바꾸지 않았습니다.
- 실패 관련 필터 `--lib native_ide_server는_` 실행은 compile4.76초/suite0.06초,3건 중2 PASS·1 FAIL입니다. notification RED는 GREEN이 됐으며 broad wire fixture는 원본 missing-project TAB_CLOSED를 DIFF_REJECTED라고 잘못 기대했습니다. 필터가 이미 성공한 disabled 검사1건도 포함한 선택 실수가 있었으며 이를 새로운 고유 성공으로 중복 집계하지 않습니다. source/store를 확인해 fixture 기대값만 수정했습니다.
- 마지막 broad wire 검사만 `--lib native_ide_server는_실제_인증_mcp_rpc_notify와_pending_종료_lockfile을_보존한다` 실행: compile3.64초/suite0.06초·1 PASS/exit0입니다. missing/wrong auth401·MCP/no-offer·2연결·caller version/protocol·12tool·invalid JSON 이후 ping·두 연결 notify·pong·연결 취소 pending 회수·missing project TAB_CLOSED·empty workspace refresh·stop socket/lockfile 회수·restart 다른 token/old auth401·TaskSupervisor tracked0을 확인했습니다. 그 이전의 독립 성공3개는 재사용합니다.
- 최종 native `cargo clippy --lib --bins --tests -- -D warnings`: exit0,27.27초입니다. 기존 Wry17 dependency warning은 별도이며 authored 검사 억제는 없습니다. root store 변경의 `cargo clippy -p taide-ide --lib --tests -- -D warnings`도 exit0,1.44초입니다.
- authored native3개 edition2024/root store edition2021 exact Rustfmt·tracked diff whitespace는 exit0입니다. 이전 agent hooks2/IDE dispatcher3/AppFile7/theme11 성공을 변경 없는 근거로 재사용했고 whole suite/keybinding RED를 다시 실행하지 않았습니다.

## 완료와 잔여

- [x] 실제 인증 WebSocket/MCP·RPC/notify·lockfile/start/stop/restart 경계 구현
- [x] 서로 다른4검사의 성공·notification RED/GREEN·fixture 실패 정정·고유 성공 재사용
- [x] root static 최종 출력·authored exact Rustfmt·tracked whitespace 확인
- [x] 문서5개 기존 Prettier 포맷 exit0·신규 source/manifest/lock/QA/bug7개 whitespace 빈 출력 확인. 신규 파일 no-index exit1은 추가 diff이며 공백 오류가 아닙니다.
- [ ] production layout lifecycle·NativeApplication diff/save/selection/diagnostics 소비/발행
- [ ] 실제 remote transport·Settings IDE→hooks→remote 순서·AppFile write/저장 키·startup/Exit/aux/GUI
- [ ] 전체 IDE 보안/메모리/성능·M8 gate

원본 IDE는 unbounded outgoing channel/request spawn과 broadcast lagged skip을 사용하며 같은 정책을 유지했습니다. M7 remote256-frame 상한을 IDE에 임의로 적용하거나 새 거절 계약을 만든 것은 아닙니다. 연결/요청/응답 byte aggregate·flood/slow reader·writer 실패 후 read loop 종료 정책은 전체 보안/메모리 gate에 남습니다. pending diff/save의 종료 응답 코드·600초/5초 실제 timeout·handshake10초 대기·동시 refresh/stop/start·OS process/lockfile 권한 실측은 각각 production assembly/해당 위험 변경 시 확인하며 이번4검사를 전수 증거로 확장하지 않습니다. keybinding offscreen Tab RED·PTY remount A/B는 그대로 유지합니다.
