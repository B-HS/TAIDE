# 배치 16 — 문서 심볼·팔레트 이동·고정 줄 공급

현재 상태: 원본 대조·모델·typed LSP·팔레트 @·고정 줄 공급과 전체 대상 통합 게이트·선별 Git을 마쳤습니다. 기능 대응표 275/588(46.8%), 이번 체크리스트 7/7입니다. 전체 잔여 시간은 미산정이며 배치 17의 아웃라인 패널·경로/심볼 탐색 막대로 계속 진행합니다.

## 원본과 공급 경계

- TS `src/shared/lib/lsp/adapters/document-symbol.ts`는 documentSymbolProvider capability를 확인하고 nested 결과의 name/detail/kind/range/selectionRange/children, flat 결과의 location.range를 정규화합니다. 알 수 없는 종류는 Variable이며 표시 tags는 비웁니다. Flat 응답의 containerName으로 새 계층을 추측하지 않습니다.
- `document-symbol-session-waiters.ts`는 서버별 실제 파일 root와 프로젝트 fallback을 사용하고, 해당 root의 준비된 capability 지원 세션을 순서대로 선택합니다. 취소/세대가 바뀐 회신은 버립니다. outline/breadcrumb의 편집 갱신은 400ms trailing debounce입니다.
- 팔레트는 현재 파일의 결과만 표시합니다. `command-palette-query.ts`의 preorder 평탄화와 `Class > method` 조상 breadcrumb, NFC fuzzy 이름 검색을 사용합니다. TS symbol-group 아이콘은 종류와 무관한 Braces이며 상세 signature는 표시하지 않습니다. 선택은 현재 파일 탭의 selectionRange 시작으로 reveal한 뒤 닫습니다. 파일 없음/loading/결과 없음은 기존 메시지 카탈로그를 사용합니다.
- Monaco 0.56.0 `stickyScrollModelProvider.js`는 outline → syntax folding → indentation 순입니다. outline이 빈 결과면 folding으로 대체하며 헤더는 range 시작 대신 selectionRange 시작 줄, 끝은 range 끝 줄입니다. 같은 시작 줄의 자식 중복을 제거하고 범위를 정렬합니다. native `StickyModel`과 UI 공급 슬롯은 이미 문서/revision/언어를 확인하며 folding fallback을 보존합니다.
- native LSP는 문서별 mirror와 프로젝트/server/root 키를 보유합니다. SessionClient의 typed documentSymbol 요청과 future drop의 실제 취소 경로를 재사용합니다. 응답 대기는 transient task로 옮겨 UI 및 문서 동기화 actor를 막지 않습니다. 초기화에는 기존 범위에서 hierarchicalDocumentSymbolSupport를 알립니다.

## 체크리스트

- [x] a. 실제 원본/계약과 기존 찾기 RGBA 경계 재현
- [x] b. 문서 심볼 정규화·좌표·수명 모델
- [x] c. typed LSP 공급·준비·취소·회신
- [x] d. 팔레트 @ 목록·입력·현재 탭 reveal
- [x] e. outline 기반 고정 줄 공급과 fallback
- [x] f. 의미 있는 회귀·전체 대상·동결 컴파일·포맷/diff·디스크
- [x] g. 결과·기능표·PROCESS·선별 Git·다음 필수 구현

## 검증 결과

본문 반투명 찾기 색 회귀는 수정 전 0통과/1실패(exit 101, `[103,66,11,126]`/기대 `[208,134,22,126]`)였습니다. 앱/위젯 사이에서는 Color32를 전달하고 장식 생성 시 unmultiplied RGBA로 바꾼 뒤 editor-find 16건이 통과(exit 0)했습니다. 로그는 `/private/tmp/taide-batch16-find-body-before.log`, `/private/tmp/taide-batch16-find-body-after.log`입니다.

새 문서 심볼 코어 3건이 통과(exit 0)했습니다. nested/preorder/breadcrumb/UTF-16/태그·flat 동일 URI/빈 fallback·잘못된 범위/반쪽 surrogate/문서·언어·편집 만료를 확인했습니다. 로그는 `/private/tmp/taide-batch16-symbol-core.log`입니다. 기존 LSP 타입의 deprecated 생성 필드에서 테스트 경고 2개가 출력됐습니다.

팔레트 기존 28건과 새 symbol 1건, 고정 줄 16건이 통과했습니다. 팔레트 새 검사는 기존 영어 카탈로그를 `No active file`로 잘못 기대해 최초 1실패였으며 실제 `Open a file first`로 수정한 해당 1건이 통과했습니다. 로그는 `/private/tmp/taide-batch16-symbol-palette.log`, `/private/tmp/taide-batch16-symbol-palette-after.log`, `/private/tmp/taide-batch16-symbol-sticky.log`입니다. 실제 공급 모델의 selectionRange 시작 줄이 표시되고 빈 결과·편집 뒤에는 접기 fallback을 사용합니다.

