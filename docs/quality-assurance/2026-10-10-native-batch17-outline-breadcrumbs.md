# 배치 17 — 아웃라인 패널·경로/심볼 탐색 막대

현재 상태: 아웃라인·경로/심볼 막대·형제 메뉴와 실제 앱/host 수명 연결, 변경한 앱 전체 67대상·648건과 접두사 수정의 관련 12건이 통과했습니다. 서로 다른 app 성공은 649건이며 미해결 실패 0입니다. 기능 대응표 279/588(47.4%), 이번 체크리스트 7/7입니다. 구현 `38ce4725`·근거 `be85b7b7` 일반 푸시와 차이 0/0을 확인했으며 전체 잔여 시간은 미산정입니다.

## 기준과 공급 경계

아웃라인은 실제 `outline-panel-container.tsx`의 현재 프로젝트·창·파일 탭을 사용하고 문서 심볼 공급의 400ms 편집 갱신을 공유합니다. `outline-rows.ts`는 preorder/트리 위치 ID로 같은 이름의 형제를 구별하고 접힌 자손을 목록에서 제외합니다. `outline-panel.tsx`의 행 높이 20px·가상 스크롤·트리 단일 focus·↑↓ 선택/←→ 접기/Enter reveal과 빈 상태 2종을 재현합니다. 클릭도 현재 탭의 selectionRange 시작으로 이동합니다. 종류 아이콘·이름·detail·선택/포커스/테마는 `outline-symbol-row.tsx`를 기준으로 합니다.

`breadcrumbs-bar.tsx`는 파일 본문 위 높이 32px 막대에 상대 경로와 현재 커서의 바깥→안쪽 심볼 체인을 표시합니다. 범위의 양 끝은 포함하고, 같은 수준에서는 원본 순서의 첫 포함 심볼을 고릅니다. 경로 메뉴를 열면 현재 파일의 tree reveal을 통해 형제 행을 준비하며 directory는 비활성, 파일은 현재 창의 집중 pane에서 preview로 열어 1:1로 이동합니다. 심볼 메뉴는 해당 수준의 형제만 표시하며 형제가 하나면 비상호작용입니다. 선택은 기존 현재 탭 UTF-16 reveal을 사용합니다. `breadcrumb-segment.tsx`의 마지막 항목 강조·메뉴·구분자·테마/키보드 focus를 기준으로 합니다.

native 공급은 `editor-symbols.rs`의 프로젝트+문서·세대·revision/언어 gate, 현재 layout/window scope·view caret, 기존 tree_reveal/preview/reveal 큐를 사용합니다. 같은 파일의 다른 프로젝트·다중 pane·readonly 문서·문서/탭/서버 교체·닫힘 뒤 오래된 메뉴 동작을 확인합니다. engine/의존성·새 기능/디자인·OS 합성 입력은 추가하지 않습니다. 별도 검색/Git 패널과 전체 4뷰 완료는 해당 소비자까지 연결될 때 판정합니다.

## 체크리스트

- [x] a. 실제 TS/테마/locale와 native 공급·egui API·위치/상태 대조
- [x] b. outline 트리 행·접기/선택·심볼 체인/형제·경로 모델과 수명
- [x] c. 가상 outline·종류 아이콘·테마·키/마우스·빈 상태
- [x] d. 경로/심볼 막대·메뉴·caret·형제 준비·preview 1:1 이동
- [x] e. 실제 앱·프로젝트/창/pane·tree/문서 수명과 진입/reveal 통합
- [x] f. 의미 있는 회귀·변경 크레이트 전체 대상·동결/포맷/diff·디스크
- [x] g. 실제 근거·기능표·PROCESS·선별 Git·다음 필수 구현

## 검증 상태

`symbol-navigation.rs`의 실제 preorder 위치 ID·같은 이름 형제·자손 접기/부모/형제·양 끝 포함/첫 포함 체인·4096 깊이·relative/루트/직계 자식·루트 밖의 실제 경로 회귀 3건이 통과했습니다(`/private/tmp/taide-batch17-symbol-navigation.log`, exit 0). 원본은 루트 밖 절대경로를 root에 다시 붙이는 잘못된 메뉴 대상을 만들 수 있으므로 native는 실제 절대경로를 보존합니다. 기존 root guard를 우회하지 않습니다.

