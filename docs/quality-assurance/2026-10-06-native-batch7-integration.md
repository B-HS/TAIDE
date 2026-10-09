# Native 전환 배치 7 통합 검증

검증일: 2026-10-09. 기준 체크포인트 `fbc28b84`, 재개 HEAD `bf5175ca`, 브랜치 `to_rust_native`. 메인이 직접 직렬 구현·검증했으며 서브에이전트와 workflow를 사용하지 않았습니다. 배치 7 구현을 마무리하고 기록·커밋·푸시 뒤 중단합니다. 배치 8 이후는 미착수입니다.

## 변경 결과

1. `native/taide-native-editor/src/auto-indent.rs`: 닫는 괄호 내어쓰기의 여는 줄 탐색에서 String·Comment·Regex의 괄호를 제외하고 Other만 셉니다. 실패 테스트 3건과 Other 대조를 먼저 실행했습니다. 수정·회귀 테스트·언어 구성 QA는 `3a011d35`로 분리 커밋했습니다.
2. `native/taide-native-editor/src/line-commands.rs`, `native/taide-native-syntax/src/text-transforms.rs`, `native/taide-native-app/src/editor-command-text.rs`: 줄 이동·복사·삭제·삽입·합치기·들여쓰기·정렬·중복 제거·역순·대소문자·주석·괄호 제거·transpose 등 문서 명령 34개를 연결했습니다. 기존 앱 ICU 정렬과 앱 전용 syntax의 단어/문자 변환을 공급합니다. transpose의 잘못된 UTF-16 단위는 사용자 결정대로 U+FFFD로 변환하며 유효한 쌍은 보존합니다.
3. `native/taide-native-editor/src/cursor-commands.rs`, `bracket-navigation.rs`, `view.rs`, `store.rs`: 커서·선택 명령 23개, 중복/겹침 정규화, 커서 이력·스크롤·anchor 추적, 목표 표시 열 유지, 괄호 이동·스마트 선택·일치 선택과 드러내기 요청을 연결했습니다. 문서 변경은 커서 이력과 선택 캐시를 초기화합니다.
4. `native/taide-native-ui/src/editor-pointer.rs`, `editor_surface.rs`, `editor-paint.rs`, `editor-gutter.rs`, `command-registry.rs`, `keymap.rs`와 app 큐: Alt 커서·컬럼 드래그·다중 클릭·단위 드래그, 다중 커서 타이핑·붙여넣기·삭제·Enter·자동 닫기·IME, 원본 기본 키·chord·재지정·읽기 전용과 큐를 결합했습니다. 실제 화면에 합성 키/마우스를 보내지 않았습니다.

상세는 `2026-10-06-native-batch7-language-config.md`, `2026-10-06-native-batch7-line-commands.md`, `2026-10-06-native-batch7-cursor-commands.md`와 `docs/acknowledge/2026-10-09-native-transpose-utf16.md`입니다. 기존 TS·Tauri·Monaco·xterm 소스는 보존합니다.

## 원본 기준값과 관련 검증

| 검증 대상 | 실제 결과 | 범위 |
| --- | --- | --- |
| 줄·주석·들여쓰기·복사 내용 | 11,730개 일치 | 23언어, 실제 Monaco 명령·TextModel 범위 검증·PieceTree 편집 |
| 구성 단어 범위 | 1,656개 일치 | 원본 getWordAtText, UTF-16 1,000단위 창 |
| 대소문자 7종 | 182개 일치 | 원본 `_modifyText`, Unicode·약어·CRLF·astral |
| 괄호·스마트 선택 | 2,760개 일치 | 원본 괄호 파서·word/bracket provider, orphan opener·예상 밖 closer |
| 리터럴 검색 / transpose | 각각 48개 일치 | 원본 검색의 Unicode·CRLF / UTF-16 역순·선택 추적·U+FFFD 정책 |

