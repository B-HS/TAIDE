# 배치 23 편집기 메뉴의 실제 App 연결

상태: c9c1의 실제 App 연결과 관련 4건을 확인하고 전체 게이트를 진행합니다. editor-33은 missing→partial입니다. 현재 대상 588행 중 296완료/부분 92/미연결 112/미구현 88·292미완료이며 전체 전환율/잔여 시간은 미산정입니다. 배치 23 잔여 25행·현재 23/최종 33은 그대로입니다. 기존 c9a/c9b 기준은 [배치 23 기록](2026-10-10-native-batch23-editor.md)에 있습니다.

## 대상과 기준

- App의 application.rs·editor-context-menu.rs·editor-locations.rs·host.rs·terminal_surface.rs와 UI의 메뉴·registry·keymap·formatting-input·clipboard 기억 경계를 변경합니다.
- 실제 TS code-editor 등록과 설치 Monaco contextmenu.js의 ShowContextMenu에서 Shift+F10/textInputFocus를 확인했습니다. 기본 Monaco 메뉴는 영어를 유지하며 TS에서 번역하는 미니맵·터미널 항목만 기존 locale을 사용합니다. 표시는 기존 keymap catalog의 플랫폼/재지정/해제 값을 소비합니다.
- 실제 연결된 LSP 공급자를 현재 문서에서 조회합니다. source/owner/view key·viewport·문서 revision/메타데이터·선택/스크롤/composition과 window/pane의 활성 소유가 만료되면 실행과 늦은 클립보드 응답을 거절합니다.
- Copy/Cut/Paste는 기존 core 편집 트랜잭션과 UI ClipboardText를 소비합니다. 잘라내기는 주입 writer 성공 뒤에만 편집하며 붙여넣기의 포맷 입력·dirty/mirror·undo를 기존 경로에 인계합니다.
- 터미널 실행은 primary 선택 또는 현재 줄 뒤에 줄바꿈을 붙이고 같은 pane의 기존 terminal을 우선합니다. 없으면 기존 layout API로 열고 실제 terminal attach/Hub가 준비될 때까지 payload를 보존합니다. 임시 App과 /bin/cat의 실제 PTY에서 새/기존 세션의 전달을 확인했습니다. raw writer·종료/삭제·attach 실패의 추가 앱 경계는 아직 남습니다.

## 재현과 수정

1. 첫 App check는 UI owns_event 중복 내보내기로 실패했습니다. 이를 수정한 다음 check는 catalog의 공개 모듈 경로·private pasted/Option 반환·활성 탭 Option 비교의 5개 컴파일 오류로 실패했습니다. 실제 선언에 맞게 수정했고 최초 관련 App lib 검사가 컴파일됐습니다.
2. 최초 관련 App 4건은 1통과/3실패·exit 101입니다. Shift+F10은 레지스트리와 표시용 메타데이터에는 있으나 실행 기본 키에는 없었습니다. host 메뉴 기본 키를 별도 파일로 추가했습니다.
3. 두 번째 App 3건도 실패했습니다. App 검사에서 UI keymap.rs를 include하는 기존 경계가 App의 native-host cfg를 갖지 않아 등록을 제외했습니다. 기존 다른 기능과 같은 registry의 feature 상수를 사용해 동결 browser에서는 false를 유지합니다.
4. 새 UI 검사에서 Shift+F10→문자→Enter의 본문 편집은 차단했으나 메뉴 실행이 없었습니다. 첫 포커스 수정만으로 해결되지 않았고 진단은 opened=true/body_clicked=true/내용 불변을 확인했습니다. 본문 Enter가 생성한 clicked를 실제 pointer release와 구분하도록 수정했습니다.
5. /private/tmp/taide-batch23-menu-keyboard-click-green.log는 같은 프레임 Copy 실행·본문 불변 1건 통과/12filtered·exit 0입니다. 앞선 diagnostic/실패를 별도 성공 건수로 합산하지 않습니다.
6. App fixture의 임의 HashMap 포커스 ID, 오류를 status 문자열로 찾는 가정, undo가 기존 requires_save를 회수한다는 기대값, 정의 명령의 alias/선택 범위 가정이 잘못됐습니다. 실제 본문 response ID·기존 toast·dirty 계약과 canonical 명령을 사용했습니다. 설치 Monaco goToCommands.js의 _openReference는 collapseToStart이며 mock의 정의 응답은 같은 파일입니다. 커서가 정의 범위 안이면 기존 References 대체가 작동하므로 범위 밖 요청과 12..12 커서를 단언했습니다. 이 잘못된 전제들 때문에 같은 메뉴 검사가 여러 번 실패·재실행됐으며 이를 유의미한 구현 진척으로 계산하지 않습니다.
7. app-integrated3.log의 클립보드 관련 2건과 app-peek-final.log의 본문/peek 레이블·복사·정의 이동 1건, 최초의 터미널 대상 1건으로 서로 다른 관련 App 4건을 확인했습니다. peek 최종은 1통과/637filtered·exit 0·8.31초입니다. 중간 app-peek-diagnostic.log의 문서 준비 10초 timeout은 메뉴 실행 실패와 구분하며 원인 미확정 준비 지연을 배치 31에 남깁니다.
8. 현재 UI 전체 --all-targets --no-fail-fast 최초 실행은 28대상/491통과/1실패·exit 101입니다. 새 메뉴 13건은 모두 통과했고 기존 registry의 native 명령/액션 기대값이 새 Show Context Menu를 누락했습니다. 기대값과 readonly 실행 가능 단언을 수정한 뒤 registry-both-final.log의 영향 1건만 통과해 최종 서로 다른 492건입니다. 전체 최초 exit 101을 전체 exit 0으로 바꾸지 않고 성공한 나머지 검사는 재사용합니다.
9. app-all-targets.log의 App 전체 --all-targets --no-fail-fast 직접 1회는 67대상/878통과/0실패·보호 Trash 3제외·exit 0으로 종료했습니다. 직렬 실행과 기존 보호 skip을 유지했습니다. 앞선 관련 4건은 이 전체 대상에 포함되므로 합산하지 않습니다. 모든 소유 App/UI 검증 핸들이 종료했으며 15개 소유 Rust 파일의 rustfmt check도 exit 0입니다.

