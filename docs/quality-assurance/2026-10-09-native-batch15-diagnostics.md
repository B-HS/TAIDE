# 배치 15 — 편집기 진단·문제 이동·overview ruler

현재 상태: 진단 표시·문제 이동·overview/미니맵의 구현과 전체 대상/동결 컴파일/포맷 게이트를 마쳤습니다. 기능 대응표 기준 272/588(46.3%), 이번 체크리스트 6/7입니다. 결과 기록과 선별 Git을 진행하며 전체 잔여 시간은 미산정입니다.

## 원본과 구현 경계

- `src/shared/lib/lsp/adapters/diagnostics.ts`: owner는 서버명이 아닌 client/session별입니다. 정규화된 URI별 원본 code/data/source를 보관하고 모델 폐기/세션 폐기 때 해당 owner만 제거합니다. severity 누락/알 수 없는 값은 Error, Warning/Information/Hint는 각각 대응합니다. marker로 전달하는 필드는 range/severity/message/source/문자열 code이며 tags/relatedInformation/codeDescription은 전달하지 않습니다. 이 배치에서 원본 raw의 태그를 보존하되 표시 동작을 추가하지 않습니다.
- `src/features/editor/code-editor.tsx`: glyphMargin=false, renderValidationDecorations 별도 설정 없음, minimap은 대형 파일에서 꺼집니다. installed Monaco `editorOptions.js`의 기본 editable은 readOnly에서 validation 표시를 숨깁니다. 진단 메시지 읽기/이동은 쓰기 동작이 아닙니다.
- Monaco `common/services/markerDecorationsService.js`: Hint는 같은 줄의 시작 뒤 UTF-16 2단위로 표시하고 Error/Warning/Info는 squiggly 및 z=30/20/10, Hint z=0입니다. 빈 범위는 가능한 단어로 확장하며 빈 줄/줄 끝은 1ch 폭입니다. 장식 anchor는 NeverGrowsWhenTypingAtEdges, overview Right, minimap Inline, Hint의 overview/minimap 색은 없습니다. 표시량 상한은 원본 500을 사용하되 원본 전체 raw/문제 목록을 잃지 않습니다.
- Monaco `contrib/hover/browser/markerHoverParticipant.js`: 메시지를 텍스트로 표시하고 source(code)를 이어 표시합니다. 진단 hover는 LSP 일반 hover와 별개입니다. quick fix/AI 메뉴 공급은 별도 미구현 LSP 범위이며 가짜 성공/임의 동작을 만들지 않습니다.
- Monaco `contrib/gotoError/browser/{gotoError,markerNavigationService}.js`: 현재 파일 Alt+F8/Shift+Alt+F8, 파일 간 F8/Shift+F8. Error/Warning/Info 대상, Hint 제외, 파일 URI 순서와 기본 severity→범위 순서, 최초 커서 위치/반복 이동/끝 순환, 메시지 위젯과 reveal입니다.
- `src/shared/lib/monaco/theme.ts`: 밑줄/문제 위젯 상태색은 statusIndicator.error/warning/info, hover 배경 editor.hoverBackground/전경 editor.foreground/테두리 editor.widgetBorder입니다. overview/minimap Error는 Monaco 기본 #ff1212b3, Warning/Info는 상태색, Hint는 dark #eeeeee·0.7/light #6c6c6c 기본입니다. overview border 기본 #7f7f7f4d, near 괄호 center 기본 #a0a0a0, 찾기 center 색은 원본 기본값을 확인해 공급합니다.
- 실제 Monaco `browser/viewParts/overviewRuler/decorationsOverviewRuler.js`: view row의 vertical offset/scrollHeight를 ruler 높이에 투영하고 최소 6 CSS px를 확보합니다. 같은 색/lane의 인접 표식은 병합합니다. 세 lane/스크롤bar 관계·wrap/접기/scrollBeyondLastLine/DPR/clip을 유지합니다. 앞서 읽은 별도 overviewZoneManager의 4px를 이 표면의 기준으로 사용하지 않습니다.
- native 앱 `lsp.rs`: protocol_revision/version 검사와 모델 revision을 담은 Reply, raw store/owner별 marker_bindings와 실제 모델 폐기 경로가 이미 있습니다. `application.rs`는 현재 snapshot revision이 Reply와 같을 때만 publish합니다. `diagnostics.rs`는 raw/count/problem panel 공급만 있고 본문 장식이 없으며 같은 raw의 새 revision 재발행을 구분하지 않습니다.
- native editor `lsp.rs`는 UTF-16/byte 경계를 검사하며 `DecorationLayer`와 `EditorStore::changes_since`가 편집 anchor 추적/만료를 제공합니다. 기존 range 엔진에 새 구문/정규식 의존성을 추가하지 않습니다.
- native UI `editor_surface.rs`의 실제 Row/DisplayMap/clip, `editor-paint.rs`의 squiggly, `editor-minimap.rs`의 선택 overlay, `editor-overlay.rs`의 위치 계산을 사용합니다. 새 UI 공급은 native-host 조건으로 제한해 frozen browser 소스/기능/의존 그래프/manifest/lock을 보존합니다.
- egui 0.36.2 실제 fork `vendor/egui-input/src/containers/tooltip.rs`의 Tooltip/Popup와 기존 memory egui 테스트를 읽었습니다. 실제 OS 합성 입력이나 사용자 앱 데이터/클립보드/Keychain/Trash를 사용하지 않습니다.

