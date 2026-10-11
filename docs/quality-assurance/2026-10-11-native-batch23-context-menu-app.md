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
