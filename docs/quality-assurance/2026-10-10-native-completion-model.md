# 자동완성 후보·필터·삽입 모델 검증

현재 상태: 배치 22의 후보·삽입·미리보기 모델을 검증했습니다. UI/App 소비자는 작업 트리에 연결해 회귀를 검사했고 배치 전체 게이트는 남아 있습니다. 이 문서의 표본·체크리스트 비율을 전체 기능 대응률로 환산하지 않습니다. 전체 전환율·잔여시간은 미산정입니다.

## 계약과 범위

실제 TS completion adapter·Monaco completionModel/filters와 기존 native snippet parser/insertion/session을 기준으로 구현했습니다. 외부 원본은 기존 node_modules/monaco-editor 0.56.0의 실제 소스이며 원본 비교 도구와 표본을 저장합니다. 버그·내부 수치를 강제로 재현하지 않는 사용자 지시를 적용합니다.

Candidate는 typed CompletionItem의 opaque data·문서/범위/종류/설명을 보존하고 UTF-16 위치·단일 줄 삽입/치환 범위·현재 커서 포함을 검증합니다. 잘못된 개별 후보는 제외하고 알려지지 않은 종류는 Text로 표시합니다. 수락 준비는 문서 ID/revision·주 커서·다중 커서의 같은 접두/접미사·readonly·범위와 총량을 검증합니다. 일반 텍스트의 달러 문법은 그대로 보존하며 스니펫만 parse/변수/transform·각 삽입 위치의 들여쓰기/EOL을 평가합니다. 준비 중 실패로 문서를 변경하지 않습니다.

필터는 UTF-16 DP·단어 경계/camel/separator·첫 일치 강도·인접 오타·filterText와 label의 별도 강조를 처리합니다. 초기 정렬은 원본 label/kind와 sortText 계약을 소비하며 원본의 비추이 비교기를 총순서로 고쳤습니다. 현재 입력이 연장되면 이전 후보를 다시 필터링하고 줄 내용/커서 변화는 새 계산을 수행합니다. 캐시와 입력 크기를 제한하며 editor/UI에 정규식·구문 엔진 의존성을 추가하지 않습니다.

## 실행 근거

모든 명령은 --locked --offline --target-dir experiments/native-shell-spike/target를 사용했습니다. 같은 상태의 성공 결과는 재사용합니다.

- core 후보/범위: cargo test --manifest-path native/taide-native-editor/Cargo.toml --test completion, completion-ranges-after-generation.log exit 0·9통과
- core 삽입: 같은 crate의 --test completion-insertion, completion-insertion-after-session-cost.log exit 0·5통과
- fuzzy 필터: --test completion-filter, completion-filter-after-fixture.log exit 0·1통과이며 실제 원본 8100표본이 일치합니다
- 목록 모델: --test completion-model 전체 4건이 completion-model-final.log에서 exit 0으로 통과했습니다. completion-ranking-first.log의 3통과/1실패 뒤 completion-ranking-after-prefix-filter.log에서 실패한 문맥을 통과했고 실제 원본 17문맥과 대조합니다. 0건 실행인 completion-ranking-after-prefix.log는 성공 근거에 포함하지 않습니다.
- 새 UI의 typed 종류/수락·Unicode 라벨/Deprecated 소비: completion-ui-final.log exit 0·11통과입니다. 이 중 모델 소비 근거만 재사용하며 실제 앱 연결 완료로 판정하지 않습니다.

로그 경로는 /private/tmp/taide-batch22- 접두사입니다. editor fmt와 소유된 diff check는 exit 0입니다. 원본 추출 도구는 docs/utils의 completion-filter-oracle.js·completion-model-oracle.js, 표본은 editor/tests/fixtures의 completion-filter-reference.tsv·completion-model-reference.tsv입니다.

전체 대상 실행·실제 Body/peek 후보 요청과 갱신 소비·preview·snippet session/nested 삽입과 최종 기능 완료 판정은 배치 22 QA와 PROCESS의 c–g에 남아 있습니다.

## 편집 후 후보 범위 갱신

