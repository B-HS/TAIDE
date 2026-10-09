# 배치 19 — 구문 접기 공급·수동/Import 접기 명령

현재 상태: 구문·수동 접기와 실제 LSP/표면/앱 연결의 native 전체 게이트를 마쳤습니다. 서로 다른 최신 성공은 editor 206·UI 362·app 663·LSP SDK 81, 합계 1312건입니다. 현재 기능표는 완료 281/588(47.8%), 부분 93·미연결 113·미구현 101이며 batch19 체크리스트 6/7입니다. 전체 출시 전환율/잔여 시간은 미산정입니다. main이 직접 수행하고 실제 앞 Cargo/fmt process 종료를 확인한 뒤 다음 명령을 실행했습니다.

## 범위와 실제 기준

TS `src/shared/lib/lsp/adapters/folding-range.ts`는 준비된 foldingRangeProvider의 textDocument/foldingRange를 요청해 완전한 줄 범위와 comment/imports/region 종류를 공급합니다. `initialize-params.ts`는 lineFoldingOnly와 rangeLimit를 광고합니다. 설치된 Monaco의 syntaxRangeProvider는 여러 provider 결과를 시작 줄/우선순위로 정렬하고 중첩·교차·같은 시작을 정리하며 foldingModel/foldingRanges는 사용자 범위·현재 커서·편집 후 상태를 병합합니다. 원본 수동 범위 생성/제거와 toggleImportFold는 착수 시 native registry에서 세 명령 모두 미지원이었습니다.

착수 시 native는 들여쓰기·언어 marker의 FoldRegion/FoldingModel·tracked fold/선택·16종 명령과 gutter/접힌 본문·wrap/스크롤/고정 줄 폴백을 공급했고, typed SDK의 FoldingRangeRequest는 앱에서 소비하지 않았습니다. 이번 변경은 구문 범위·종류·수동 범위를 연결하고 기존 16종에 수동 생성/제거·Import 토글을 더한 원본 카탈로그 19종을 공급합니다. 원본에 없는 기능/디자인·원본 버그/내부 수치 강제 재현·engine/의존성·OS 합성 입력은 추가하지 않았습니다. frozen remote-web source/manifest/lock·실제 데이터/OS 설정/clipboard/Keychain/Trash·보호 app bundle은 유지합니다.

## 체크리스트

- [x] a. 실제 TS/Monaco·공식 typed API·현재 native 접기/표시/명령 경계 대조
- [x] b. 구문/종류·중첩/순서/한도·수동 범위·편집/다중 뷰 모델 검증
- [x] c. 실제 typed LSP 준비/미지원/빈/오류·취소/버전/프로젝트/재시작 공급
- [x] d. native-host gutter/본문/키/마우스·수동/Import 명령·표시/스크롤 소비
- [x] e. 실제 앱·고정 줄 provider 우선/폴백·reveal/readonly/대형 통합
- [x] f. 변경 크레이트 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 현재 검증 상태

editor 전체 `--all-targets --no-fail-fast` 직접 1회 실행은 29대상·205통과·1실패·1ignored였습니다(`/private/tmp/taide-batch19-core-full.log`). 기존 본문 삭제 검사가 실패했고, 빈 줄 접기를 허용한 변경이 삭제된 일반 접기까지 빈 범위로 남기는 원인이었습니다. 기존 범위가 처음부터 비었는지 추적하여 실제 빈 줄 접기는 유지하고 삭제된 일반 접기는 회수했습니다. 영향받은 folding 대상만 다시 실행해 21통과·0실패를 확인했습니다(`/private/tmp/taide-batch19-core-fold-after-tracking.log`). 현재 서로 다른 editor 검사는 206통과·1ignored이며 최초 전체 실행의 실패와 수정 근거를 보존합니다.

taide-lsp SDK 전체 `--no-fail-fast` 직접 1회 실행은 단위 81통과·0실패·0ignored, 문서 대상 0건입니다(`/private/tmp/taide-batch19-sdk-full.log`). UI inspection 전체 `--all-targets --no-fail-fast` 직접 1회 실행은 19대상·361통과·1실패였습니다(`/private/tmp/taide-batch19-ui-full.log`). 새 명령을 기대 목록의 원본 등록 순서에 놓지 않은 검사 오류를 바로잡고 해당 1건만 실행해 통과했습니다(`/private/tmp/taide-batch19-ui-registry-after-order.log`). 현재 UI 서로 다른 362건 통과·미해결 실패 0입니다. app 전체와 최종 동결/포맷/디스크의 실제 결과는 아래에 기록합니다.