로그: /private/tmp/taide-batch23-menu-app-check.log, -check2.log, -related.log, -keymap.log, -integrated.log, -integrated2.log와 /private/tmp/taide-batch23-menu-keyboard-*.log입니다. integrated.log의 잘못된 검사 전용 leaves 조회/타입 오류는 실행 성공에서 제외하며 실제 같은 pane 조회로 수정했습니다.

## 남은 확인

- [x] App 관련 4건: 실제 본문·peek 키/복사·정의 이동/레이블·재지정, 잘라내기 실패·오래된 paste·undo/dirty, 같은 pane의 기존/새 터미널 대상
- [x] 실제 터미널 준비/attach와 raw 전달: 현재 줄/UTF-8 primary 선택·동일 session 재사용·준비 큐 회수·전체 문서 미전달
- [ ] 추가 터미널 실패/종료/소유 scope, 공급자/읽기 전용·팔레트/찾기/보조 창의 App 경계
- [x] 변경 UI 전체 --all-targets --no-fail-fast 직접 1회·실패 영향만 재검사: 최초 491통과/1실패 뒤 영향 1통과
- [x] 변경 App 전체 --all-targets --no-fail-fast 직접 1회: 67대상/878통과/0실패·보호 Trash 3제외·exit 0
- [x] frozen host/Wasm compile·소유 15Rust fmt/diff·manifest/lock/engine/동결/보호 불변·디스크: 두 compile exit 0, 여유 369GiB/사용 80%
- [x] c9c1의 검증된 연결 단위 선별 Git: 소유 21경로·외부 PROCESS 하단 제외·사용자 단독 author·AI 트레일러 없이 a81c8392 커밋/일반 푸시·원격 0/0

실제 사용자 데이터·OS 설정·클립보드·Keychain·Trash·보호 M8 앱·합성 OS 입력은 사용하지 않습니다. 새 엔진/의존성과 remote-web source/manifest/lock/기능 변경은 없습니다. 앞선 491건 전체 성공과 386GiB 여유는 변경 전 상태의 기록이며 현재 상태의 성공·디스크 수치로 재사용하지 않습니다.

## 진척·반복·끝점 확인

48.6%는 전체 기능 완성률로 사용할 근거가 없습니다. 복합 요구사항 행의 수나 검사 수를 전체 전환율로 환산하지 않습니다. 최종 배치는 기존 33이며 세부 수정 때문에 배치를 추가하지 않습니다. 23~30의 잔여 배정은 25/52/23/64/32/39/51/6이고 31~33의 통합·실기·출시·전환·제거 게이트는 별도입니다.