Candidate.rebased와 Model.rebase는 기존 변경 journal과 장식 범위 추적을 소비합니다. 같은 transaction의 여러 편집은 이전 문서 좌표의 역순으로 적용하고 Unicode 경계·단일 줄·현재 커서 포함·문서/revision·읽기 전용을 다시 검사합니다. 모델은 모든 후보의 범위를 먼저 검사한 뒤 함께 갱신하므로 후속 후보가 실패해도 일부만 갱신하지 않습니다. 원본 요청의 필터 길이와 초기 정렬은 유지하고 현재 입력 delta로 다시 필터링합니다.

cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test completion-model completion_rebase는 /private/tmp/taide-batch22-completion-rebase-tests.log에서 exit 0·2통과입니다. 한글/이모지 앞쪽 편집과 동시 입력·삭제, insert/replace의 접미사 범위, 줄 경계를 넘는 후보의 원자적 거절과 유실 journal을 검사했습니다. 앱 completion_cache-first.log의 실제 입력 후 목록 소멸 1실패를 먼저 재현했으며 앱 캐시의 안정된 표시 token·불완전 provider 재요청·늦은 응답 거절은 배치 22 QA에 별도 기록합니다. editor fmt exit 0이며 전체 대상 게이트는 아직 실행하지 않았습니다.

선별 스테이징의 diff check가 TSV의 빈 마지막 강조 목록 필드 때문에 trailing whitespace를 보고했습니다. 빈 목록을 명시적인 none 필드로 직렬화하도록 원본 추출 도구와 검사 decoder를 함께 바꿔 8100표본을 재생성했고 전체 필터 검사 1건이 다시 통과했습니다. 검사기를 끄거나 의미 있는 구분자를 임의로 trim하지 않았습니다.

## 기본 입력의 스니펫 추적

snippet-default-input-first.log의 기본 입력 검사는 exit 101·1통과/1실패였으며 여러 revision의 한글/이모지 mirror 입력 뒤 세션이 만료됐습니다. journal span에 기존 바이트 좌표와 함께 이전 문서 기준 UTF-16 시작/기존 끝/삽입 끝을 기록하고 Session.synchronize가 활성 placeholder 안의 편집만 직렬 추적하도록 수정했습니다. 활성 범위 밖·유실 journal·문서/뷰/revision/선택/읽기 전용 경계는 세션을 취소하며 실제 문서 변경을 되돌리지 않습니다.

후속 검사에서 인접 mirror의 두 번째 시작이 첫 번째 편집까지 늘어나는 실패를 관찰했습니다. snippet-default-input-boundary-diagnostic.log의 실제 범위는 0..6과 0..12였습니다. 같은 active 그룹이어도 편집을 포함하는 placeholder만 경계를 늘리고, 인접한 다음 placeholder 시작은 앞 placeholder 끝에서의 삽입을 따라 이동하도록 고쳤습니다. 내부 range mapper를 기본 입력과 기존 Session.replace/step이 함께 소비합니다.

snippet-default-input-after-adjacent.log에서 --test change-journal의 13건은 모두 통과했습니다. 함께 실행한 스니펫 경계 검사 때문에 전체 명령은 exit 101이었으며 성공 종료로 확대하지 않습니다. 이후 실패 범위를 수정한 snippet-default-input-after-range-ownership.log의 --test snippet-insertion --test completion-insertion는 exit 0·7+5통과·0실패입니다. 모두 cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target로 직접 실행했습니다. 신규 검사는 journal UTF-16 동시 편집/undo 1건, 기본 입력의 여러 revision/Unicode mirror/tabstop과 활성 영역 밖 취소 2건입니다. 실제 앱 세션/choice/nested 수락과 화면 연결은 후속 범위입니다.

## 실제 선택지 소비의 공개 코어 경계

ActiveChoice가 첫 커서 대신 원래 주 커서의 placeholder/index·선택지를 반환하도록 수정하고 select_active로 현재 활성 그룹의 전체 mirror 범위를 문서 변경 없이 다시 선택합니다. 문자 입력 후 축소된 커서 상태에서도 선택지 수락이 전체 placeholder를 대체할 수 있습니다. 앱은 선택지 삽입의 용량 거절 시 이전 선택/스크롤/접기를 복원하며 세션을 유지합니다. nested 삽입의 합성은 아직 남은 범위입니다.