원본 실행 도구는 `docs/utils/2026-10-09-monaco-{line,case,cursor,transpose}-oracle.js`와 `2026-10-09-monaco-editor-keybindings.js`입니다. 기준값은 syntax fixture와 UI 기본 키 JSON에 저장했습니다. 줄 내용 기준값은 모든 선택 상태를 전수 검증했다는 뜻이 아닙니다. 스마트 선택에는 LSP provider가 포함되지 않습니다. 이전 문서의 363/433·212/573을 현재 기능 대응률로 사용하지 않습니다.

## 전체 테스트 대상 실행

공통 옵션은 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 다음 전체 대상을 크레이트별로 한 번씩 직접 실행했고 실패해도 다음 대상을 실행하도록 `--no-fail-fast`를 사용했습니다. 단일 Cargo 프로세스만 실행했습니다.

```sh
cargo test --manifest-path native/taide-native-editor/Cargo.toml --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
cargo test --manifest-path native/taide-native-syntax/Cargo.toml --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
cargo test --manifest-path native/taide-native-app/Cargo.toml --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
```

| 크레이트 | 통과 | 실패 | ignored | 종료 코드 |
| --- | ---: | ---: | ---: | ---: |
| editor | 175 | 0 | 1 | 0 |
| syntax | 147 | 0 | 3 | 0 |
| UI (`inspection`) | 257 | 0 | 0 | 0 |
| app, 전체 최초 실행 | 581 | 31 | 0 | 101 |

app 전체의 실패 31건 중 2건은 이번 동작 변화에 따른 기존 기대값, 25건은 loopback 서버·ImageIO·OS watcher의 실행 권한 제한, 3건은 실제 휴지통 호출 제한입니다. 나머지 1건은 아래의 알려진 XLML 실패입니다. 변경 관련 2건은 수정했으며 안전한 실패 항목 27건을 선별 재검사해 모두 통과했습니다. 전체 결과와 재검사 결과를 합치면 **서로 다른 app 검사 608건 통과, XLML 1건 실패, 보호 대상 검사 3건 검증 미완료**입니다. app 전체 통과 판정은 아닙니다.

재검사는 전체 성공 대상을 반복하지 않았습니다. `--lib -- --exact`로 실패한 20개 이름만 실행해 20 통과·351 filtered out·exit 0입니다. 여기에는 명령 실행 가능 집합과 zen Escape 기대값 수정 2건, 임시 데이터만 사용하는 로컬 서버 검사 18건이 포함됩니다. 별도로 `preview-macos`, `preview-web-http`, `preview-web-http-lifetime`, `preview-web-lazy-host`, `preview-web-media`, `preview-web-served`, `projects`의 실패 이름 7개만 같은 방식으로 실행해 7 통과·exit 0입니다. 실제 실패 이름과 재현 옵션은 `docs/utils/2026-10-09-native-batch7-recheck.json`에 보존합니다. 이 선별 재검사만 제한 밖 실행을 허용받았으며 휴지통 호출은 포함하지 않았습니다.

### 알려진 실패와 보호 대상

- `native/taide-native-app/tests/preview-spreadsheet-xlml.rs:94`: `xlml은_할당전_grid_합산_인덱스와_xml_보안_경계를_거절한다`가 `<Row ss:Index="0"><Cell/></Row>`를 수락해 실패합니다. 이전에 알려진 배치 5 부채이며 이번 편집 명령 범위 밖이라 수정하지 않았습니다.
- `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`, `tests/explorer-delete.rs`의 실제 삭제 검사, `tests/lsp-workspace-worker.rs`의 실제 workspace 삭제 검사: 합성 임시 파일에 대한 `trashItemAtURL` 호출이 권한 제한으로 거절됐습니다. 사용자 지시의 Trash 보호를 유지해 제한 밖에서 재실행하지 않았습니다. 휴지통 접근을 허용하는 별도 검증 범위가 확정될 때 확인해야 합니다.
- editor ignored 1건은 수십만 줄 wrap 성능 기록, syntax ignored 3건은 만 줄 토큰화·큰 문법 첫 지연·한도 근처 긴 줄 성능 기록입니다. 자동으로 실행했다고 보고하지 않습니다.

