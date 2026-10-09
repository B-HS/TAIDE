# 배치 18 — 워크스페이스 심볼 검색·팔레트 이동

현재 상태: 구현 `2870345d`와 완료 근거 `a99fb507`를 일반 푸시해 0/0을 확인했습니다. 모델·팔레트·실제 child/host/앱 연결과 변경 app/UI 전체 86대상·1018건이 통과했고 체크리스트 7/7입니다. 기능 대응표는 완료 281/588(47.8%), 부분 93·미연결 113·미구현 101입니다. 전체 출시 전환율/잔여 시간은 미산정이며 main이 직접 수행합니다. Cargo 직렬 규칙 이탈 1회는 아래 실제 로그와 함께 기록하며 batch19 구문 접기 공급·수동/Import 명령으로 계속합니다.

## 범위와 기준

실제 `use-workspace-symbol-search.ts`·`adapters/workspace-symbol.ts`·`command-palette-workspace-symbol-group.tsx`·팔레트 선택은 프로젝트의 여러 준비된 LSP 세션에서 `workspace/symbol`을 받아 성공 응답을 합치고 해결된 file URI 위치만 표시합니다. 이름/컨테이너·서버 결과 순서를 보존하고 로컬 fuzzy는 이름 강조를 복구하며 결과를 다시 숨기거나 정렬하지 않습니다. # 입력이 비었거나 프로젝트가 없을 때의 안내, 현재 입력 응답만 표시하는 loading과 preview/UTF-16 위치 이동을 확인합니다.

native는 기존 typed SDK와 앱 전용 세션/취소·팔레트 native-host 경계를 재사용합니다. 200ms trailing의 의도와 실제 원본의 두 timer 층을 대조하며 원본 버그/내부 수치를 강제하지 않습니다. 여러 세션의 독립 오류·미지원/빈/해결 안 된 URI, 검색/프로젝트/서버 교체·닫힘·늦은 응답과 루트 guard를 검증합니다. 새 기능/디자인·의존성/정규식 엔진·OS 합성 입력을 추가하지 않으며 browser source/manifest/lock·실제 앱 데이터/clipboard/Keychain/Trash를 보존합니다.

## 체크리스트

- [x] a. 실제 TS·공식 protocol/로컬 typed API·native 공급/소비 대조
- [x] b. 응답 모델·좌표/순서·검색/프로젝트/세대·debounce/취소 수명
- [x] c. 실제 typed 여러 세션 요청·부분 오류/미지원·준비/재시작/취소
- [x] d. # 팔레트 이름/컨테이너/Hash·강조/안내·키/마우스/IME 선택
- [x] e. 실제 앱/host preview·기존 탭·UTF-16 reveal·경계/늦은 선택 통합
- [x] f. 위험 회귀·변경 크레이트 전체 대상·동결/포맷/diff·디스크
- [x] g. 실제 QA/기능표/PROCESS·선별 커밋·일반 푸시·다음 범위

## 검증 상태

설치된 공식 lsp-types 0.97.0 `workspace_symbols.rs`·`request.rs`와 url 2.5.8 `to_file_path` API, 저장소 SDK의 typed workspace/symbol/capability·Running 준비/실패·future drop 취소를 실제 코드로 확인했습니다. 웹 공식 문서 조회는 본문을 반환하지 못했으므로 웹 문서를 읽었다고 판정하지 않습니다. 기존 의존성/API로 구현하며 새 패키지·엔진은 추가하지 않습니다.

`workspace-symbols.rs`의 200ms trailing·JS 공백과 raw 검색 식별·프로젝트/서버/세대 변경·취소·늦은 응답/선택·서버 결과 순서·file/lazy/가상 URI·flat/nested·UTF-16 좌표 검사 3건이 통과했습니다(`/private/tmp/taide-batch18-state-first.log`, exit 0). 실제 child·앱/host 검사를 진행 중이며 팔레트 표면 22건은 통과했습니다. 원본은 hook과 search 인스턴스에 각각 200ms timer를 두므로 native에서는 사용자 버그/내부 수치 강제 재현 제외 지시에 따라 단일 200ms trailing으로 연결합니다.

