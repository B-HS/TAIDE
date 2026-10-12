# 배치 23 IDE 선택 동기화

현재 상태: editor-83의 본문별 300ms debounce·typed Host·기존 current/latest 서비스와 알림을 연결했습니다. 실제 readonly/입력 게이트·대기/거절/취소·동일 문서 소유·보조 창/peek/비파일 문서의 서로 다른 3건은 관련 검사와 전체 library 실행에서 통과했습니다. 전체 App은 67대상/877통과/22실패·보호 3제외·exit 101이며 준비 단계 실패 진단과 선별 Git이 남아 있습니다. 배치 23과 전체 전환은 미완료입니다. 기능 체크리스트는 297/588(50.5%)이며 전체 전환 완료율/잔여 시간은 실기·출시 게이트의 규모와 실행 근거가 부족해 미산정입니다.

- [x] a. 원본·서비스·실제 소비 경계 확인
- [x] b. 본문별 debounce·typed 요청·소유/취소/닫힘 연결
- [x] c. 실제 입력/조회/알림·Unicode/역방향/readonly·수명 검증
- [ ] d. 형식/의존 경계·기능표/PROCESS·선별 Git

## 원본과 완료 조건

`src/widgets/editor-pane/use-editor-ide-selection.ts`는 file 본문과 실행 중인 IDE 서버에만 구독합니다. 초기 mount/reveal은 보내지 않고 이후 커서/선택 변화의 300ms debounce가 끝나면 현재 모델의 선택 텍스트와 UTF-16 정규화 범위를 보냅니다. 빈 선택도 현재 조회에는 반영하지만 최신 비어 있지 않은 선택은 보존합니다. 본문 해제/교체는 대기를 취소하고 현재 선택을 지웁니다. AppFile/untitled/peek의 선택을 새 IDE 공급자로 확장하지 않습니다.

기존 `crates/taide-runtime/src/ide_actions.rs`의 set/clear facade와 `taide-ide`의 current/latest 저장·selection_changed 알림을 재사용합니다. native의 `ide-tools.rs`는 두 조회를 이미 제공하지만 본문 선택 공급자는 없습니다. 범위는 기존 `taide-native-editor::lsp::byte_to_position`으로 변환합니다. 서비스 종료는 `IdeStore::take_shutdown_state`가 두 선택 슬롯을 회수합니다.