snippet-choice-core-api.log의 cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test snippet-insertion --test completion-insertion -- --test-threads=4는 exit 0·8+5통과입니다. 새 코어 1건은 두 원래 커서와 네 mirror/두 줄/이모지 입력 뒤 주 커서의 선택지 범위, select_active의 revision 불변/전체 mirror 선택과 다음 tabstop의 primary를 확인합니다. 앱 completion-choice-primary-capacity.log의 실제 Provider 메모리 입력 7건은 exit 0이며 선택지 UI·typed 필터/수락·다중 커서 primary·용량 거절을 확인합니다. 앱 전체 구현 근거는 배치 22 QA에 기록합니다. core fmt exit 0이며 이 결과를 배치 전체 게이트로 확대하지 않습니다.

## 중첩 스니펫의 탭 이동과 원자적 삽입

실제 설치된 Monaco 0.56.0 snippetSession.js의 OneSnippet.merge·SnippetSession.merge와 원본 controller의 기존 세션 병합 경로를 읽었습니다. 원본은 활성 placeholder의 각 발생에 안쪽 snippet을 연결하고, 안쪽 nonfinal 위치·안쪽 final 위치·바깥 다음 위치 순서로 이동합니다. 일반 텍스트/최종 위치만 있는 삽입은 기존 세션의 범위를 추적하며 유지합니다. 원본의 소수 index 누적으로 생기는 위치 충돌은 재현하지 않습니다.

Session.insert_nested는 기존 insertion의 검증·위치 계산을 준비/적용 단계로 나눠 재사용합니다. projected Rope에서 안쪽 metadata·부모 대응·전체 합산 용량·선택과 UTF-16 범위를 검사한 뒤 Store의 분리 undo transaction을 적용합니다. 용량이나 준비물 검증 실패로 실제 문서·선택·revision·undo가 일부 변경되지 않습니다. 교체된 활성 부모와 이전 자손은 제거하고 살아 있는 상위 범위는 유지합니다. 안쪽 final은 일반 이동 위치로 승격하고 바깥 final만 세션을 끝냅니다. 각 커서의 변환/들여쓰기 문맥을 보존하고 사용하지 않는 문맥을 회수합니다. 기존/안쪽 그룹의 순서로 작은 정수 index를 다시 부여하므로 깊은 병합에서도 위치가 합쳐지지 않습니다.

실제 앱은 살아 있는 Session에 수락을 병합하고 표시된 선택지 상태와 변환 엔진 캐시를 갱신합니다. 정규식 엔진은 기존 앱 전용 Syntax 경계의 MonacoSnippetTransforms이며 editor/UI에 엔진 의존성을 추가하지 않습니다. 제거된 placeholder의 과거 컴파일 캐시를 누적하지 않으며 살아 있는 변환은 필요할 때 평가합니다.

- /private/tmp/taide-batch22-completion-nested-first.log: 실제 메모리 Provider의 중첩/일반 수락 2건을 먼저 실패로 재현했습니다. 중첩 final 도달과 일반 후보 수락에서 바깥 세션이 사라졌습니다.
- /private/tmp/taide-batch22-snippet-nested-core-final.log: cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test snippet-insertion --test completion-insertion, exit 0·14+5통과·0실패입니다. 신규 6건은 두 원래 커서/여덟 mirror·choice·주 커서·Unicode, 합산 metadata 용량/분리 undo, 20회 병합, 교체된 자손 회수, 안쪽/바깥 변환의 각 들여쓰기·커서 문맥, stale/잘못된 Unicode 경계/IME/문서 용량의 삽입 전 거절을 확인합니다.
- /private/tmp/taide-batch22-completion-nested-final.log: app --lib completion_ -- --test-threads=4, exit 0·41통과·0실패입니다. 실제 child 3건과 소비자 12건을 포함하며 신규 소비자 4건은 같은 프레임 문자/Tab/ShiftTab·중첩 final·바깥 tabstop·Unicode mirror·일반 수락·두 변환 엔진 평가·선택지 수락을 확인합니다. OS 입력을 합성하지 않은 egui RawInput 검사입니다.

전체 대상 --no-fail-fast와 미리보기·최종 기능표 완료 판정은 배치 22 PROCESS c–g의 남은 범위입니다. 이 결과를 배치 전체 완료로 확대하지 않습니다.

