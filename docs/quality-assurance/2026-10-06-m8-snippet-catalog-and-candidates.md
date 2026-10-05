# M8 Snippets 앱 전체 캐시·후보 데이터

후속 콘텐츠200ms/배경150ms Presence·기본 키 입력 정본은 [Dialog QA](2026-10-06-m8-snippet-dialog-presence.md)입니다. 아래 Dialog 미완료 표기는 당시 경계이며 원본 전체 스타일/포커스·실제 Chrome/종료/자동완성 UI는 계속 남습니다.

## 대상 파일

- shared UI `snippet-catalog.rs`, `snippet-completion.rs`, `snippet-draft.rs`, `snippet-editor.rs`, `settings-view.rs`, lib 등록입니다.
- native `application.rs`, `host.rs`, `snippet-host-tests.rs`입니다.
- remote `snippet-operations.rs`, `browser-workbench.rs`, `browser-editor.rs`, `tests/snippet-catalog.rs`입니다.
- `tests/snippet-completion.rs`와 `fixtures/snippet-completion-parity.json`입니다.

선행 실제 저장·삭제·Toast·unmount admission 근거는 [소비자 QA](2026-10-06-m8-snippet-consumers.md)입니다. 이 문서는 이후 앱 전체 목록 캐시와 순수 후보 계산의 정본이며, 자동완성 추천 화면이나 삽입 완료를 주장하지 않습니다.

## 원본 계약