소유 범위는 App 본문 조립/Host와 앱 전용 선택 상태·관련 검사입니다. 외부 입력이나 새 RPC/의존성·엔진·frozen 변경을 추가하지 않습니다. 임시 앱과 주입한 서버 상태만 사용하며 실제 IDE 클라이언트/개인 파일/키/클립보드/OS 설정은 사용하지 않습니다. 소유 수명이 끝난 대기 요청은 기존 watch 취소 패턴으로 회수합니다. [Tokio 공식 watch 문서](https://docs.rs/tokio/latest/tokio/sync/watch/)의 채널 폐쇄/has_changed 계약을 확인했습니다.

실제 앱 검사는 readonly 본문의 초기 Unicode 위치를 먼저 설정한 뒤 서버를 주입하고 실제 Shift+방향키를 보냅니다. 초기 reveal 미전송·debounce·UTF-16 범위/알림·idle 중복 억제·빈/역방향 선택·최신 선택 유지·닫힘과 원문/디스크 불변을 확인합니다. 검사 수와 기능 분모를 구분하며 전체 게이트를 좁은 검사로 완료 처리하지 않습니다.

## 실제 재현과 첫 전달

`/private/tmp/taide-batch23-ide-selection-red.log`는 실제 readonly 본문에서 `IDE Unicode selection` 준비 실패, 0통과/1실패·32.66초·exit 101입니다(컴파일 2분 48초). 기능을 끄거나 제한을 늘리지 않았습니다.

`editor-ide-selection.rs`를 App 전용으로 추가했습니다. 본문별 초기 선택은 구독 기준값으로만 저장하며 이후 변화만 debounce합니다. 같은 문서의 mirror도 뷰 키·문서·프로젝트·창 범위로 구분합니다. 이전 요청의 watch 취소와 채널 폐쇄를 확인하고 Host에서 활성 file 탭/프로젝트/창·서버 실행 조건을 재검증합니다. 실제로 발행한 소유와 입력은 공유 lease에 기록해 다른 뷰의 선택을 지우지 않습니다. 상태/요청은 기존 DTO와 facade를 사용하며 내용이나 개인정보를 로그에 쓰지 않습니다.

`/private/tmp/taide-batch23-ide-selection-green.log`는 실제 App 1통과/0실패·10.11초·exit 0(컴파일 34.72초)입니다. 초기 reveal 미전송·UTF-16 (1:2~1:4)의 emoji·알림 1회/idle 무전송·빈 선택과 이전 latest·역방향 (12→8)·닫힘/대기 회수·readonly 원문/revision/dirty/디스크 불변이 통과했습니다. 새 검사의 대문자 함수명 경고는 snake case로 수정했고 기존 wry/linker 경고와 구분합니다.

이 성공을 입력 게이트/거절·동일 파일의 다른 소유/보조 창·AppFile/untitled/peek·전체 대상/실기/출시로 확대하지 않습니다. 해당 직접 위험을 확인한 뒤 d2/기능표를 완료로 바꿉니다.

## 입력 게이트와 전송 거절

실제 선택 직후 System Usage 상세 창을 열어 본문 입력을 잠근 `ide-selection-gated-red.log`는 0통과/1실패·23.22초·exit 101(컴파일 16.72초)이었습니다. 상태는 그대로였지만 준비된 선택 요청을 일반 화면 명령 Vec에 넣어 `is_enabled == false`에서 버렸습니다. 선택 전달은 본문이 구독 중인 동안의 동기화이므로 일반 입력 게이트와 분리해 typed Host에 직접 제출합니다. 거절된 요청은 같은 300ms 간격으로 보존하며 취소된 이전 선택은 재시도 목록에서도 제거합니다.

`ide-selection-gated-green.log`는 같은 검사 1통과/0실패·42.23초·exit 0(컴파일 30.27초)입니다. 상세 창이 열린 상태에서도 전달과 원래 불변 검사가 통과했습니다. 앞선 같은 검사 성공을 추가 합산하지 않습니다.

## 추가 수명과 창/문서 검사

두 추가 검사의 첫 시도는 ViewKey 변수명이 입력 key 보조 함수를 가리는 E0618 네 곳으로 컴파일에 실패했습니다(`ide-selection-extra.log`). 변수명을 수정한 `ide-selection-extra-final.log`는 실제 추가 경계 전에 두 검사 모두 공용 `document` fixture에서 시간 초과해 0통과/2실패·50.49초·exit 101입니다. 제품 기능의 실패나 해결로 바꾸어 표현하지 않습니다.

같은 컴파일된 library 검사 바이너리를 기존 f1 조사와 같이 저장소 cwd에서 직접 실행한 `ide-selection-extra-direct.log`는 2통과/0실패·30.83초·exit 0입니다. 원문/기한/제품을 바꾸지 않았지만 실행 cwd가 다르므로 Cargo 초기화 실패의 원인 해결이나 같은 환경 전체 성공을 뜻하지 않습니다.

추가 상태 검사는 초기 미전송·300ms 재입력/거절 보존·숨긴 뷰/Drop의 대기 취소·닫힌 뷰의 늦은 응답 거절·같은 파일의 main/aux 소유 구분·다른 뷰의 clear가 현재 선택을 지우지 않음·제거된 보조 창의 Host 재검증을 확인했습니다. 추가 실제 App은 main→보조 창의 다른 선택, main 보존·untitled/AppFile의 실제 입력/선택 미공급·IDE 실행 유지·current clear/latest 유지·설정 디스크 불변을 확인했습니다. 실제 OS 다중 viewport/입력 검증과 구분합니다.

정의 미리보기까지 추가한 후 `ide-selection-peek.log`는 보조 창의 `IDE peek mounted`에서 0통과/1실패·34.03초·exit 101입니다(컴파일 17.84초). provider 준비는 통과했지만 일반 팔레트 편집 intent가 메인 root 탭을 사용했습니다. 기존 `intent_with_editor`의 창별 본문 선택을 모든 `Run::EditDocument`에 적용해 현재 보조 창의 owner에 전달합니다. 미리보기 입력은 본문 IDE 선택을 변경하지 않는지 같은 실제 경계에서 확인 중입니다. 다른 Run 계열의 정책이나 frozen 경로는 바꾸지 않습니다.

## 초기화 대기 해결 뒤 최종 관련 검증

편집 대상 수정 뒤 Cargo와 직접 cwd 모두 document 준비에서 실패한 두 로그는 각각 `ide-selection-peek-fixed.log` 0통과/1실패·42.20초·exit 101과 `ide-selection-peek-fixed-direct.log` 0통과/1실패·32.63초·exit 101입니다. 같은 초기화 실패를 더 반복하지 않고 Host의 RestoreWatchers 선행 대기를 조사·수정했습니다. [초기화 큐 기록](2026-10-12-native-watcher-restore-queue.md)에 결정적 실패→수정→회수 근거를 보존합니다.

초기화 수정 뒤 `ide-selection-final-related.log`는 현재 제품의 서로 다른 3통과/0실패·36.81초·exit 0(컴파일 24.65초)입니다. 보조 창의 실제 팔레트 Peek Definition과 preview Shift+방향키가 실행됐고, preview 선택 변화는 IDE의 본문 선택과 debounce를 바꾸지 않았습니다. 이전 body/추가/직접 실행의 성공을 다시 합산하지 않습니다. 관련 기능/호스트 변경의 전체 App 회귀는 `ide-selection-app-all-targets.log`로 직접 1회 진행 중이며 보호 Trash 3건을 제외하고 no-fail-fast로 끝까지 관찰합니다.

전체 library는 636통과/21실패/보호 1제외·568.97초입니다. IDE 세 검사와 창별 터미널/팔레트 경계는 통과했습니다. 실패 16건은 code action 9·code lens 2·메뉴 5의 `phase=provider`에서, 나머지 5건은 프로젝트 준비의 timeout에서 발생했습니다. 기능 본문에 도달하지 않은 준비 실패를 기능 회귀나 해결로 단정하지 않습니다. 이후 Host 9건과 LSP 통합/복구/WorkspaceEdit 대상은 통과했으며 전체 실행은 아직 종료되지 않았습니다.

최종 전체 결과는 67대상/877통과/22실패/보호 3제외·exit 101입니다. 추가 실패는 `tests/projects.rs:114`의 기존 project open timeout 1건이며 Host 9건은 exit 0·6.76초입니다. 소유 Cargo 전체 실행은 종료됐습니다. 대표 provider 실패의 준비 상태를 진단하려고 두 공용 App wait 단언에 문서 언어/등급/revision·서버 요약·loading 수만 추가했습니다. 검사 입력/기한/단언 조건은 유지합니다. 실제 mock의 필요한 초기화 capability는 응답했으며, 빈 params와 초기화 뒤 EOF로 끝낸 수동 조회는 검증 성공으로 합산하지 않습니다.

`ide-selection-provider-diagnostic.log`의 정확한 code action 대표 검사 1건은 17.88초·exit 0입니다. 기한/입력/단언 조건을 바꾸지 않았으며 추가한 진단 정보는 실패할 때만 평가됩니다. 전체의 나머지 실패 21건을 이 단독 성공으로 대체하지 않습니다. code action의 기능 실패가 재현되지 않았으므로 제품 우회나 기한 확대를 추가하지 않고 최초 준비 실패의 원인 미확정을 배치 31 신뢰성 게이트에 보존합니다. 전체 성공은 여전히 미확인입니다.

이 진단의 컴파일은 6분 08초입니다. 컴파일러 PID 64934의 CPU 누적은 관찰 시 4분 12초 중 2.26초였고, `provider-build-sample.txt`의 실제 스택은 proc macro dylib 로드의 `dyld4::Loader::mapSegments → fcntl`에서 대기했습니다. Cargo의 컴파일 대기 관찰이며 App provider나 FSEvents 실패의 원인 증명으로 확대하지 않습니다. 샘플과 Cargo는 종료됐고 OS 설정·시그니처·quarantine·캐시·기한을 바꾸지 않았습니다.

초기 워처 복원은 `143bb85b`로 구현·회귀·QA 4경로만 독립 커밋했습니다. App/UI/Editor/Syntax/frozen manifest·lock과 엔진 변경은 없으며 관련 production 8Rust의 기존 fmt 성공을 재사용합니다. 두 진단 검사 파일의 형식과 최종 diff를 확인하고 IDE 선택 구현·실제 세 검사·QA를 선별 저장합니다. 사용자의 기존 문서 변경은 포함하지 않습니다.
