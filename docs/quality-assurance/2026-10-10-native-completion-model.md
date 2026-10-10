# 자동완성 후보·필터·삽입 모델 검증

현재 상태: 배치 22의 독립 core 모델 구현입니다. 실제 앱/peek 공급자와 화면 연결은 진행 중이며 기능 대응표의 완료 행을 올리지 않습니다. 현재 기능 대응표는 286/588(48.6%), 배치 체크리스트는 2/7입니다.

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