## 체크리스트

- [x] a. 원본·API·공급·수명 경계 확인
- [x] b. 문서/revision/owner 기반 공급과 실패 재현
- [x] c. 본문 진단 밑줄·메시지 hover
- [x] d. 문제 이동·명령·기본 키·메시지 reveal
- [x] e. overview/미니맵의 실제 진단·찾기·near 괄호 공급
- [x] f. 의미 있는 회귀·변경 크레이트 전체 대상·동결 컴파일·포맷/diff/디스크
- [ ] g. 결과/기능표/PROCESS·선별 커밋·일반 푸시·다음 구현 계속

## 실제 검증

- core `cargo test --test diagnostics` 3건 통과입니다. 실제 installed Monaco 0.56.0 범위 684표본이 일치합니다. 잘못된 surrogate 끝 범위는 먼저 실패(0..1 vs 0..5)를 재현한 후 수정했습니다. fixture 생성은 실제 TextModel/MarkerDecorationsService를 사용하며 새 의존성은 없습니다.
- 앱 `cargo test --lib diagnostics::tests` 4건 통과입니다. 같은 raw의 새 revision 재발행이 기존 batch를 재사용하는 실패를 먼저 재현한 후 수정했습니다. 원본 raw/data/code 보존, owner별 폐기, 언어/문서 폐기, 편집 anchor와 표시 캐시를 검증했습니다. 기존 LSP Reply gate는 변경하지 않았습니다.
- UI `editor-display` 44건·`editor-find` 15건·`editor-minimap` 17건·`command_registry` 8건이 통과했습니다. 정지 포인터와 메시지 내부 진입/이탈, fold 숨김/펼침, readonly·IME·Space 해제·외부 입력창·본문과 문제 이동의 직렬 이벤트, wrap/접기/view zone/DPR 투영과 같은 범위의 여러 owner hover를 확인했습니다. hover 검사에서 매 프레임의 동일 PointerMoved가 대기 시간을 초기화하는 오류를 수정했습니다.
- 앱 `editor_problems::tests` 5건이 통과했습니다. 현재 파일/파일 간 이동·severity 순서/Hint 제외·readonly/다중 선택·독립 뷰·UTF-16 좌표·편집 anchor·요청 토큰·닫힘/owner 폐기·현재 layout/보조 창 입력원과 대상 탭 확인을 검증했습니다. HostBridge의 실제 dispatch는 기존 직렬 actor를 사용합니다. 새 요청은 preview 탭을 열고 응답 시 같은 layout과 활성 대상 탭일 때만 적용합니다.
- editor 전체 29대상·198건(ignored 1), UI inspection 전체 20대상·354건, app 전체 67대상·620건(보호 Trash 3 filtered)이 통과했습니다. 전체 UI 실행 뒤 같은 범위 여러 owner hover 중복을 추가 재현해 수정하고 영향 대상 `editor-display` 44건을 재실행했습니다. 서로 다른 UI 검사 합계는 355건이며 변경 크레이트 합계는 116대상·1173건입니다. 입력·환경·코드가 같은 이전 성공 검사를 반복하지 않습니다.

## 실패 재현과 수정

