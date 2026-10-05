# M8 native 탐색기의 파일·폴더 생성

## 대상·원본

- 대상: `native/taide-native-app/src/{explorer,explorer_toolbar,application,lsp,host,lib}.rs`, `tests/{explorer,lsp}.rs`입니다.
- 원본: `src/features/explorer/{file-tree-toolbar,file-tree-draft-row,explorer-shortcuts}`, `src/widgets/explorer/{use-explorer-entry-crud,explorer-path,explorer-panel}`, `src/shared/lib/entry-name.ts`입니다. 구현 순서는 기존 TS view 재현을 우선하며 CPU/GPU와 사용자 담당 실제 입력기·VoiceOver 검증은 마지막으로 유지합니다.
- 고정 egui 0.36.2의 TextEdit/IME/WidgetInfo/Modifiers/Response·Label/animation/opacity와 epaint CubicBezierShape source를 확인했습니다. 설치된 Lucide의 FilePlus·FolderPlus·RefreshCw·ChevronsDownUp path를 native 선·Bezier로 옮겼으며 원본 Lucide ISC 고지 전문을 `native/taide-native-app/LICENSE-LUCIDE`에 포함했습니다. native 배포 bundle에 해당 고지를 포함하는 최종 배포 게이트와 곡선 변환의 픽셀 동일성은 아직 검증하지 않았습니다.

## 연결한 코드

- [x] 프로젝트 이름과 네 버튼·locale label/tooltip·24px 버튼/16px 아이콘/2px 간격을 추가했습니다. 클릭과 tree-local Cmd+N/Cmd+Shift+N이 파일·폴더 입력을 시작합니다. tree 단축키는 modifier를 정확하게 비교하고 입력 Enter/Escape는 TS처럼 modifier와 무관하게 처리합니다.
- [x] 실제 현재 row에서 선택 parent를 구합니다. 파일은 parent, 폴더는 자신, 사라진 선택은 프로젝트 root입니다. 접힌 폴더는 해당 path의 펼치기 응답 후 입력을 설치합니다. 일반 트리 갱신과 펼치기 응답을 별도 DTO로 구분해 무관한 갱신이 pending 생성을 취소하거나 조기 실행하지 않습니다.
- [x] 생성·이름 변경이 동일한 입력/마지막 segment 검증·전체 선택·IME preedit/commit 프레임 보호·blur/빈 입력 취소를 사용합니다. 오류가 나면 입력·error를 유지하고 completion 후 재시도할 수 있습니다. pending request latch로 중복 생성을 막습니다.
- [x] bounded 기존 bridge에서 추적된 별도 작업을 실행합니다. owned mutation guard를 실제 blocking create까지 유지하며 shutdown·live project·절대 경로와 selected project root 경계를 재검사합니다. 기존 file create_new와 directory 생성 서비스·self-write를 사용하며 CLI 승인으로 프로젝트 경계를 넓히지 않습니다.
- [x] 생성 후 guard를 해제한 뒤 parent refresh→new path reveal을 수행합니다. 파일만 preview=false 탭으로 열고 optional layout을 GUI controller에 전달합니다. 이 고정 탭은 원본 용어의 비-preview 탭이며 닫기 금지 pinned 탭을 의미하지 않습니다. 폴더는 탭을 추가하지 않고 새 경로만 선택합니다. 닫힌 프로젝트에 성공 reply로 layout을 재설치하지 않습니다.
- [x] Refresh는 기존 root와 펼친 dir refresh 경로, Collapse는 기존 tree_collapse_all을 호출합니다. Toggle는 이미 반환된 전체 TreeRowPage를 사용해 같은 tree_rows를 다시 조회하지 않습니다.
- [x] 원본 32px header/8px 수평 padding/4px gap/12px uppercase caption과 truncate를 적용했습니다. hover/tree·입력·toolbar focus에 따라 150ms opacity를 변경합니다. 현재 animation은 native linear이며 원본 CSS easing의 곡선 동일성은 아래 미검증입니다. 22px virtual row에 외부 item spacing을 더하지 않고, 생성 입력을 처음 설치하거나 다시 focus할 때 해당 draft index를 보여 줍니다.

## 검사·실패 기록