같은 문서를 두 프로젝트에서 관찰하면 첫 요청이 취소되는 실패를 재현했습니다(`/private/tmp/taide-batch16-symbol-project-before.log`, 0통과/1실패·exit 101). 심볼 상태와 LSP 동기화의 키를 프로젝트+문서로 바꾸고 실제 화면의 프로젝트를 mirror에 사용하도록 수정했습니다. 한 프로젝트 binding만 닫을 때 다른 프로젝트 mirror를 보존합니다.

최종 앱 상태 5건·실제 child 3건, 총 8건이 통과했습니다(`/private/tmp/taide-batch16-symbol-shared-child.log`, exit 0). 계층/flat/오류/미지원·최신 didChange·owner/서버/등록 세대·readonly UTF-16 reveal·400ms debounce·취소/닫힘/늦은 회신·두 프로젝트 mirror/owner/선별 닫힘을 확인했습니다. 최초 취소 검사는 같은 문서 didChange의 자동 취소 이벤트를 앞 단계에서 소모해 대기 시간이 초과됐습니다. 응답을 보류한 상태에서 다른 문서의 동기화를 확인한 뒤 명시적 취소를 확인하도록 검사를 수정했고 실제 서버 취소 알림과 회신 미적용이 통과했습니다.

원본 종류와 tag를 모델에 보존하되 현재 팔레트는 원본처럼 Braces와 이름/조상 경로만 표시합니다. Flat 외부 URI와 잘못된 UTF-16 범위는 다른 문서에 적용하지 않습니다. 새 엔진/의존성·browser 소스/manifest/lockfile 변경은 없습니다. 여러 outline 공급자는 최대 범위·기존 선호 owner를 사용하며 palette는 첫 지원 provider 결과를 사용합니다.

변경 editor 전체 30대상·201건 통과/0실패/1 ignored, UI inspection 전체 20대상·358건 통과/0실패, 앱 전체 67대상·628건 통과/0실패/보호 Trash 3건 제외를 확인했습니다. 합계 117대상·서로 다른 1187건 통과입니다. 각 크레이트의 전체 대상을 Cargo 직렬로 직접 `--no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target`로 실행했습니다. UI에는 `--features inspection`을 사용했습니다. 앱 전체 첫 명령은 기존 `tests/lsp.rs`가 새 DocumentSymbols 응답을 분기하지 않아 컴파일 오류(exit 101)로 종료했습니다. 저장 참여자 검사에서 요청하지 않은 심볼 응답을 명시적으로 거절하도록 보완한 뒤 전체 실행이 통과했습니다. 로그는 `/private/tmp/taide-batch16-editor-full.log`, `/private/tmp/taide-batch16-ui-full.log`, `/private/tmp/taide-batch16-app-full.log`, `/private/tmp/taide-batch16-app-full-after.log`입니다.

동결 browser host `--tests` 컴파일 4.76초와 Wasm `--target wasm32-unknown-unknown --features canvas` 컴파일 2.89초가 exit 0입니다. 로그는 `/private/tmp/taide-batch16-browser-host.log`, `/private/tmp/taide-batch16-browser-wasm.log`입니다. 변경 editor/UI/app `cargo fmt --check`, 소유 코드/고지 diff 검사가 exit 0이며 native manifest/lockfile·remote-web 변경 경로 0개입니다. 디스크 여유 689GiB·사용률 63%입니다. 앱에는 기존 wry deprecated/unsafe 경고 17개와 debug 링크의 unwind 표 경고가 출력됐습니다. 검사기나 경고를 끄지 않았습니다.

변경 없는 syntax는 batch13 전체 159건/3 ignored, SDK는 batch12 단위 52·문서 167건/문서 1 ignored의 성공 증거를 재사용합니다. 이번 editor 성능 ignored 1을 합한 ignored 5건과 보호 Trash 3건은 통과로 세지 않습니다. 본문 찾기 색 수정은 별도 논리 단위이며 `../bug/2026-10-10-native-find-decoration-rgba.md`에 재현·검증을 기록했습니다.

본문 찾기 색 수정과 회귀·기록 4파일을 `641ff334`, 심볼 구현/회귀/고지/QA와 PROCESS 상단 22파일을 `112120eb`로 선별 커밋·일반 푸시했습니다. 로컬/원격 차이 0/0을 확인했습니다. 이전 세션의 관련 없는 문서 변경은 제외했습니다. 실제 @ 심볼 이동 1행만 완료로 갱신하고 문서 심볼/breadcrumb·explorer 호출과 sticky의 남은 소비자는 부분으로 보존했습니다. 현재 기능표는 완료 275/588(46.8%), 부분 94·미연결 114·미구현 105이며 599행·34근거 묶음·203경로를 검사했습니다. 전체 구조·실기/성능·출시 게이트를 유지하고 배치 17로 계속 진행합니다.

실제 OS 입력/IME·접근성·대형 파일/성능·패키징은 미검증이며 보호 Trash 3개와 ignored 검사 부채를 유지합니다.