## 확인한 계약과 결정

실제 Monaco languageFeatureRegistry는 같은 언어 점수의 provider를 최근 등록 순으로 정렬합니다. foldingController의 auto 전략은 성공한 배열 결과를 시작 줄·provider 순서로 합치며 같은 시작은 먼저 나온 범위를 사용합니다. native는 기존 session 생성 순서의 역순을 안정적인 우선순위로 사용합니다. TS의 준비 완료 후 provider 등록 도착 시점과 순서까지 동일하다고 판정하지 않습니다. 중첩은 유지하고 교차는 최종 FoldingRegions 병합에서 제외합니다. 모든 provider가 미지원/오류/null이면 들여쓰기·언어 marker로 폴백하고 성공한 빈 배열은 명시적인 빈 구문 모델입니다. TS adapter의 null→빈 배열 변환 때문에 폴백이 막히는 특이 동작은 사용자의 원본 버그 비재현 지시에 따라 강제하지 않습니다. limit 5000/line-only 광고와 외부 범위 유효성은 유지합니다.

원본 createFoldingRangeFromSelection은 선택 끝이 열 1이면 끝 줄을 제외하고 두 줄 이상을 사용자 범위로 만들며 Cmd/Ctrl+K, Cmd/Ctrl+Comma를 사용합니다. removeManualFoldingRanges는 선택과 교차하는 사용자/회수 범위를 제거하고 Cmd/Ctrl+K, Cmd/Ctrl+Period를 사용합니다. toggleImportFold는 imports 종류 각 범위를 개별 반전하며 기본 키가 없습니다. 펼친 수동 범위도 남고 같은 시작/교차 provider보다 우선합니다. 원본 반복 setSelection 때문에 마지막 선택만 남는 다중 선택 결함은 강제하지 않고 유효한 각 선택의 시작 caret를 보존합니다. 문서 내용을 바꾸지 않으므로 readonly에서도 실행하며 기존 large-file folding 비활성 gate는 유지합니다.

ViewState는 현재 selection/scroll/접힌 byte 범위와 편집 journal을 보유합니다. 수동 범위도 같은 줄 끝 decoration 추적으로 편집·undo/redo를 따르고 뷰별로 유지하며 문서 교체/언어 변경의 기존 fold 초기화에서 함께 비웁니다. 구문 모델은 document/key/revision/language에 묶이고 공급 Arc 교체를 surface cache가 확인합니다. 고정 줄은 outline→구문→indent 순서이며 현재 StickyState가 FoldRegion 공급과 Arc 갱신을 이미 소비합니다. 기존 앱의 접기 state는 살아 있는 뷰에 유지되고 디스크 memento 저장은 구현되어 있지 않아 이번 범위의 완료 근거로 확대하지 않습니다.

설치된 공식 lsp-types 0.97.0의 FoldingRangeKind는 세 종류의 닫힌 enum이지만 공식 foldingRangeKind 계약은 임의 문자열의 graceful 처리를 요구합니다. SDK typed 응답에 원본 FoldingRange를 flatten한 wrapper와 문자열 kind를 공급했습니다. 사용자 종류의 실패 재현 `/private/tmp/taide-batch19-sdk-before.log`는 1실패였고 보완 뒤 `/private/tmp/taide-batch19-sdk-after.log`는 1통과입니다. 표준/사용자/생략/null 종류와 숫자·범위 검증을 함께 보존하며 의존성은 추가하지 않았습니다.

설치된 공식 lsp-types 0.97.0의 FoldingRangeParams/Range/Kind/ClientCapabilities와 기존 SDK typed 요청 등록, TS adapter/initialize 및 Monaco의 syntax provider·수동 생성/제거·Import toggle 코드를 읽었습니다. native의 FoldRegion·들여쓰기/marker·surface cache와 명령/view state·provider 순서 및 수명을 대조하여 a를 완료했습니다. 최신 실제 child 4건에서 종류/빈/null/오류/미지원·보류 중 다른 문서의 실제 응답·편집/닫힘 취소·재시작/프로젝트 격리가 통과했고 실제 앱 Import 접기/reveal 1건도 통과했습니다. UI의 원본 접기 키·수동/Import 본문/gutter·outline→syntax→indent 고정 줄 및 Arc 교체 검사를 확인하여 c~e를 완료했습니다. 실제 child 여러 folding provider의 부분 오류 조합을 별도로 실행한 것은 아니며 core 다중 provider 병합 검사와 실제 코드의 독립 오류 처리를 근거로 구분합니다.