1. `src/app/bootstrap-snippets.ts`는 앱 전체 수명 QueryObserver를 구독하고 completion getter가 같은 query cache를 읽습니다. Settings가 사라져도 observer가 살아 있으므로 기본10분 inactive GC의 대상이 아닙니다. Settings 전용 캐시로 대체하지 않았습니다.
2. `src/app/query-client.ts`의 stale60초·retry0·refetchOnWindowFocus=false와 `snippet.query.ts`의 성공 mutation invalidation을 대조했습니다. staleTime은 주기적인 폴링 간격이 아닙니다. 새 관찰/재연결의 stale 조회와 invalidation의 active 재조회 경계는 [공식 기본값](https://tanstack.com/query/latest/docs/framework/react/guides/important-defaults), [공식 무효화](https://tanstack.com/query/latest/docs/framework/react/guides/query-invalidation)와 대조했습니다.
3. `src/shared/lib/snippet-completion.ts`와 원본 테스트를 읽었습니다. 언어 파일 이름과 global 확장자는 대소문자를 구별하며 global scope만 ECMAScript trim/쉼표 분리합니다. prefix는 문자열 그대로 각각 후보로 만들고 body/description 배열은 개행으로 합칩니다. plugin의 동적 language ID를31개 Settings 옵션으로 제한하지 않습니다.

## 구현

1. 앱 전체 `Catalog`가 root Weak lifetime·독립 read identity·단일 진행 조회·성공 Arc snapshot·60초 stale·관찰자 reply·오류·invalidation을 소유합니다. 초기 목록은 Settings 없이 조회하며 새 Settings에는 기존 데이터가 즉시 표시됩니다. stale 관찰은 데이터를 유지하며 background read를 요청합니다. 실패는 이전 성공 데이터를 유지하고 자동 반복하지 않습니다. 새 mount/명시 refresh/재연결은 별도 read로 재시도할 수 있습니다.
2. `Views.finish_frame`은 Settings editor만 회수하며 전역 목록은 유지합니다. root clear는 이전 Weak lifetime을 해제합니다. mutation success는 editor가 이미 사라져도 전역 목록을 무효화하며, 늦은 이전 read는 새 generation을 덮지 않습니다. 성공 재조회는 모든 살아 있는 takeover에 기존 `State.set_files`로 전달하므로 draft를 덮지 않습니다. 같은 파일 결과는 동일 Arc를 재사용합니다.
3. native `background_tick`과 browser `poll`에 실제 bootstrap read를 연결했습니다. native는 기존 supervised HostBridge/read action, browser는 기존 인증된 `snippet_list`/null 인자·seq decoding 경계입니다. 개별 Settings List는 전역 캐시에 관찰 요청으로 합쳐집니다. 목록 조회는 mutation 종료 대기/실패 큐에 섞지 않습니다. 단절은 pending을 일회성 정산하고 재연결 전에 자동 replay하지 않습니다.
4. 공용 `snippet_completion::collect`는 원본 후보 계산을 옮겼습니다. 숫자 object key 순서·빈/배열/raw prefix·배열 body/description·scope·case·plugin 언어를 보존합니다. 캐시 갱신 후 collector가 읽는 데이터는 새 snapshot입니다. 실제 NativeEditor의 추천 UI/provider, Monaco InsertAsSnippet에 대응하는 placeholder/선택/삽입 controller는 아직 연결하지 않았습니다.

## 신규 검증

- [x] 공용 앱 캐시 연속1 PASS(build1.97초/suite0.00초·filtered38): bootstrap/동시 관찰 dedup·즉시 cached reply·60초 경계·실패 데이터 유지/무반복·새 관찰 retry·동일 Arc·Settings 없는601초 유지·mutation 세대/옛 응답 폐기·새 후보/삭제·단절/재연결·root drop입니다. 시계를 전달하여 실제60초/10분을 기다리지 않았습니다.
- [x] 원본 TypeScript 후보 대조1 PASS(2.35초/0.00초): 설치된 Bun으로 실제 `collectSnippetCompletionCandidates`를 실행해 얻은 기대값을 fixture로 고정했습니다. rust/go/plugin-lang/Rust/plaintext5개 case에서 후보7/2/6/2/2=19개 전체 객체가 일치합니다. FEFF trim/NEL 비trim·숫자2/10/01 순서·개행/빈 description·raw prefix·언어 파일 scope 무시·uppercase 확장자 제외를 포함합니다. Rust 함수로 기대값을 생성하지 않았습니다.
- [x] 실제 native Host 전역 조회 연속1 PASS(21.14초/.03초·filtered319): Settings layout이 없는 실제 AppState에서 bootstrap read→빈 목록→synthetic runtime save→actual Host read→Settings 없는 frame 회수 뒤 Arc 유지→변경 read/새 body→삭제 read/빈 목록→root clear/실행 및 late reply 거절→worker disconnect/task shutdown입니다. 저장·삭제 setup은 runtime action이며 이번 검사는 UI 클릭 증거가 아닙니다. OS clipboard/Keychain은 호출하지 않고 자기 UUID 임시 파일만 사용·성공 뒤 제거했습니다.
- [x] 신규 portable 원격 wire 연속1 PASS(3.79초/0.00초): Settings 없는 조회·DTO decoding·중복 reply·옛 invalidation 세대·잘못된 null DTO·성공 cache 유지·조회 실패가 close mutation 실패 큐에 들어가지 않음·중복 disconnected 일회성·late ack 폐기·명시 reconnect recovery·root clear입니다. 실제 WebSocket/Chrome 실행이 아니라 portable seq 소비자 검사입니다.
- [x] 최종 native lib/bins/tests check9.42초·normal Canvas Wasm check.89초 exit0입니다. 현재 소스는 실제 native/portable 공통 경로로 컴파일했습니다. 신규 Rust14 파일 대상 rustfmt와 diff 검사 exit0이며 선행 성공한 core/UI/Toast/admission 검사는 반복하지 않았습니다. 기존 Wry17/큰 libtest eh_frame 경고 외 새 경고·검사 억제·의존성/버전/lock 변경은 없습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --lib snippet_catalog::tests::앱_전체_snippet_catalog는_관찰자_중복_신선도_실패_변경_세대와_단절을_보존한다 --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -- --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --test snippet-completion --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --lib snippet_host_tests::snippet_전역_catalog는_settings없이_실제_host를_조회하고_변경_삭제와_종료_세대를_보존한다 --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -- --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --test snippet-catalog --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

## 미완료·다음 경계

- 원본 Dialog200ms Presence·전체 스타일·포커스/AX/theme/DPI와 실제 Snippets Chrome 연속 화면/입력/Toast/거절·명시 retry/close drain은 남습니다. 최신 bindings는 선행 키바인딩 소스이고 이번 전역 bootstrap은 모든 BrowserEditor 모드에 추가 조회를 발생시킵니다. 다음 probe는 서버의 `snippet_list` fixture와 요청 집계를 현재 소스에 맞춰야 합니다. 옛 Chrome 성공을 현재 캐시 bootstrap의 성공으로 재사용하지 않습니다.
- actual NativeApplication frame bootstrap/완전 native 종료 drain·브라우저 auth/recovery 실측도 남습니다. 실제 Host 조회 및 ordinary unmount 저장 성공을 앱 OS 종료 전체 성공으로 확대하지 않습니다.
- NativeEditor 추천 UI/삽입/placeholder·LSP suggestion과의 정합은 N3/전체 editor 경계에 남습니다. 이번 순수 collector만으로 자동완성을 완료 체크하지 않습니다.
- 나머지 Settings/App/assets·최종 N1~N8·TS 제거/Rust99%·사용자 담당 CJK/VoiceOver 마지막 검증은 남습니다.

이번 캐시·후보 경계4/4(100%)·Snippets 전체1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8·전체 ETA 산정 보류입니다. goal active·main 직접·workflow/서브에이전트 없음·전체 M8 완료 전 Git 없음입니다. 보호 bundle/OS 설정/사용자 프로젝트/실제 Keychain/제품 TS/원격 저장소는 변경하지 않았습니다.