최근 6개 커밋은 메뉴 모델/표시·스마트 선택 계약/구현·문서의 서로 다른 변경입니다. 현재 App 전체 실행은 같은 Cargo PID 99744에서 관련 child의 테스트 로그가 LSP→remote→lib 637건 종료→나머지 대상으로 진행했습니다. 이 관찰에서 정지한 무한 반복은 확인하지 않았습니다. 과거 세션 전체의 종료성을 입증한 것은 아니며, 이번 fixture 기대값 오류로 불필요하게 길어진 반복은 위 실패 기록에 보존합니다. 동일 상태의 성공한 UI 전체를 다시 실행하지 않습니다.

App 전체와 두 frozen 컴파일은 모두 종료했습니다. native/taide-remote-web 및 native의 Cargo.toml/Cargo.lock·구문 엔진 경로의 diff는 없습니다. wasm은 기존 canvas,inspection 기능으로만 컴파일했고 feature/의존 그래프·lockfile을 바꾸지 않았습니다. 현재 메뉴 표시 소비자가 닫힌 프레임에서도 shortcut catalog를 계산하는 비용은 배치 31 성능 게이트에서 측정할 항목으로 남기며 성능 통과로 보고하지 않습니다.

## 실제 메뉴의 PTY 전달 (c9c2a)

application-context-menu-tests.rs의 새 exact 검사에서 임시 앱의 shell_override만 /bin/cat로 지정했습니다. 실제 메뉴의 현재 줄 전달 뒤 같은 파일 탭을 활성화하고 UTF-8 primary 선택을 전달해 실제 TerminalCore 출력·PTY session ID·준비 큐를 단언했습니다. 전체 문서의 나머지 줄은 전달되지 않았고 실제 사용자 터미널/앱 데이터나 OS 클립보드는 사용하지 않았습니다.

terminal-actual.log는 최초 1통과/638filtered·exit 0·8.19초입니다. 이 단위는 production 변경 없이 새 검사 1건만 추가했으므로 앞선 App 전체 878건·UI 최종 492건·frozen 결과를 재사용하며 App 전체를 다시 실행하지 않습니다. 현재 서로 다른 App 성공 근거는 879건이고 879건을 한 전체 실행에서 검사했다고 보고하지 않습니다. c9c2의 남은 앱 경계와 editor-33 partial·완료 296/미완료 292·배치 23/최종 33을 유지합니다.

새 검사의 소유 Rust 1파일 fmt/diff check는 exit 0이며 최신 디스크 여유는 368GiB/사용 80%입니다. 실제 메뉴 전달을 검증한 해당 핸들은 exit 0으로 종료했습니다. 검증한 검사/QA·기능표·소유 PROCESS만 다음 선별 Git에 포함하며 외부 문서와 PROCESS 하단은 보존합니다.

## 실제 공급 조건과 팔레트 소유 (c9c2b)

- additional-app.log의 새 2건 최초 실행은 1통과/1실패·exit 101·30.19초입니다. 실제 메뉴에서 Normal/writable·Normal/readonly·Large·ReadOnly·lossy·Normal 복원의 탐색/수정/클립보드 항목과 readonly UTF-8 선택 복사·원문 보존은 통과했습니다. 물리 대형 문서 성능이나 실제 OS 클립보드를 검사한 것은 아닙니다.
- 최초 팔레트 검사와 palette-diagnostic.log는 메뉴 확인 시점에 ContextMenu 명령이 아직 큐에 남아 있었습니다. 기존 10초 제한 안에서 명령 소비를 기다리도록 검사 전제를 수정했습니다. 시간을 늘리거나 임의 키 상태를 초기화하지 않았습니다.
- palette-owned-source.log는 본문 메뉴/복사 이후 실제 peek 팔레트 복사의 결과가 선택 문자열 대신 본문의 현재 줄이라는 1실패·exit 101·9.07초를 재현했습니다. 팔레트를 닫은 뒤 현재 focus로 대상 preview를 추측하면서 원래 source를 잃었습니다.
- App의 show_palette에서 닫기 전 source/owner·문서/선택/뷰포트를 세션으로 캡처하고 소유 본문에서 한 번만 회수합니다. 실제 body ID로 해당 peek의 메뉴를 열며 세션이 만료되면 본문 메뉴로 대체하지 않습니다. UI/엔진/manifest/lock/동결 source는 바꾸지 않았습니다.
- 수정 뒤 palette-source-green.log의 1통과·9.15초와 닫힘 사례를 추가한 palette-close-green.log의 1통과·9.12초·exit 0을 확인했습니다. 같은 검사 1건이므로 합산하지 않습니다. 소유 4Rust fmt/diff는 exit 0이며 새 App 전체는 palette-app-all-targets.log에서 직접 1회 진행 중입니다. 이전 App 전체 878은 새 코드의 전체 성공으로 재사용하지 않습니다. App에 의존하지 않는 frozen host/Wasm과 source 불변 UI 492는 재사용합니다.