변경 없는 syntax/egui SDK의 이전 근거는 해당 경계가 바뀌지 않을 때만 재사용합니다. 이번 변경 크레이트 전체 대상은 f에서 직접 실행했습니다. 보호 Trash 3·기존 ignored 5·실제 OS/IME/접근성/pixel/대형/soak/출시 부채와 잘못된 7자리 theme HEX 결정은 미완료로 유지합니다.

## 앱 전체와 최종 게이트

app 전체 `--no-fail-fast` 직접 1회 실행은 67대상·662통과·1실패·보호 검사 3제외였습니다(`/private/tmp/taide-batch19-app-full.log`). 기존 command-dispatch 검사에서 수동 생성/제거·Import 토글을 여전히 미지원으로 기대했습니다. 실제 원본 카탈로그의 19종을 지원 목록으로 갱신하고 공유 목록을 사용하는 명령 모듈만 재검사하여 7통과·0실패를 확인했습니다(`/private/tmp/taide-batch19-app-dispatch-after-supported.log`). 현재 서로 다른 app 성공은 663건이며 최초 실패와 수정 근거를 보존합니다. 전체를 반복하지 않았습니다.

이번 변경 editor/UI/app/LSP SDK의 서로 다른 최신 성공은 206+362+663+81=1312건입니다. 최초 전체 실행 117대상의 실패 3건은 각각 영향받은 대상 재검사로 해소했습니다. editor 성능 1ignored와 보호 Trash 3건은 실행하지 않았습니다. 변경 없는 syntax/egui SDK의 기존 검사/ignored는 이번 합계에 더하지 않습니다.

검증 서버는 app Cargo의 `native-lsp-mock` example 경로로 빌드했고 실제 child 4건 및 앱 통합에서 소비했습니다(`/private/tmp/taide-batch19-mixed-mock-build.log`). 별도 experiments/lsp-coordinator-spike의 `--all-targets --no-fail-fast --locked --offline` 실행도 시도했으나 현재 prototype lockfile이 manifest와 맞지 않아 시작 단계에서 exit 101로 거절됐습니다(`/private/tmp/taide-batch19-fixture-full.log`). 해당 standalone 검사를 통과로 세지 않았고 의존성/lockfile을 바꾸지 않았습니다. 이 기존 prototype build 정리는 전체 workspace/CI 게이트의 잔여로 남깁니다.

standalone prototype `fmt --check`는 exit 1입니다(`/private/tmp/taide-batch19-fixture-fmt-check.log`). 같은 mock-server.rs를 edition 2024인 app example과 edition 2021인 prototype이 서로 다른 import/줄바꿈 스타일로 포맷하며, 수정하지 않은 native-session.rs 두 긴 함수 선언도 현재 formatter와 차이가 납니다. app 경로의 포맷 검사는 통과했고 prototype 전역 재포맷/edition 변경은 이번 범위에 섞지 않았습니다. 전체 workspace/CI의 edition 통일 게이트에서 해소해야 합니다.

최종 frozen host `check --tests`는 exit 0(6.93초), wasm32-unknown-unknown/canvas check는 exit 0(3.59초)입니다(`/private/tmp/taide-batch19-web-host.log`, `/private/tmp/taide-batch19-web-wasm.log`). editor/UI/app/LSP SDK `fmt --check`도 exit 0이며 Cargo는 실제 앞 명령 종료를 확인한 뒤 하나씩 실행했습니다. app의 기존 vendor wry deprecated/unsafe 경고와 linker unwind 경고는 실행 로그에 남겼고 새 공유 keymap의 잘못된 cfg 경고는 원인을 수정했습니다.

원본 syntax provider·수동/Import 명령·기본 키의 MIT 출처/전문을 `THIRD_PARTY_LICENSES.md`에 확장했습니다. manifest/lock/remote-web diff는 비었고 editor/UI의 engine 의존성은 추가하지 않았습니다. 디스크는 668GiB 여유·64% 사용이며 산출물을 정리하지 않았습니다. 실제 app bundle·사용자 데이터/OS 설정/clipboard/Keychain/Trash를 조작하지 않았습니다.

실행 명령의 공통 옵션은 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 전체 대상은 editor `test --all-targets --no-fail-fast`, UI `test --all-targets --no-fail-fast --features inspection`, app `test --no-fail-fast`, LSP SDK `test --no-fail-fast`를 각 manifest에 실행했습니다. app 인자에서 아래 세 검사만 제외했습니다.

- `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`
- `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`
- `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다`