추가 위험 2건은 /private/tmp/taide-batch22-snippet-nested-context.log의 동일 core 명령 --test snippet-insertion snippet_nested_context에서 exit 0·2통과입니다. 활성 자식에 안쪽 snippet을 넣은 뒤 살아 있는 상위 placeholder의 활성 표시와 ShiftTab 역이동을 확인했습니다. 또 기존/안쪽 들여쓰기 문맥이 각각은 한도 안이어도 합계가 한도를 넘으면 문서·선택·revision·undo를 변경하기 전에 거절합니다. 기존 14+5건과 겹치지 않는 두 검사이며 변경 크레이트 전체 대상 실행으로 확대하지 않습니다.

현재 app 모든 테스트 대상 check는 completion-nested-app-check.log에서 exit 0이며 editor/app fmt check도 exit 0입니다. core 추가 검사 후 fmt도 exit 0입니다. 공유 core 변경 뒤 동결 host/Wasm은 completion-nested-frozen-host.log·completion-nested-frozen-wasm.log에서 각각 exit 0입니다. 모두 기존 locked/offline/shared target이며 Wasm에만 --target wasm32-unknown-unknown을 추가했습니다. 동결 디렉터리 git status는 비어 있습니다. manifest/lock/의존 그래프를 변경하지 않았고 기존 Wasm 경고 6건을 억제하지 않았습니다.

## 자동완성 미리보기 문맥·표시 모델 (2026-10-10)

Core의 후보 미리보기 문자열·삽입 전용 diff·첫 줄 주입/추가 줄/숨긴 접미사·표시 줄 투영과 여러 view zone을 연결했습니다. 원문 문서와 undo를 바꾸지 않으며 선택 수락은 기존 삽입 경로를 사용합니다. 원문과 화면 문자열의 바이트 매핑을 나누어 wrap·탭·Unicode·접기·다음 문서 줄과 원문 커서 위치를 보존합니다. 엔진은 앱 전용 Syntax 경계를 유지합니다.

설치된 Monaco 0.56.0의 computeGhostText·SnippetParser/adjustWhitespace·computeGhostTextViewData를 실제로 실행한 재생성 도구는 [미리보기 oracle](../utils/2026-10-10-monaco-completion-preview-oracle.js)입니다. Core fixture는 diff 9,711개·스니펫 문자열 1,344개·실제 GhostTextView 12개입니다. diff의 9개 원본 UTF-16/문자 수 혼용 사례는 승인한 유효 UTF-8 경계로 바로잡았으며 나머지 원본 결과와 일치합니다. 원본 버그까지 일치했다거나 표본을 전체 기능 완료율로 확대하지 않습니다.

Syntax의 기존 TextMate worker가 원문 앞줄의 상태에서 미리보기 줄만 별도로 평가합니다. 원문 토큰을 바꾸지 않고 revision·테마 세대·요청 ID·후보 입력을 확인하며 늦은 결과를 폐기합니다. 같은 후보는 공유 문자열/토큰 캐시를 재사용하고 활성 문서·줄 밖의 캐시를 회수합니다. 문서 전체 접두사를 복사하거나 editor/UI에 엔진 의존성을 추가하지 않습니다.

실행 결과:

- cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test completion-preview --test display-layout --test display-map: exit 0, 23건 통과·기존 성능 검사 1건 ignored. completion-preview-core-final.log에 기록했습니다.
- cargo test --manifest-path native/taide-native-syntax/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test token-pipeline: exit 0, 23건 통과. 앞줄 주석·추가 줄·원문 토큰 불변·테마/문서 변경·후보 교체/늦은 응답·닫힘/캐시 회수를 포함하며 completion-preview-syntax-final.log에 기록했습니다.

로그의 공통 접두사는 /private/tmp/taide-batch22-입니다. UI/App의 본문 미리보기 소비는 작업 트리에 연결했고 별도 회귀를 실행 중입니다. 이 모델 검증은 배치 22 전체 --no-fail-fast·최종 기능표·실기 pixel/IME/접근성·성능 게이트를 대체하지 않습니다. 배치 22 전체 완료 판정은 [배치 QA](2026-10-10-native-batch22-completion-snippets.md)의 c–g에 남깁니다.