- [x] 실제 metadata 공급·readonly 복사
- [x] 본문/peek 팔레트 source·확정 키 중복 실행 방지·소비 전 peek 닫힘의 본문 대체 방지
- [x] 새 App 전체 종료 결과·디스크·근거: 67대상/881통과/0실패·보호 Trash 3제외·exit 0, 여유 363GiB/사용 81%
- [x] 팔레트 source 수정 단위 선별 Git: 소유 9경로·외부 PROCESS 하단 제외·사용자 단독 author·AI 트레일러 없이 46e059aa 커밋/일반 푸시·원격 0/0
- [ ] 추가 찾기/보조 창·터미널 실패/종료/소유 scope와 c9 완료 게이트

palette-app-all-targets.log의 새 App 전체는 exit 0으로 종료했습니다. 관련 2건·재검사와 앞선 PTY 1건은 이 전체에 포함되므로 합산하지 않습니다. 소유 4Rust fmt/diff check는 exit 0이고 native의 manifest/lock·UI/syntax/remote-web source 변경은 없습니다. frozen의 egui 입력 vendor 경로도 바꾸지 않았습니다. 현재 전체와 관련 검사 핸들은 모두 종료했으며 동일 성공 검사를 다시 실행하지 않습니다.

다음 찾기 연계 검사의 원본은 설치 Monaco contextmenu.js의 ShowContextMenu(321~338)·editorContextKeys.js(23)·codeEditorWidget.js(1092~1097, 1669~1673)입니다. Shift+F10은 실제 editor view의 hasTextFocus로 설정하는 textInputFocus 조건이며 팔레트 액션 자체에는 별도 precondition이 없습니다. native 찾기 입력에서 본문 메뉴를 잘못 열지 않는지와 팔레트의 원래 editor source를 각각 검증합니다. 이 조건의 코드 확인을 실제 앱 검사 통과로 보고하지 않습니다.

현재 재개 점검에서 실제 기능표는 599행·제외 10/동결 1·대상 588 중 complete 296/partial 92/unwired 112/missing 88이며 코드 기준은 46e059aa입니다. 종료 로그와 최근 구현 커밋을 대조했고 읽기 전용 프로세스 조회에서 TAIDE Cargo/rustc/감사 작업 잔류는 없었습니다. 무관한 다른 프로젝트의 검사는 이 판정에서 제외합니다. 이 관찰은 현재 실행의 잔류 여부이며 과거 전체 반복이나 제품의 모든 루프 종료성에 대한 증명은 아닙니다. 문서 전용 커밋 뒤에는 native 코드 기준을 다시 바꾸거나 같은 성공 검사를 반복하지 않습니다.

## 실제 찾기 입력과 메뉴 source (c9c2c)

- [x] 본문/peek의 기본 Cmd+F 찾기 입력에서 Shift+F10의 본문 메뉴 차단·입력 포커스와 문서 선택 보존
- [x] 같은 찾기 입력에서 실제 팔레트 Show Context Menu→Copy의 원래 본문/peek UTF-8 선택·본문 선택/원문 보존
- [x] 새 exact 검사 최초 실행·검사 Rust 1파일 fmt/diff·디스크 여유 363GiB/사용 81%
- [ ] 보조 창·터미널 실패/종료/소유 scope와 c9 완료 게이트

application-context-menu-tests.rs의 새 실제 App 검사 1건은 본문과 peek의 두 경우를 실행합니다. 기존 App 프레임과 주입 클립보드 포트·임시 LSP를 사용하고 egui 메모리 입력만 전달합니다. 실제 OS 합성 입력이나 사용자 앱/클립보드를 사용하지 않습니다. 제품 코드·UI/엔진/manifest/lock·동결 remote-web 변경은 없습니다.

