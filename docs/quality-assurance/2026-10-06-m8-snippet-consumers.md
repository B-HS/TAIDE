# M8 Snippets 실제 typed 소비자·쓰기 수명

후속 앱 전체 cache/원본 후보 데이터 정본은 [전역 캐시 QA](2026-10-06-m8-snippet-catalog-and-candidates.md)입니다. 아래 bootstrap 미완료 표기는 이 소비자 경계 당시 이력이며 실제 추천 UI/삽입과 전체 shutdown/Chrome는 계속 미완료입니다.

## 대상 파일

- 공용 `snippet-edit.rs`, `snippet-editor.rs`, `settings-view.rs`, `toast.rs`입니다.
- native `host.rs`, `application.rs`, `snippet-host-tests.rs`/lib 등록입니다.
- remote `snippet-operations.rs`, `browser-workbench.rs`, `browser-editor.rs`, `browser-application.rs`, `close.rs`, lib/검사 등록입니다.
- `resources/toasts/success.svg`와 설치된 Sonner MIT LICENSE입니다.

## 구현 결과

1. native AppSurfaces의 실제 Output.snippets→HostCommand→64-capacity 기존 supervised worker→기존 snippet_actions→HostReply→matched Editor를 연결했습니다. submit 거절도 typed reply로 돌아가 pending 입력을 해제합니다. List는 owner/Weak lifetime을 계속 확인합니다. 쓰기는 HostBridge.submit에서 현재 owner/lifetime을 검증한 뒤 admission을 기록해 큐 대기 중 화면을 닫아도 실행을 취소하지 않습니다. 새 요청/재승인에는 현재 owner가 필수이며 admitted 값으로 재승인을 우회하지 않습니다. UI is_active는 admission과 독립이므로 폐기된 화면에 늦은 결과를 적용하지 않습니다.
2. browser 실제 Settings output→BrowserWorkbench→기존 authenticated invoke/snippet_list/save/delete→seq/typed 결과→matched Editor를 연결했습니다. 원본 fileName/content 계약·null delete 응답을 사용하고 저장 결과의 file_name 불일치/비JSON/잘못된 DTO는 실패 처리합니다. mutation과 조회를 구별하고 sent 요청은 unmount 후에도 정산합니다. Closed/auth/dispose는 pending을 한 번 실패로 끝내며 자동 replay하지 않습니다.
3. BrowserApplication Pending은 Snippets 저장/삭제를 기다리며 거절/Closed는 CloseFailure::Snippet으로 전달합니다. 종료 실패 큐를 각 poll에서 일회성 drain하여 정상 실행 중 과거 실패가 성공 retry 뒤 새 종료를 다시 실패시키지 않게 했습니다. 실제 Chrome 종료 실측은 아직 수행하지 않았습니다.
4. 살아 있는 모든 Snippets takeover는 각 성공 mutation 결과에서 목록을 무효화하고 재조회하며, 최신 mutation observer가 아니거나 원래 editor가 사라져도 다른 active takeover의 목록을 갱신합니다. dirty 입력은 State.set_files의 기존 보존 계약을 유지합니다. 전역 앱 목록 query/bootstrap·stale/GC·자동완성은 아직 연결하지 않았습니다.
5. Toast는 원본 저장 success·미완성 count·중복명·create InvalidArgument의 invalidFileName·save InvalidArgument의 parseError·나머지 saveFailed·delete describe-error를 같은 공용 provider로 표시합니다. Sonner 설치본 SuccessIcon과 light/dark success HSL의 RGB 값을 사용하며 warning으로 대체하지 않습니다. 기존 warning/error 인덱스와 provider 수명은 보존합니다.

## 신규 검증