변경 없는 배치 17 app 전체 67대상·648건과 추가 실패 수정·menu 12건의 최종 서로 다른 649건, batch16 editor 201/UI inspection 358, 이전 syntax/SDK 결과는 해당 경계가 바뀌지 않을 때 재사용합니다. 이번 변경 app/UI 전체 대상은 f에서 별도로 실행합니다. 보호 Trash 3·기존 ignored 5와 실제 OS/접근성·pixel/대형/출시 부채는 미검증입니다.

## 팔레트 표면과 실패 수정

관련 UI 팔레트 단위 22건이 통과했습니다(`/private/tmp/taide-batch18-palette-size-after.log`, exit 0, 실행 0.29초). 현재 raw 질의 일치·pending gate·서버 순서/미일치 이름과 같은 이름 중복·Hash 16px·컨테이너 inline 32px·loading/빈/프로젝트 안내·세대/index의 Enter/마우스 선택·질의 교체 후 Enter 무동작을 확인합니다. 기존 공통 IME/Tab/포커스·전환·스크롤 검사도 포함됩니다.

첫 20 성공/2 실패는 새 검사의 실제 locale와 다른 기대 문구와 과거 # 미연결의 No results 기대였습니다. 실제 메시지/새 loading에 맞췄으며 이어 남은 클릭 실패는 원래 4번째 행 y=494~~526이 목록 clip y=366~~470 밖에 있었기 때문입니다(`/private/tmp/taide-batch18-palette-click-diagnostic.log`, exit 101). egui Area가 이전 작은 크기를 max rect로 공급하므로 `ScrollArea::max_height`만으로는 비동기 목록이 자라지 않았습니다. 본문의 max height를 기존 input 36px+list 300px로 공급하고 실제 내용에 대한 shrink를 유지해 수정했습니다. 마지막 행 click과 기존 21건이 함께 통과했으며 검사에서 보이지 않는 행을 다른 선택으로 대체하지 않았습니다.

## 실제 공급·host·앱 통합

실제 child 4·host 3·모델 3건이 통과했습니다(`/private/tmp/taide-batch18-workspace-app-after.log`, 10 성공/앱 1 실패). flat/nested/lazy·미지원/오류, 두 Cargo root의 세션 생성 순서·부분 오류와 다른 프로젝트 세션 격리, hold 요청을 취소하면서 다른 문서를 동기화하는 경로, 서버 종료 후 동일 owner의 새 세대/mirror replay·이전 검색 만료를 확인합니다. host는 main/보조 창 범위의 기존 고정 탭/preview·저장된 view state·현재 reply/reveal과 mutation 잠금 대기 중 layout revision/focus/창/프로젝트/slot 교체·경계 밖/디렉터리/0 좌표 거절을 확인합니다. 종료 뒤 해당 TaskSupervisor의 tracked count는 0입니다.

child 첫 실행은 모델 3 성공/child 2 시간 초과였습니다(`/private/tmp/taide-batch18-workspace-after-api.log`). 준비 상태를 세분해 TransportClosed를 확인했고 fixture stderr가 `fixture requires valid roots and client capabilities`를 반환했습니다(`/private/tmp/taide-batch18-workspace-stderr.log`). mock의 root 검사 분기가 formatting 모드만 native root로 인정해 새 workspace 모드 초기화를 종료한 원인이었습니다. workspace 모드도 실제 root를 검사하도록 수정·재빌드한 뒤 관련 9건이 통과했고 재시작 검사도 통과했습니다. 검사 코드의 잘못된 TaskSupervisor API·진단 타입/borrow는 실제 API로 수정했으며 실패를 성공으로 집계하지 않습니다.

실제 앱의 # 팔레트→typed child 응답→Enter→host→기존 탭→editor reveal 검사 1건이 통과했습니다(`/private/tmp/taide-batch18-app-navigation-after-focus.log`, exit 0, 실행 2.94초). 이모지 뒤 UTF-16 2:5가 byte 12로 이동하고 기존 탭 ID·문서 revision을 보존합니다. 첫 앱 실패는 input이 focus되기 전 sizing 프레임에 텍스트를 넣어 질의가 비었던 검사 준비 문제였습니다. 실제 공통 팔레트 검사의 두 준비 프레임을 적용한 뒤 통과했습니다. OS 입력/실제 사용자 데이터·clipboard/Keychain/Trash를 조작하지 않았습니다. 크기 수정과 Hash/검색 표면은 native-host에만 반영합니다.

