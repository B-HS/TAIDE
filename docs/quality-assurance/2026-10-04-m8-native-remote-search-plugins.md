# M8 원격 search/plugin 실제 backend

## 대상과 결과

대상은 `native/taide-native-app/src/remote-search.rs`, `remote-plugins.rs`, `remote-content-tests.rs`, `lib.rs`입니다. 원본 검색4명령과 플러그인3명령을 기존 runtime에 연결했습니다. 선행 preferences19/IDE7/file-tree20/layout19/project-session23과 합쳐95/177의 domain adapter이며 남은82명령과 생산용 전체 assembly는 미완료입니다. 전체 M8 N1~N8은0/8·목표 active입니다.

## 구현 경계

- 각 `extend_backend(remaining)`은 자기 명령만 처리하며 다른 JSON/channel/raw는 필수 remaining에 전달합니다. 바깥 `with_policy`의 default-deny·owner 강제·raw 모드와 plugin install/uninstall 거절을 유지합니다.
- 검색은 기존 SearchRunContext·SearchStore·TaskSupervisor와 per-file blocking replace/mutation·self-write를 사용합니다. `onMatch`의 원본 localized 필수 인자 오류와 `__CHANNEL__:` prefix 제거, JSON batch 전달을 연결했습니다. 원본 Tauri의 channel send 실패 무시 정책을 유지하고 transport 포화 정책을 바꾸지 않습니다.
- SearchRun/SearchListFiles perf span을 보존합니다. worker 종료/입장 거절은 기존 runtime 경계이며 `search_cancel`과 plugin list/reload/read에 임의 shutdown gate를 추가하지 않았습니다.
- 플러그인은 실제 read-through PluginStore·reload mutation·manifest/grammar 검사와 읽기 시점 contribution root guard를 사용합니다. 원본 install/uninstall을 허용하지 않습니다. 새 dependency/package/lock/MSRV·root/Tauri/제품TS·보호 bundle·OS 설정 변경은 없습니다.

## 검증

Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, manifest `native/taide-native-app/Cargo.toml`, `--locked --offline`, target `experiments/native-shell-spike/target`입니다. 모든 파일/manifest/grammar는 새 합성 임시 프로젝트/data 아래 생성했습니다. 사용자 데이터/앱/키체인/클립보드/설정은 사용하지 않았습니다.

- [x] 최초 컴파일은 AppServices의 `plugins` 필드가 없고 self-write `take_recent`가 private라 실패했습니다. 실제 `plugin` 필드와 public `resolve_from_app`으로 수정했습니다. API 가시성을 넓히거나 검사 억제를 추가하지 않았으며 최초 시점에 실행한 검사/통과 결과는 없었습니다.
- [x] `cargo test --lib remote_content_tests:: -- --nocapture`: compile7.82초/suite0.05초, 고유4검사 모두 PASS입니다. 이전 layout/project/remote 검사들은 입력/구현이 같아 재사용하고 재실행하지 않았습니다.
- [x] catalog7/actual routing arm·이전 domain 비중복·허용/default-deny·typed argument·누락 channel localized 오류·remaining channel/raw/owner 전달·plugin 거절 전 cache 비실행·tracked0을 확인했습니다.
- [x] actual 검색은 합성 텍스트2개에서 batch JSON·UTF-16 보조평면 문자 위치와 전후 문맥·총3match·channel owner Drop·파일 목록을 확인했습니다. 파일1개만 실제 atomic 치환·self-write하고 다른 프로젝트 내부 파일 및 외부 명시 경로는 불변입니다. 사용한 보조평면 문자는 Unicode escape로 쓴 Deseret 문자이며 이모지를 추가하지 않았습니다.
- [x] actual SearchStore의 remote owner 취소는 main 세션을 취소하지 않습니다. prefix 유무2경로·channel delivery 거절의 원본 결과 유지·regex 오류·없는 project·stopped supervisor의 run/replace/list 입장 거절·디스크 불변과 cancel의 기존 수명을 확인했습니다.
- [x] actual plugin3은 empty cache·새 manifest 생성 뒤 reload 전 빈 결과 유지·manifest 이름 갱신/reload·grammar 원문·없는 plugin/language 오류를 확인했습니다. Unix에서는 캐시에 로드된 grammar 파일을 새 합성 외부 파일로 향하는 symlink로 교체해 읽기 시점 root escape 거절과 외부 파일 불변을 확인했습니다. 실제 plugin 설치·삭제나 사용자 파일 삭제는 하지 않았습니다.
- [x] native lib/bin/tests strict `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,12.91초·authored4 exactfmt exit0입니다. 기존 Wry dependency17경고는 별도이며 authored 검사 억제는 없습니다.
- [x] PROCESS/HANDOFF/재개/QA 문서의 Bun Prettier 결과는 unchanged·exit0이며 tracked2문서 whitespace exit0입니다. 신규 source8·QA3·재개1의 no-index whitespace는 모두 빈 출력이며 exit1은 신규 파일 diff 존재입니다. 마지막 결과 표기는 이 QA 단독 포맷으로 확인합니다.

## 미완료

- [ ] 남은82 backend와 실제 socket dispatcher/장기 channel·production lifecycle/AppInfo/Settings IDE→hooks→remote·assets/events/App startup/Exit·HostBridge 저장을 조립합니다. fixture remaining을 제품 handler로 세지 않습니다.
- [ ] 다음 원본 Git41명령은 root git_actions와 Tauri commands/gateway를 읽었습니다. status invalidation 설치 포트와 diff plugin overlay·cache hit/감독자 admission 차이를 보존해야 합니다. 아직 native Git adapter를 작성하거나 검사한 상태가 아닙니다.
- [ ] 검색 대규모/동시 replace/모든 symlink·plugin grammar tokenizer/GUI/AX·전체 CPU/메모리/byte quota·보안·TS 제거/Rust99%·배포/rollback·keybinding RED/PTY remount 결정은 기존 M8 gate입니다. 전체 suite green·M8 완료를 주장하지 않으며 전체 완료 뒤만 commit/push합니다.

## 근거

원본 `src-tauri/src/remote_gateway.rs` search4/plugin3 arm·channel 인자 처리와 search/plugin commands, root `search_actions`/SearchStore/search service·plugin actions/service/model·AppServices·public self-write API를 확인했습니다. 기존 serde_json1.0.151 직렬화/역직렬화 공식 문서와 현재 runtime 경로를 재사용하며 새 API/의존성을 추측으로 추가하지 않았습니다.