- [x] portable 실제 renderer 요청→wire 연속1 PASS(1.13초 build/.05초 suite): List→Save 거절→명시 retry/별도 identity→다른 파일 응답 거절→성공→재조회→Delete→unmount→Closed/late ack 폐기/중복 disconnected 무동작입니다. seq11/12/13/16/14 각각 독립이며 unknown15는 미소비입니다. 초기 private Appearance 접근 compiler 오류는 공개 Snippet Appearance 별도 생성으로 고쳤고 Dialog 첫 sizing frame의 disabled fixture 관찰은 실제 stable frame까지 한 번 표시해 정정했습니다. 생산 click 강제/포커스 주입은 없습니다. fixture의 selected/open 상태 초기화는 통신 검사 setup이며 전체 사용자 상호작용 증거는 선행 UI 검사와 구별합니다.
- [x] shared 신규 admission·Toast 독립2 PASS(2.54초/.01초·filtered47). admission은 현재 owner/Weak 체크→쓰기 승인→unmount/owner 제거→승인된 쓰기만 계속 허용/조회와 재승인 거절입니다. Toast는7 Notice 경로/count/description 없음·성공 아이콘 실제 raster/기존 error와 다른 texture/light·dark 색을 검사했습니다.
- [x] actual native HostBridge 연속1 PASS(8.65초/.05초·filtered318). actual List→NewFile 요청→합성 clipboard callback gate로 worker를 대기시킨 뒤 Save enqueue→Editor drop/owner 제거→이전 조회 admission 거절→gate 해제→실제 atomic rust.json 저장/내용 `{}`→worker disconnect/task shutdown입니다. OS clipboard는 호출하지 않았고 callback은 합성 동기화 포트입니다. synthetic temp 데이터만 쓰고 성공 fixture는 제거합니다. 초기 eframe::egui import compiler 오류와 gate의 CopiedText ack를 Save ack로 기대한 fixture 오류를 정정했습니다. 마지막 단일 검사 성공은 반복하지 않았습니다.
- [x] 최종 native lib/bins/tests check7.48초·normal Canvas Wasm check.89초 exit0입니다. 앞선 consumer 중간 check7.24초/.66초는 후속 admission/전체 active invalidation 변경 전 이력이며 최종 성공과 중복 집계하지 않습니다. Toast count argument 타입 compiler 오류는 실제 message API의 &str 경계로 수정했습니다.
- [x] authored Rust20 exactfmt·git diff --check exit0입니다. 기존 UI/core/키바인딩/서버 backend 성공 검사는 재사용했습니다. 기존 Wry17/큰 libtest eh_frame linker 경고 외 새 경고/검사 억제는 없습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --test snippet-operations --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --lib snippet_ --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --lib snippet_host_tests::snippet_host의_승인된_실제_저장은_큐_대기중_unmount에도_보존되고_늦은_조회는_거절된다 --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -- --exact
```

## 남은 범위·다음 구현

- 원본 `src/app/bootstrap-snippets.ts`는 앱 전체 수명 QueryObserver로 목록을 항상 활성 상태로 두고 completion getter가 그 cache를 읽습니다. 단순 Settings remount cache/10분 GC만 구현하면 원본 계약이 아닙니다. 실제 query-client 기본60초 stale/10분 GC/retry0/refetchOnWindowFocus=false와 앱 bootstrap·모든 mutation invalidation·새 completion 데이터 갱신이 다음입니다.
- actual Chrome Snippets 화면/Toast/거절·명시 retry/close drain·auth/recovery/no replay는 미완료이고 최신 bindings는 선행 키바인딩 소스입니다. normal Wasm 컴파일을 실제 브라우저 통과로 세지 않습니다.
- native 앱 종료 전체 drain은 기존 host shutdown/queued reply 수명과 함께 아직 검증·완료하지 않았습니다. 이번 unmount admission 성공을 OS 종료/hot-exit 성공으로 확대하지 않습니다.
- 원본 Dialog200ms Presence/전체 스타일·포커스/AX/theme/DPI와 실제 CJK/VoiceOver 사용자-last·나머지 Settings/App/assets/N1~N8/Rust99%는 남습니다.

Snippets1/4(25%, UI/소비자 단계 부분 진행)·provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 산정 보류입니다. goal active·main 직접·서브에이전트/workflow 없음·전체완료 전 commit/push 없음·live handle 없음입니다. 보호 앱/OS 설정/사용자 프로젝트/실제 Keychain/원격 저장소는 변경하지 않았습니다.