실제 native egui fork의 `ScrollArea::show_rows/show_viewport`·`Ui::menu_button`·`Popup::menu`·focus lock API와 기존 트리 입력 소유/IME/popup gate를 확인했습니다. 새로운 7개 lucide-react 1.28.0 종류/트리 SVG는 실제 로컬 모듈의 `__iconNode`에서 geometry를 읽었으며 기존 File/Package/Box/Component/Chevron와 Braces를 재사용합니다. 앱 전용 아이콘 표면에서 기존 SVG resolver 차단·DPR/크기 캐시 패턴을 따르며 shared UI/browser 그래프를 변경하지 않습니다.

배치 16의 변경 editor/UI/app 전체 117대상·1187건 성공은 변경 없는 공급 경계의 증거로 재사용합니다. 이후 변경 위험의 관련 검사와 변경 크레이트 전체 대상은 별도로 실행합니다. 보호 Trash 3건·ignored 5건·실기/OS 입력/접근성·대형 성능·출시 부채는 통과로 표시하지 않습니다.

아웃라인 표면 3건(`/private/tmp/taide-batch17-outline-surface-after.log`)과 슬롯/프로젝트·보조 창 집중·진입 버튼 최종 3건(`/private/tmp/taide-batch17-sidebar-selection-final.log`)이 통과했습니다. 5000개 가상 행·접기/펼치기·키/클릭 이동·외부 입력·IME·Tab·두 빈 상태를 확인했습니다. 진입 버튼의 클릭 후 focus 누락과 클릭 프레임 선택색 불일치를 실패 재현한 뒤 명시적 focus 요청·IME gate와 선택을 먼저 확정하는 두 단계 그리기로 수정했습니다. 앱 연결 컴파일(`/private/tmp/taide-batch17-outline-app-check-after.log`)도 exit 0입니다.

막대 UI 6건·host 3건·경로 model 1건의 접두 검사 10건이 통과했습니다(`/private/tmp/taide-batch17-breadcrumb-final.log`, exit 0). 실제 문서/revision/언어/서버 세대·탭/프로젝트/창 슬롯·caret 체인의 메뉴 수명, 경로 tree reveal/직계 파일·비활성 directory·preview 1:1와 기존 탭, 대기 중 소스 변경·루트 밖 대상 거절을 확인했습니다. 5000개 형제는 50개 미만으로 가상 표시하고 End/선택으로 마지막 대상을 이동합니다. 실제 Radix 2.1.24의 clamp 방향키·Home/End·1초 이름 키검색·Tab 처리·Escape trigger focus 복원을 재현하며 IME Preedit/Commit의 Enter 가짜 click을 소비하지 않습니다. 추가한 AccessKit click 회귀도 전체 앱 실행에서 통과했으며 실제 OS 접근성 검사는 아닙니다.

초기 표면 실패는 AccessKit callback 내부의 context 재잠금, Popup 첫 sizing pass의 disabled 행 focus, 이미 열린 trigger의 ArrowDown 재토글, IME Commit의 egui 가짜 click에서 발생했습니다. 각각 잠금 전에 값 계산·활성 행 안에서 focus 요청·이미 열린 메뉴 방향키 소유·pointer/keyboard/접근성 입력 gate로 해결했습니다. headless 텍스처 반환 경고는 테스트에서 FullOutput 텍스처 delta를 명시적으로 회수했습니다. 검사기나 경로 guard를 끄지 않았습니다.

격리된 실제 NativeApplication 검사 1건이 통과했습니다(`/private/tmp/taide-batch17-navigation-app.log`, exit 0, 실행 2.94초). 실제 파일/프로젝트/layout/view 공급에 현재 caret와 typed 심볼을 넣고 아웃라인/본문 막대가 같은 Outer/method를 표시하며 문서 revision을 바꾸지 않음을 확인했습니다. 실제 LSP child 공급·400ms debounce는 변경 없는 배치 16의 증거를 재사용합니다. 본문 bar→readonly→conflict 순서는 실제 `src/widgets/editor-pane/editor-pane.tsx`와 일치합니다. 원본처럼 loading/error의 조기 반환에는 막대를 추가하지 않습니다.