검증 명령은 cargo test --manifest-path native/taide-native-app/Cargo.toml --features inspection --locked --offline --target-dir experiments/native-shell-spike/target --lib application::context_menu_tests::실제_찾기_입력은_본문_메뉴_키를_차단하고_팔레트의_본문과_peek_대상을_보존한다 -- --exact --test-threads=1입니다. /private/tmp/taide-batch23-menu-find-actual.log는 최초 1통과/641filtered·23.74초·exit 0입니다. 해당 핸들은 종료했으며 같은 성공 검사를 반복하지 않습니다.

제품 코드 불변의 직전 App 전체 881건·UI 최종 서로 다른 492건·frozen 성공을 재사용합니다. 현재 서로 다른 App 근거는 882건이며 전체 882건을 한 번에 실행한 결과가 아닙니다. 추가 보조 창/터미널 실패·종료/소유 범위는 미완료로 유지하고 editor-33 partial·완료 296/미완료 292·배치 23/최종 33·전체율/잔여 시간 미산정을 유지합니다.

## 실제 보조 창·연결 실패·종료 회수 (c9c2d)

- [x] scoped NativeShell의 보조 창 복사·실제 PTY 전달·잘못된 Main scope 거절·본문 선택/터미널 보존·닫힌 창의 전달 회수
- [x] 실제 연결 실패 후 16프레임 전달 회수·실제 실패 탭 닫기·다음 PTY에 이전 payload 미전달
- [x] 실제 연결된 세션 종료의 대기 잔류 실패 재현·수정·앱 shutdown 회수
- [ ] 현재 수정의 App 전체 --all-targets --no-fail-fast 직접 1회 종료·경계/fmt/디스크·선별 Git
- [ ] peek LSP 메뉴의 source/owner와 보조 창 팔레트의 활성 탭·c9 완료 게이트

owner-failure.log의 새 2건은 0통과/2실패·exit 101·10.35초입니다. 보조 창 fixture는 같은 revision을 apply_layouts에만 넘겨 새 창이 표시되지 않았고, 연결 실패 검사에서는 탭 제거 직후 아직 닫기 인계가 끝나지 않았습니다. 실제 finish_mutation 발행과 pending_tab_close/활성 탭 완료 조건을 사용했습니다. owner-ready.log는 연결 실패 1통과/보조 창 1실패·exit 101·19.98초입니다. 후자는 주입 클립보드 helper가 HostBridge의 터미널 포트를 None으로 바꿔 실제 연결을 끊은 검사 전제 오류였습니다. helper에서 기존 Tabs와 실제 environment provider를 보존한 뒤 auxiliary-ports.log의 영향 1건이 23.49초·exit 0으로 통과했습니다. 성공한 연결 실패 검사를 다시 실행하지 않았으며 물리 보조 창/OS 입력 통과로 확대하지 않습니다.

closed-session-red.log는 실제 /bin/cat PTY의 종료 후 flush_dirty가 payload를 다시 대기로 보존하는 제품 버그를 0통과/1실패·8.44초·exit 101로 재현했습니다. TerminalDelivery가 처음부터 연결된 세션 ID를 보존하고 탭의 현재 ID 및 실제 terminal store의 생존을 함께 확인합니다. 이미 연결된 세션의 종료/교체는 거절하며 최초 빈 세션과 아직 연결되지 않은 탭은 기존 준비 인계를 유지합니다. 레이아웃 read guard는 terminal store 조회 전에 해제합니다.

첫 수정 검사는 terminal store를 AppState의 필드로 가정해 컴파일 실패·실행 0건이었습니다. 실제 소유자인 AppServices를 session 검증의 인자로 받아 App의 늦은 응답/flush와 Host의 raw writer가 같은 검증을 소비하도록 수정했습니다. closed-session-services-green.log는 종료/앱 shutdown 새 exact 1건이 8.50초·exit 0으로 통과했습니다. 앞선 closed-session-green.log 컴파일 실패를 통과 건수에 포함하지 않습니다.

로그 접두사는 /private/tmp/taide-batch23-menu-입니다. 제품 수정은 editor-context-menu.rs·application.rs·host.rs이며 application-context-menu-tests.rs에 새 서로 다른 3건을 추가했습니다. 이전 App 전체 881은 새 제품 코드의 전체 성공으로 재사용하지 않습니다. owner-all-targets.log의 현재 App 전체가 직접 1회 진행 중이며 UI/Editor/syntax·manifest/lock·frozen source/의존 경계는 바꾸지 않았습니다. 기존 frozen의 host/Wasm 성공은 App에 의존하지 않는 실제 그래프와 source 불변 근거로 재사용합니다.