## 전체 게이트·보호·실기 부채

UI inspection 전체 19대상·359건이 exit 0으로 통과했습니다(`/private/tmp/taide-batch18-ui-full.log`, 컴파일 21.18초). app 전체 67대상·659건도 exit 0으로 통과했습니다(`/private/tmp/taide-batch18-app-full-after.log`, 컴파일 4.94초). 합계 86대상·1018건이며 실행된 실패/ignored는 0, 실제 Trash에 영향을 주는 보호 3건은 filtered입니다. 변경 없는 batch16 editor 201건은 재사용하므로 최신 서로 다른 editor/UI/app 성공은 1219건입니다. syntax batch13 159·SDK batch12 단위 52/문서 167은 별도 재사용 근거이며 기존 ignored 5와 실기 부채는 성공으로 집계하지 않습니다.

첫 app 전체 명령은 `tests/lsp.rs:292`의 기존 저장 검사에 새 WorkspaceSymbols 응답 분기가 없어 컴파일에서 exit 101로 멈췄습니다(`/private/tmp/taide-batch18-app-full.log`). 새 응답을 예상하지 않은 요청으로 명시 거절하는 분기를 보완하고 전체 실행이 통과했습니다. 컴파일 실패 명령을 전체 검사 성공으로 세지 않습니다.

```text
cargo test --all-targets --no-fail-fast --features inspection --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target
cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --no-fail-fast -- --skip remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다 --skip 실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다 --skip 실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다
cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target
cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target wasm32-unknown-unknown --features canvas --target-dir experiments/native-shell-spike/target
cargo fmt --manifest-path native/taide-native-app/Cargo.toml --check
cargo fmt --manifest-path native/taide-native-ui/Cargo.toml --check
```

동결 host 11.86초·Wasm canvas 21.83초도 exit 0입니다(`/private/tmp/taide-batch18-browser-host.log`, `taide-batch18-browser-wasm.log`). host의 tool 반환이 실행 중 session인 상태에서 다음 Wasm/fmt 명령을 시작해 Cargo 직렬 규칙을 1회 위반했습니다. Wasm 로그의 `Blocking waiting for file lock on build directory`를 확인했고 두 session의 최종 exit 0을 회수했습니다. 검사의 통과와 실행 규칙 준수 여부를 구분하며 이후 명령은 이전 process의 종료를 확인한 뒤 시작합니다.

app/UI fmt와 소유 변경 whitespace 검사 exit 0, remote-web·editor·syntax source/manifest/lock 및 app/UI manifest/lock diff 0입니다. 엔진/의존성은 추가하지 않았습니다. 디스크는 675GiB 여유·64% 사용이며 빌드 산출물을 정리하지 않았습니다. 실제 OS/IME/접근성·pixel/대형/soak/출시 전체 게이트와 theme의 잘못된 7자리 HEX 결정은 미완료로 보존합니다.

## 실제 완료 근거와 Git

구현/검사/QA/상단 PROCESS 21파일을 `2870345d`로 선별 커밋했습니다. `commands-ui-50`(# workspace 검색/취소/목록·선택)과 `commands-ui-60`(해당 팔레트 진입)의 실제 소비자/검사를 연결해 두 행을 완료로 반영했습니다. 599행 원본/ID·36근거 묶음·224경로와 표의 일치 검사가 통과했고 완료 281/588(47.8%), 부분 93·미연결 113·미구현 101입니다. TaskRunner와 검색/Git·완성/hover/정의/구문 접기·실기/출시를 함께 완료로 세지 않습니다. 이미 연결된 파일/아웃라인과 모순된 일부 잔여 설명도 상태를 변경하지 않고 바로잡았습니다.

기능표/완료 근거 3파일을 `a99fb507`로 커밋해 두 커밋을 일반 푸시했고 로컬/원격 0/0을 확인했습니다. 기존 HANDOFF/architecture/합의/PROCESS 하단 11추가·1삭제와 불관련 새 문서를 스테이징하지 않았습니다. 사용자 단독 author·한국어 Conventional Commits·AI 트레일러 없음·선별 staging·일반 push를 유지했습니다. 상단 PROCESS와 batch19 QA에서 구문 접기 공급·수동/Import 명령의 다음 범위를 고정해 전체 목표를 계속합니다.