### 전체 게이트에서 발견한 기대값 수정

editor 예비 전체 검사에서는 중복 선택 병합과 겹치는 snippet 요청의 입력 검증 순서 때문에 2대상이 실패했습니다. UI 예비 검사에서는 빠른 재클릭의 더블클릭 해석, gutter의 줄 번호/장식 영역 구분, 인접 snippet mirror 병합의 session 취소 때문에 2대상이 실패했습니다. 원본 규칙과 맞춰 수정한 뒤 관련 대상을 재검사하고, 위의 최종 editor·syntax·UI 전체 게이트를 실행했습니다. app의 실행 가능 집합 검사는 새 명령의 포함과 미연결 명령의 제외를 확인하도록 수정했고, zen Escape 검사는 처음부터 겹치지 않는 커서를 주어 단계별 Escape 동작을 유지했습니다. 검사기를 끄거나 실패를 ignored로 바꾸지 않았습니다.

## 컴파일·포맷·변경 범위·디스크

- 재개 직후 app·syntax·editor·UI·remote-web 5개에 `cargo check --tests`를 공통 옵션으로 순차 실행해 모두 exit 0입니다. mock LSP example도 빌드했습니다. 마지막으로 remote-web의 같은 컴파일 검사를 다시 실행해 exit 0, Cargo 표시 시간 4.24초입니다. remote-web source·feature·의존 그래프·lockfile은 변경하지 않았습니다.
- 변경 4개 크레이트의 `cargo fmt --manifest-path ... --check`가 각각 exit 0입니다. app의 `fmt --all`은 기존 vendored eframe workspace metadata 때문에 실패해 해당 크레이트만 포맷하고 검사했습니다. 의존 크레이트에 생긴 무관한 import/module 재정렬 두 곳은 원래대로 복구했습니다. JS 도구 5개와 기본 키 JSON의 명시적 Prettier 검사는 exit 0이며 관련 diff 공백 검사도 exit 0입니다.
- Cargo.toml·Cargo.lock·remote-web·기존 crates에는 이번 작업 diff가 없습니다. ferriki-textmate 0.12.0 + ferroni 1.8.1을 유지하며 새 의존성을 추가하지 않았습니다. 기존 vendored wry 경고는 별도 부채로 유지합니다.
- `df -h /Users/hyunseokbyun/development/TAIDE`: 여유 **701GiB**, 사용률 **62%**입니다. 빌드 캐시를 삭제하지 않았으며 보호한 Egui/Iced Spike 앱 번들, 실제 앱 데이터, OS 설정·클립보드·Keychain은 수정하지 않았습니다.

## 실기 검증 부채와 후속 경계

실제 OS IME·dead key·접근성 입력, RTL 레이아웃, 대형 파일 응답성·메모리, 모든 겹치는 다중 편집 조합과 모든 선택 위치 조합은 미검증입니다. 대소문자의 tr·az·lt locale 특수 규칙도 미검증이며 일반 Unicode 기준값 결과만 보고합니다. 토큰 워커 지연에서의 실제 구문 상태, LSP selectionRange 결합은 해당 후속 작업에서 검증해야 합니다. 실행 시점은 사용자님이 그 배치 또는 실기 범위를 지시한 뒤입니다.

찾기 정규식 엔진, 번들 테마 7자리 색 값 등 HANDOFF 6절의 결정 대기는 유지합니다. 배치 8 이후 구현, TS 제거와 전체 전환 통과 선언은 하지 않습니다. 재개 전부터 있던 HANDOFF·architecture·기존 결정 문서와 이전 feedback/memory 변경은 이번 커밋에서 제외합니다.
