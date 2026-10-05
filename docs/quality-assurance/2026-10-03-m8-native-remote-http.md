# M8 native 원격 HTTP 인증·파일 제공·서버 수명

## 대상 파일

- `native/taide-native-app/src/{remote-http,remote-http-tests,remote-serving,lib}.rs`
- `native/taide-native-app/{Cargo.toml,Cargo.lock}`
- 원본: `src-tauri/src/domain/remote/{commands,server,serving,types}.rs`
- 재사용: `crates/taide-remote/src/{service,store,login_page,types}.rs`, `crates/taide-infra/src/{root_guard,range_file,secret}.rs`, TaskSupervisor

## 리포트

원격 HTTP의 실제 Host/Origin 방어·링크/nonce/세션 cookie·비밀번호/독립 lockout·프로젝트 파일/range/stream과 시작/중지/재시작 포트를 native로 구현했습니다. 실제 loopback·합성 메모리 secret·합성 파일에서 서로 다른4검사가 통과했습니다. WebSocket upgrade route는 실제 인증 layer와 digest·operation lease를 사용하되 socket 소비자는 필수 typed port입니다. 아직 native 원격 command/channel/WebSocket 소비자·생산용 assets·NativeApplication/Settings/startup/Exit 조립이 없으므로 원격 전체/Settings/M8 완료가 아닙니다.

## 상세

1. 모든 route/fallback에 원본 순서의 Host allowlist→POST Origin 필수→Origin exact match→활성 session→login route→단일 link→password-only login 정책을 적용합니다. nonce가 위조/소비/만료됐으면 hash 조회 전에 거절합니다. keyring 오류는 password configured로 취급해 무비밀번호 fast path로 내려가지 않으며 POST에서는403입니다. 원본 Rust login_page·salt/hash·constant-time 검증·nonce/anonymous 독립 backoff를 재사용합니다. nonce/session token을 로그에 쓰지 않습니다.
2. cookie는 원본 HttpOnly/SameSite=Strict/Path/nonce Max-Age를 사용합니다. loopback은 X-Forwarded-Proto를 무시하고 등록된 tunnel hostname만 첫 https token에 의해 Secure로 처리합니다. 성공 후 nonce cookie를 지우고 nonce를 재사용하지 못합니다. URL encoded password의 plus/percent·UTF-8와 원본 잘못된 percent 처리도 유지합니다. 근거 없는 새 trim/로그인 정책을 넣지 않습니다.
3. 시작은 TaskSupervisor lease·loopback bind·ready oneshot·RemoteStore mark_started를 사용합니다. cached start/unchanged toggle·shutdown/등록 거절을 source에 맞춥니다. 원본 remote_start 자체에 없던 settings-enabled gate는 새로 추가하지 않습니다. stop은 take_shutdown_state의 세션 무효화·watch signal·최대2초 server abort·default event를 사용합니다. OS keyring cache refresh는 별도 포트이며 테스트에서는 SecretStore를 합성 메모리 구현으로 교체했습니다. 정상 기본 경로의 keyring get을 실행하지 않았습니다.
4. axum::serve는 내부 HTTP 연결을 tokio::spawn하므로 native의 감독자에게 해당 future Drop이 드러나지 않습니다. 같은 axum Router를 Hyper HTTP/1·TowerToHyperService로 구동하고 연결마다 spawn 전 operation lease를 잡는 JoinSet을 사용했습니다. source처럼 종료 신호 뒤 listener를 먼저 해제하고 연결의 graceful shutdown·최대2초 drain/abort를 수행합니다. 부모 abort 때 JoinSet의 자식 Drop까지 lease가 남습니다. HTTP/WS upgrade를 위한10초 header read deadline과 TokioTimer를 명시했고 axum TcpListener source의 connection error 즉시 재시도/나머지 오류1초 지연도 유지합니다. 이10초 header deadline을 모든 WebSocket 소비자 수명·프레임 deadline으로 확대하지 않습니다.
5. file query/wildcard route는 열린 프로젝트의 기존 root guard·확장자 MIME·range parser/read_slice와 응답 CSP/nosniff/no-store/Content-Range를 사용합니다. 전체 파일은 원본64KiB 청크로 stream합니다. SPA path/index fallback은 필수 AssetResolver가 반환하는 실제 asset bytes/mime을 사용하고 원본 browser CSP를 적용합니다. 제품 assets resolver·개발 서버 proxy/최종 package asset 조립은 아직 없습니다. 검사에서 반환한 합성 HTML을 제품 remote UI라고 주장하지 않습니다.

## 실제 최소 검증

Cargo는 직렬·locked/offline·`CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`·공유 target `experiments/native-shell-spike/target`를 사용합니다. 앞서 확인된 loopback bind 제한 때문에 소켓 테스트만 정확한 합성 범위로 escalation했습니다. 보호 bundle·사용자 홈/실제 Claude lockfile·keychain·clipboard·CLI·OS 설정은 건드리지 않았습니다.