다음 실제 App 확인은 peek 메뉴의 typed 위치 요청/이동과 보조 창 팔레트의 창별 활성 탭입니다. peek의 가상 ViewKey와 본문 owner, 팔레트의 source_focus와 scoped snapshot은 코드로 확인한 경계이며 해당 메뉴의 실제 통과로 보고하지 않습니다. editor-33 partial·296완료/292미완료·배치 23/최종 33·전체율/잔여 시간 미산정을 유지합니다.

현재 전체 실행의 lib 대상은 642통과/2실패·304.41초입니다. 기존 startup 포맷 검사는 본문 rope/revision/dirty가 보존됐으나 강제로 관찰한 read_only=true가 false로 바뀌어 실패했습니다. taide-file의 read_only는 파일 권한이 아니라 size tier/encoding_lossy에서 계산되므로 권한 변경으로 검사 전제를 고치지 않습니다. 기존 하이라이트 child 검사는 5초 준비 상한에서 Initializing에 머물러 실패했고 stderr는 비어 있었습니다. 둘의 근본 원인이나 이번 수정과의 인과 관계는 아직 확인하지 않았습니다. 전체 종료 후 동일 코드의 실패 exact만 각각 한 번 실행하며 성공한 전체 대상을 반복하지 않습니다. 실패 검사 생략·상한 확대·read_only 단언 완화는 하지 않습니다.

owner-all-targets.log의 App 전체 직접 1회는 67대상/882통과/3실패·보호 3제외·exit 101로 종료했습니다. 추가 실패는 tests/projects.rs:114의 실제_project_open_restore_file_tab_전체_tree와_exit가_같은_소유권을_쓴다 준비 시간 초과입니다. startup-exact.log는 변경 없는 readonly 실패가 13.48초에 재현됐으며 highlight-exact.log는 변경 없이 1통과/644filtered·0.32초·exit 0입니다. project-exact.log는 동일 준비 지점에서 7.55초·exit 101로 다시 실패했습니다. 기존 f1의 OS watcher/프로젝트 시작 신뢰성 과제를 유지하며 상한 확대·생략·전체 반복을 하지 않습니다. LSP 준비의 단독 성공도 최초 전체 실패 원인의 해결 증거로 확대하지 않습니다.

startup 검사는 실제 writable 파일에서 OpenedFile.read_only만 true로 바꾼 전제가 디스크 관찰의 실제 정책과 달랐습니다. 실제 Host ObserveFile→prepare_document→commit_observation→EditorStore.observe_file는 파일의 tier/encoding_lossy로 만든 최신 메타데이터를 dirty 문서에도 반영합니다. 임시 소유 파일에 invalid UTF-8 suffix를 넣어 실제 open_file이 read_only/encoding_lossy를 모두 true로 반환하도록 바꿨습니다. 기존 readonly=true·본문 rope/revision/dirty·peek 포맷/undo·disk 보존 단언을 유지하고 읽기 전용 원본은 bytes로 비교합니다. pending observation 제거·메타데이터 강제 유지·권한 변경·상한 확대를 추가하지 않았습니다. 영향 exact 결과는 아직 대기 중입니다.

startup-lossy.log는 DocumentMetadata의 실제 필드 lossy를 OpenedFile의 encoding_lossy로 가정해 컴파일 실패·실행 0건입니다. DocumentMetadata::from_opened의 실제 매핑을 확인한 뒤 lossy 단언으로 바로잡았으며 해당 로그를 통과 건수에 넣지 않습니다. startup-lossy-green.log의 영향 exact가 진행 중입니다.

startup-lossy-green.log는 실제 lossy/read_only 원본과 writable peek의 포맷·undo·본문 rope/revision/dirty·readonly=true 및 디스크 bytes 보존이 1통과/644filtered·29.70초·exit 0입니다. 성공한 전체 대상은 재실행하지 않습니다. 현재 서로 다른 App 성공은 884건이고 프로젝트 열기 1건은 미해결입니다. 최초 전체 882통과/3실패·exit 101을 전체 성공으로 바꾸지 않습니다. 소유 5Rust rustfmt --check와 6경로 diff check는 exit 0이며 동결/manifest/lock/UI/Editor/엔진 변경은 없습니다. 디스크 여유는 353GiB/사용 81%이고 정리/OS 설정 변경을 실행하지 않았습니다. 모든 이번 Cargo 핸들은 종료했으며 선별 Git은 대기 중입니다.