새 7개 SVG의 ISC 고지와 메뉴 키보드/typeahead를 참고한 Radix MIT 원문을 각각 인접 라이선스와 `native/taide-native-app/LICENSE-RADIX-MENU`, 루트 `THIRD_PARTY_LICENSES.md`에 보존했습니다. 앱 전용 파일만 변경하고 shared UI/editor/engine·browser manifest/lock 의존 그래프는 유지합니다.

## 전체 게이트와 최종 실패 수정

변경한 app 크레이트의 전체 기본 테스트 대상을 직접 `--no-fail-fast`로 1회 실행해 67대상·648건 성공·0 실패·0 ignored·보호 Trash 3 filtered를 확인했습니다(`/private/tmp/taide-batch17-app-full.log`, exit 0). 컴파일은 34.52초이며 위 AccessKit click과 실제 앱 소비 검사도 포함됩니다. 명령은 다음과 같습니다.

```sh
cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --no-fail-fast -- --skip remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다 --skip 실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다 --skip 실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다
```

마지막 원본 대조에서 `Alpha/Alpine/Algae`의 현재 Alpha와 맞는 긴 접두사 `al`을 Alpine으로 이동시키는 차이를 발견했습니다. 새 최소 검사 1건은 수정 전 Some(1)/None 불일치로 실패했습니다(`/private/tmp/taide-batch17-typeahead-before.log`, exit 101). 한 글자/반복 키는 순환하고 긴 접두사는 현재 일치를 유지하도록 수정한 뒤 영향 메뉴 접두 검사 12건이 통과했습니다(`/private/tmp/taide-batch17-typeahead-after.log`, exit 0, 실행 0.46초). 전체 성공 결과를 재실행하지 않고 실패 영향만 재검사했으며 서로 다른 최종 app 성공은 649건입니다.

```sh
cargo test --lib --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target breadcrumb
cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target
cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target wasm32-unknown-unknown --features canvas --target-dir experiments/native-shell-spike/target
cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check
```

동결 host 0.10초·Wasm canvas 0.12초(`/private/tmp/taide-batch17-browser-host.log`, `taide-batch17-browser-wasm.log`)와 app fmt(`/private/tmp/taide-batch17-fmt.log`), 소유 변경 whitespace 검사가 모두 exit 0입니다. remote-web·editor/UI/syntax·app manifest/lock의 diff는 0입니다. Cargo/fmt는 한 번에 하나씩 수행했고 디스크는 681GiB 여유·63% 사용입니다. 빌드 산출물을 정리하지 않았습니다.

변경 없는 editor 30대상·201건과 UI inspection 20대상·358건은 batch16 결과를 재사용합니다. 이번 app 67대상·648건과 추가된 1건을 합치면 최신 범위의 서로 다른 성공은 1208건이며, 전체 출시나 모든 원본 분기 통과를 뜻하지 않습니다. 보호 Trash 3과 기존 ignored 5(편집기 1·syntax 3·SDK 문서 1)는 미검증으로 유지합니다. 실제 OS IME/접근성·pixel 상태·대형 지연/soak·패키징은 해당 실기/출시 단계에서 확인합니다.

## 기능표와 Git

`editor-54/71`·`explorer-search-39/40`의 실제 공급/소비 4행을 완료로, `shell-59`·`explorer-search-1`의 파일/아웃라인 2뷰를 부분으로 갱신했습니다. 전체 599행·35근거 묶음·217경로의 원본/ID/분류/표 대조가 exit 0이며 기능 분모 588 중 완료 279(47.4%), 부분 94·미연결 114·미구현 101입니다. 실제 전체 4뷰·실기/출시를 완료로 세지 않습니다.

구현/검사/고지/QA/상단 PROCESS 28파일은 `38ce4725`, 기능표/완료 근거 3파일은 `be85b7b7`로 선별 커밋·일반 푸시해 로컬/원격 차이 0/0을 확인했습니다. 이전 문서 변경과 PROCESS 하단 12줄은 포함하지 않았습니다. 상단 PROCESS에 다음 workspace # 심볼 검색·팔레트 이동의 배치 18 체크리스트를 작성해 전체 목표를 계속 진행합니다.