- 최초 `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib native_remote_http`는 빠진 taide-remote 직접 edge 때문에 E0432/E0433·exit101로 컴파일에서 중단됐습니다. 기존 코어는 transitive로 이미 lock에 있었고 직접 edge를 추가했습니다. 이 단계에서 테스트는 실행되지 않았습니다.
- 동일 최초3검사의 실제 실행: compile10.57초/suite0.03초·2 PASS/1 FAIL입니다. Host/Origin·링크·cookie·실제 파일/range/64KiB 초과 stream·revoke/stop/restart/TaskSupervisor0과 unchanged toggle/shutdown 거절은 성공했습니다. password 검사만 fixture가5회 실패 후 lockout을 기대해303≠429였습니다. 실제 Store는5회를 초과하는6번째부터 lockout하며 source를 변경하지 않았습니다.
- 실패한 password/nonce 검사만 다시 실행: compile5.29초/suite0.03초·1 PASS/exit0입니다. forged nonce의 hash 조회0·wrong/correct password·nonce 소비/replay·익명 axis lockout 중 nonce axis 성공·등록 tunnel Secure·loopback spoofed proto 무시·keyring 실패 link redirect와 fresh nonce POST403을 확인했습니다. 앞선 다른2성공은 반복하지 않았습니다.
- 신규 미완성 요청 수명 검사의 초기 버전은 compile4.19초/suite0.02초·1 PASS입니다. 처음은 두 연결 admission을 기다린 뒤 supervisor shutdown했고 body 처리기의 실제 진입까지 보장하지 않았습니다. 이후 server+두 connection을 초과하는 request lease 조건으로 강화한 검사도 compile1.91초/suite0.02초·PASS였지만 역시 supervisor abort와 standalone stop을 구분하지 못했습니다. 이를 최종 종료 증거로 사용하지 않았습니다.
- 같은 수명 fixture를 실제 body 처리기 진입 후 stop 단독으로 작업0까지 기다리도록 바꾸고 drain 전에 listener를 해제했습니다. 변경 위험의 해당 검사만 재실행: compile4.43초/suite2.01초·1 PASS/exit0입니다. 미완성 header/body의 두 소켓·처리 operation·server/stop 작업이 모두 회수되고 옛 port의 새 연결이 거절됐습니다. 그 뒤 supervisor shutdown도 task0이며 한 고유 검사로만 집계합니다. 입력과 검사 범위를 강화한 것이며 같은 완료 판정을 세 번 집계하지 않습니다.
- 최초 native lib/bin/tests strict exit0,27.04초입니다. 이후 바뀐 listener 종료·강화된 fixture의 최종 strict도 exit0,13.09초입니다. root 제품 코드를 이 작업에서 수정하지 않아 앞선 root IDE static은 재사용합니다. 기존 Wry17 dependency warnings는 별도이며 suppression을 추가하지 않았습니다.
- authored native4파일 exact Rustfmt edition2024·tracked whitespace exit0입니다. 변경 없는 IDE server4/dispatcher3/hooks2/AppFile7/theme11 성공·별개 keybinding RED/PTY 결정은 재사용/보존하며 whole suite green을 주장하지 않습니다.

## 의존성과 API 근거

root/Tauri의 axum0.8.9(default features off/http1,tokio,ws)를 native에 재사용했습니다. native lock에는 같은 root 버전/checksum의 axum0.8.9·axum-core0.5.6·matchit0.8.4·mime0.3.17·sha1 0.10.7·tokio-tungstenite0.29.0·tungstenite0.29.0,7개 항목이 추가됐습니다. IDE의0.30.0과 원본 axum ws의0.29.0은 root처럼 공존합니다. 기존 hyper-util0.1.20의 service feature만 활성화했으며 taide-remote 직접 edge도 기존 package입니다. root lock/MSRV1.89/edition2021·native MSRV1.95/edition2024·native Tokio1.53.1은 유지합니다.

공식 [axum Router0.8.9](https://docs.rs/axum/0.8.9/axum/routing/struct.Router.html), [WebSocketUpgrade0.8.9](https://docs.rs/axum/0.8.9/axum/extract/ws/struct.WebSocketUpgrade.html), [Hyper HTTP/1 Builder1.11.0](https://docs.rs/hyper/1.11.0/hyper/server/conn/http1/struct.Builder.html), [TowerToHyperService0.1.20](https://docs.rs/hyper-util/0.1.20/hyper_util/service/struct.TowerToHyperService.html)와 설치된 axum middleware/serve/listener·기존 native preview HTTP의 Hyper/JoinSet 경계를 확인했습니다. 도구에서 열리지 않은 axum middleware URL은 해당0.8.9 설치 source로 확인했습니다.

## 완료와 잔여

- [x] 실제 HTTP 인증/로그인·파일/range/stream·typed assets/WS upgrade 경계·서버 연결 감독
- [x] 서로 다른4검사의 성공과 fixture 오류/약한 초기 수명 증거의 구분
- [x] 최종 strict·authored exact Rustfmt·tracked whitespace
- [x] 문서4개 기존 Prettier 포맷 exit0·신규 source/manifest/lock/QA7개 whitespace 빈 출력 확인. 신규 no-index exit1은 추가 diff이며 공백 오류가 아닙니다.
- [ ] 실제 native WS 요청·JSON/binary/channel·events·256-frame 포화/세션 만료4001·revocation·request/owner 수명
- [ ] production assets/개발 proxy·전체 remote command policy와 실제 dispatcher·remote state/event bridge
- [ ] Settings IDE→hooks→remote·AppFile write/저장 키·startup/Exit/aux/실기와 전체 M8 gate

현재 테스트의 SocketAction은 호출되면 실패하는 fixture이며 실제 WebSocket 소비자를 제품에 no-op으로 조립한 것은 아닙니다. production caller는 필수 action/assets를 제공해야 합니다. 다음 WS 작업에서 원본 channel endian/seq/index/end와 pending dispatch limiter·256-frame 상한 및 큰 단일 프레임 메모리의 기존 합의를 유지하고 실제 native 소비자를 연결해야 합니다. session TTL401/4001·range wildcard/악성 symlink/TOCTOU·HTTP slow-reader/connection flood·전체 memory/worker·동시 start/stop/update·실제 keyring/GUI/OS는 아직 전수 검증이 아닙니다. 각각 해당 소비자/production lifecycle/전체 보안·성능 gate가 준비될 때 검사하며 이번 합성4검사를 면제 근거로 삼지 않습니다.