- [x] 연결 코드의 lib/bin·explorer/lsp `cargo check --locked --offline` exit 0, 1.51초입니다.
- 새 fixture의 최초 컴파일은 존재하지 않는 Bridge::new 호출 두 곳으로 실패했고 실제 connect 시그니처로 수정했습니다. 후속 컴파일은 private field가 있는 Explorer의 struct update 두 곳으로 실패했고 Default 뒤 public root 설정으로 수정했습니다. 최초 잘못된 한자 test filter는 컴파일 실패로 끝나 실제 검사를 실행하거나 성공으로 계산하지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer 생성 --locked --offline --target-dir experiments/native-shell-spike/target`: 신규 2건 통과, 0.04초입니다. toolbar 실제 pointer 클릭·선택 parent/접힌 folder/IME·수정 키 Enter·검증/중복 latch/재시도/blur/빈 입력·refresh/collapse 액션과 실제 합성 파일·폴더/고정 탭/tree/selection/충돌 내용 보존/root 밖/closed project/worker 0을 확인했습니다.
- [x] 공통 입력 renderer가 변경돼 관련 이름 변경 입력만 1회 실행했습니다. `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer 탐색기_이름입력 --locked --offline --target-dir experiments/native-shell-spike/target`: 1건 통과, 0.01초입니다. 기존 실제 rename filesystem·workspace rename/delete·Trash 성공 검사는 반복하지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer 접힌_생성_parent --locked --offline --target-dir experiments/native-shell-spike/target`: 실제 host의 일치하는 path 펼치기→입력 focus 신규 1건 통과, 0.02초입니다. 무관한 path reply 비소비·사라진 parent 거절·실패 후 다시 시작·worker 0을 확인했습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer 긴_header --locked --offline --target-dir experiments/native-shell-spike/target`: 180px 긴 caption에서도 4개 24px 버튼이 같은 y·경계 안에 있고 row pitch는 22px인 신규 1건 통과, 0.04초입니다. 40행 아래의 rename 입력을 닫은 뒤 root 생성 입력이 다시 보이고 focus를 얻는 것과 Alt+Cmd+N의 비소비도 검사했습니다. 실제 OS 화면이나 픽셀 비교가 아닌 headless layout·입력 검사입니다.
- [x] app lib/bin·explorer/lsp/host/projects 대상 strict clippy가 최초 collapsible_if 한 곳 수정 후 exit 0, 1.35초입니다. 이후 header/virtual row 변경의 lib/bin/explorer 좁은 최종 검사 결과를 별도 기록합니다. 검사기를 비활성화하지 않았습니다.
- [x] 마지막 header/virtual row lib/bin/explorer strict clippy exit 0, 0.67초입니다. 기존 생성·rename filesystem·Trash 검사는 반복하지 않았습니다.
- [x] app `cargo fmt --check`, create/open-and-context QA와 preview bug의 Prettier, `git diff --check`가 exit 0입니다. PROCESS는 활성 부분만 갱신했으며 무관한 전체 문서 재포맷은 하지 않았습니다.

## 아직 완료가 아닌 범위

- [ ] 원본 CSS easing/글꼴 weight/tracking/theme·아이콘 stroke cap/join·draft file/folder icon/error border/AX invalid·nearest scroll/full-width row hit·각 pane/window별 focus scope·전체 픽셀 동일성은 남습니다. 기본 geometry/opacity와 네 버튼의 코드 연결만으로 toolbar 화면 동등성을 통과 처리하지 않습니다.
- [ ] toast retry·context 생성/삭제·clipboard·multi-selection/typeahead·open-with·Git badges/auto reveal·전체 local/global keymap과 모든 view가 남습니다. 생성 후 refresh/tab 단계가 실패하면 물리 생성은 되돌리지 않고 오류를 반환합니다. 이런 partial success의 GUI retry/notification은 별도 경계입니다.
- [ ] 외부 filesystem 교체의 완전한 TOCTOU 봉쇄·실제 symlink/case-only/권한 실패·큐 포화/중간 종료·OS IME/VoiceOver/전체 GUI·성능/배포/TS 제거/Rust99%와 M8 N1~N8 완료는 미검증입니다.
- 사용자 앱·실기 bundle·입력기·VoiceOver 설정을 조작하지 않았습니다. 합성 임시 directory 밖의 실제 파일 생성·삭제는 없습니다. M8 완료 전 최종 commit/push는 하지 않았습니다.