- 같은 입력 묶음의 닫기 클릭/F8이 `[Next, NextInFiles, Close]`로 뒤집혔습니다. SDK의 실제 hit-test가 승인한 raw pointer index와 이전 프레임 버튼 정보를 사용해 `[Next, Close, NextInFiles]` 순서로 실행합니다. 버튼 Enter/Space는 소유자별 이벤트 경로에서 처리하며 외부 입력창의 문자/F8은 소비하지 않습니다.
- 열린 메시지 위젯을 무시한 외부 reveal의 대상 caret이 화면 밖이라 geometry가 None이었습니다. 문서/언어/편집 journal로 추적한 위젯 위치와 예약 높이를 같은 VerticalLayout에 포함해 실제 화면 중앙에 드러냅니다.
- 이미 비활성인 응답 대상 탭에도 새 뷰가 붙었습니다. 원본 요청 토큰/뷰·대상 path/project/viewport 외에 실제 활성 탭과 현재 layout을 확인하며 늦은 응답을 버립니다.
- 같은 lane/색의 근접 괄호 표식 2개가 따로 그려졌습니다. 실제 원본과 같이 겹치거나 인접한 범위를 병합합니다. 빈 줄 미니맵 진단 배경 0건은 원본 행 배경을 공급해 1건으로 수정했습니다.
- 반투명 찾기 색은 premultiplied `[103,66,11,126]`를 unmultiplied로 다시 해석해 어두워졌습니다. 장식 경계에 unmultiplied `[208,134,22,126]`를 저장해 추가 alpha 곱셈을 제거했습니다.
- 버튼 입력 처리의 첫 수정 중 egui input read 안에서 Context를 다시 읽어 10초 잠금 실패가 발생했습니다. raw 이벤트 수를 먼저 읽고 SDK hit-test 조회를 밖에서 실행하며 변경 상태의 전체 표시 회귀를 재실행했습니다.
- 같은 범위의 여러 owner가 각각 같은 hover 앵커에 Tooltip을 추가해 메시지가 두 번씩 표시됐습니다. 같은 표시 행/범위의 앵커를 한 번만 등록하고 해당 위치의 모든 진단을 한 메시지 영역에 표시합니다. 두 메시지가 각각 2건에서 1건으로 바뀌는 실패/성공을 검증했습니다.

## 구현 범위와 차이

- 원본 adapter가 tags/relatedInformation/codeDescription을 marker에 전달하지 않아 불필요/deprecated/관련 위치 UI를 임의 추가하지 않습니다. 진단 패널의 raw/code/data와 owner별 폐기는 유지합니다.
- 메시지 위젯은 원본 markerNavigation/Peek/ZoneWidget처럼 높이만 예약하며 Source 기본 옵션에 없는 resize 기능을 추가하지 않습니다. 원본과 같은 title·개수·다음/이전/닫기·선택 가능한 메시지/source(code)를 표시합니다.
- 원본의 파일 끝 탐색에서 전역 첫 파일로 건너뛰는 특정 초기 선택 결함과 Standalone opener의 null 반환 때문에 대상 위젯이 사라지는 결함은 강제하지 않습니다. 사용자의 원본 버그 강제 재현 금지에 따라 다음 파일로 일관되게 이동하고 대상 뷰에 메시지를 붙입니다.
- 엔진/의존성/manifest/lockfile과 frozen remote-web 소스는 변경하지 않습니다. OS 합성 입력·실제 앱 데이터·클립보드·Keychain·Trash를 사용하지 않습니다.

## 통합 명령과 증거

변경 editor/UI inspection/app에 `cargo test --no-fail-fast --manifest-path native/<crate>/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`를 직렬로 1회 실행했습니다. UI는 `--features inspection`, app은 기존 보호 Trash 3건만 `--skip`했습니다. 로그는 `/private/tmp/taide-batch15-{editor,ui,app}-full-20261010.log`이며 모두 exit 0입니다. UI 추가 수정의 영향 검사 로그는 `/private/tmp/taide-batch15-hover-overlap-after.log`(44건, exit 0)입니다.

변경 없는 syntax는 batch13 전체 12대상·159건·ignored 3, SDK는 batch12 단위 52/문서 167·문서 ignored 1 근거를 재사용합니다. 현재 editor ignored 1과 합쳐 ignored 5건이며 통과 수에 포함하지 않습니다. 실제 OS IME/접근성·대형 성능·soak·패키징/출시와 미구현 LSP/SCM 공급은 별도 잔여입니다. XLML 4건은 이번 app 전체 대상에서도 통과했으며 이전 실패는 batch8에서 해소됐습니다.

`cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`와 같은 명령에 `--target wasm32-unknown-unknown`을 추가한 동결 컴파일은 각각 exit 0(3.01s/17.06s)입니다. 기존 browser Wasm 경고 6건은 동결 소스의 기존 항목이며 변경하지 않습니다. 보호 Source/manifest/lockfile diff는 없습니다.

변경 세 패키지 `cargo fmt --package <crate> --check`, 새 QA/JS Prettier check와 변경 범위 `git diff --check`가 exit 0입니다. Cargo/fmt는 모두 직렬로 실행했습니다. 디스크 여유는 695GiB·사용 63%, 공유 target은 79GiB이며 빌드 산출물을 삭제하지 않았습니다.
