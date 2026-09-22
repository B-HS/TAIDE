# 프로젝트 전체 사용자 기능 버그 점검 — 2026-09-22

점검 기준은 `dev` 브랜치의 `4b57695`입니다. 실제 사용 결과가 입력·저장 상태·프로토콜 계약과 어긋나는 **14건**을 확인했습니다. P1은 잘못된 파일 변경이나 예상하지 못한 명령 실행 위험이 있는 3건, P2는 특정 조건에서 기능이 정상 동작하지 않는 11건입니다. 제품 코드 수정은 수행하지 않았습니다.

`격리 재현`은 실제 서비스·훅·편집 함수를 임시 파일이나 대체 통신 환경에서 실행했다는 뜻입니다. `코드 흐름 확인`은 호출 경로와 상태 갱신을 대조한 결과이며, 실제 앱의 해당 화면을 조작해 재현한 결과와 구분합니다.

## 1. 확인된 문제 목록

### 파일과 편집

| ID | 우선순위 | 사용자에게 나타나는 문제 | 확인 방법 |
| --- | --- | --- | --- |
| B01 | P1 | 심볼릭 링크 이름 변경·삭제가 링크의 원본 파일에 적용됩니다. | 이름 변경 격리 재현, 삭제 경로 확인 |
| B02 | P1 | 언어 서버의 파일 이름 변경·삭제 후 열린 탭과 미저장 편집 내용이 이전 경로에 남습니다. | WorkspaceEdit 격리 재현·호출 경로 확인 |
| B04 | P2 | 저장 중 추가로 입력하면 그 내용의 자동 저장이 다시 예약되지 않습니다. | 실제 저장 훅 격리 재현 |
| B06 | P2 | 언어 서버가 같은 위치에 여러 문자열을 삽입하면 순서가 뒤집힙니다. | 실제 편집 함수 재현 |
| B07 | P2 | 언어 서버가 줄 끝을 큰 문자 위치로 표현하면 다른 줄이나 파일 끝을 편집합니다. | 실제 편집 함수 재현 |

### 원격 연결과 Git

| ID | 우선순위 | 사용자에게 나타나는 문제 | 확인 방법 |
| --- | --- | --- | --- |
| B03 | P1 | 실패로 반환된 원격 파일 변경 명령이 이후 재연결에서 실행됩니다. | 실제 클라이언트·대체 WebSocket 재현 |
| B05 | P2 | 원격 연결 복구 후 기존 터미널의 출력 구독이 복구되지 않습니다. | 연결 계층 재현·양쪽 호출 경로 확인 |
| B08 | P2 | 대상이 없는 심볼릭 링크를 스테이징하면 성공으로 끝나지만 추가되지 않습니다. | 실제 Git 서비스·임시 저장소 재현 |
| B09 | P2 | UTF-8로 읽을 수 없는 파일을 Git 변경 비교에서 빈 파일처럼 표시합니다. | 실제 Git 서비스·임시 저장소 재현 |

### 설정·작업 실행·미리보기

| ID | 우선순위 | 사용자에게 나타나는 문제 | 확인 방법 |
| --- | --- | --- | --- |
| B10 | P2 | 새 기기나 연결 해제 후 재연결에서 기존 동기화 자료를 선택·다운로드할 수 없습니다. | 설정 UI와 서버 호출 경로 확인 |
| B11 | P2 | 다른 기기에서 글꼴을 기본값으로 되돌려도 다운로드한 기기의 사용자 지정 글꼴이 남습니다. | 실제 동기화 서비스 재현 |
| B12 | P2 | 여러 타깃을 선언한 Make 규칙에서 실행할 수 없는 작업을 생성합니다. | 실제 작업 탐지와 Make 실행 재현 |
| B13 | P2 | HTML 미리보기에서 상대 경로의 이미지·스타일시트가 로드되지 않습니다. | 동일 Blob·iframe 구조의 WebKit 재현 |
| B14 | P2 | 설정 JSON에서 언어·테마를 바꾸거나 다른 창에서 언어를 바꾸면 화면이 갱신되지 않습니다. | 저장·이벤트·캐시·화면 호출 경로 확인 |

## 2. 우선 수정 대상

### B01. 심볼릭 링크 작업이 원본 파일을 변경함 — P1

- **조건·절차:** 프로젝트 안에 `target.txt`와 이를 가리키는 `link.txt`를 만듭니다. 탐색기에서 `link.txt` 이름을 `renamed-link.txt`로 바꿉니다.
- **기대:** 링크 자체의 이름만 바뀌고 `target.txt`는 유지되어야 합니다.
- **실제:** `target.txt`가 이동하고, 기존 `link.txt`는 깨진 링크로 남으며, 새 이름의 항목은 일반 파일입니다. 삭제도 같은 경로 해석 후 원본 경로를 삭제 서비스에 전달합니다.
- **원인·위치:** [file/commands.rs:79](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/file/commands.rs:79), [file/commands.rs:94](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/file/commands.rs:94)에서 소스 경로를 정규화합니다. [root_guard.rs:154](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/infra/root_guard.rs:154)의 정규화는 마지막 심볼릭 링크까지 따라갑니다.
- **검증:** 임시 디렉터리에서 실제 root guard와 rename 서비스를 실행해 원본 이동·깨진 링크·새 항목 종류를 확인했습니다. 삭제는 동일한 해석 경로를 확인했으며 사용자 파일 삭제 실험은 하지 않았습니다.
- **수정 방향:** 프로젝트 경계 검증을 유지하면서 이름 변경·삭제 대상의 마지막 경로 항목은 링크 자체로 보존해야 합니다.

### B02. 언어 서버의 파일 변경이 열린 편집 상태에 반영되지 않음 — P1

- **조건·절차:** `old.ts`를 열고 저장하지 않은 변경을 남깁니다. 언어 서버가 `RenameFile(old.ts → new.ts)`와 `new.ts`의 텍스트 편집을 포함한 WorkspaceEdit을 반환하게 합니다.
- **기대:** 열린 탭·모델·복구용 미저장 내용이 새 경로를 따라가고, 이어지는 편집도 현재 편집 내용에 적용되어야 합니다.
- **실제:** 새 파일은 디스크에 저장됐던 내용으로 편집되고, 미저장 모델은 이전 경로에 남습니다. 열린 탭도 이전 경로를 유지해 파일을 찾지 못하거나, 이후 저장으로 이전 파일을 다시 만들 수 있습니다. DeleteFile도 열린 탭 정리를 거치지 않습니다.
- **원인·위치:** [workspace-edit-applier.ts:329](/Users/hyunseokbyun/development/TAIDE/src/shared/lib/lsp/workspace-edit-applier.ts:329), [workspace-edit-applier.ts:340](/Users/hyunseokbyun/development/TAIDE/src/shared/lib/lsp/workspace-edit-applier.ts:340)는 파일 명령만 호출합니다. 탐색기 경로에 있는 [file.query.ts:179](/Users/hyunseokbyun/development/TAIDE/src/entities/file/file.query.ts:179)의 탭·모델 후속 처리를 거치지 않습니다. 파일 감시 이벤트도 해당 경로를 재배치하지 않습니다.
- **검증:** 실제 `applyWorkspaceEdit`에 저장된 내용 `saved`, 열린 모델 `unsaved draft`를 제공했습니다. 성공 응답 뒤 새 파일은 `refactor:saved`, 기존 모델은 `old.ts`의 `unsaved draft`로 남았습니다. 실제 언어 서버와 네이티브 화면을 연결한 재현은 수행하지 않았습니다.
- **수정 방향:** 파일 리소스 작업과 열린 탭·모델·복구 데이터 이동을 하나의 일관된 처리 경로로 연결해야 합니다.

### B03. 실패한 원격 변경 명령이 재연결 후 실행됨 — P1

- **조건·절차:** 원격 연결이 열리지 않은 동안 파일 이름 변경을 요청합니다. 연결 시도가 실패해 호출자가 오류를 받은 다음, 후속 재연결이 성공하게 합니다.
- **기대:** 실패로 확정한 미전송 요청은 이후 자동 실행되지 않아야 합니다.
- **실제:** 호출자는 실패를 받지만 미전송 큐에는 요청이 남아 다음 연결에서 전송됩니다. 사용자가 작업을 포기하거나 다시 시도한 뒤에도 파일 변경이 일어날 수 있습니다.
- **원인·위치:** [remote-ws-client.ts:50](/Users/hyunseokbyun/development/TAIDE/src/shared/lib/remote/remote-ws-client.ts:50)의 `rejectAll`은 응답 대기 목록만 비웁니다. [동일 파일:114](/Users/hyunseokbyun/development/TAIDE/src/shared/lib/remote/remote-ws-client.ts:114)는 재연결 때 별도 `outbox`를 그대로 전송합니다.
- **검증:** 실제 클라이언트와 대체 WebSocket으로 `file_rename`의 Promise가 거부된 뒤 새 소켓에 같은 명령이 전송되는 것을 확인했습니다.
- **수정 방향:** 실패로 확정하는 요청과 전송 큐의 수명을 일치시키고, 재시도 정책과 중복 실행 방지를 명시해야 합니다.

## 3. 편집·원격 사용 오류

### B04. 저장 중 입력한 내용의 자동 저장이 누락됨 — P2

- **조건·절차:** 자동 저장을 켜고 저장 응답을 지연시킵니다. 첫 저장이 진행 중일 때 추가 입력한 뒤 타이핑을 멈춥니다.
- **기대·실제:** 첫 저장 완료 후 최신 내용도 자동 저장되어야 하지만, 첫 번째 내용만 저장되고 추가 내용은 미저장 상태로 남습니다. 추가 키 입력이나 수동 저장이 있어야 다시 진행됩니다.
- **원인·위치:** [use-editor-file-persistence.ts:295](/Users/hyunseokbyun/development/TAIDE/src/widgets/editor-pane/use-editor-file-persistence.ts:295)는 저장 중 새 타이머를 예약하지 않습니다. [동일 파일:438](/Users/hyunseokbyun/development/TAIDE/src/widgets/editor-pane/use-editor-file-persistence.ts:438)의 완료 처리도 남은 변경의 저장을 다시 예약하지 않습니다.
- **검증:** 기존 저장 훅 테스트 환경에 지연 응답을 넣었습니다. 자동 저장 간격 25ms, 첫 저장 해제 후 100ms 경과에도 저장 호출은 1회이고 최신 초안은 미저장 상태였습니다.
- **수정 방향:** 저장 완료 시 저장한 스냅샷과 현재 초안을 비교해 남은 변경의 자동 저장을 예약해야 합니다.

### B05. 원격 재연결 후 터미널 출력 구독이 끊긴 채 남음 — P2

- **조건·절차:** 원격 브라우저의 터미널을 열고 출력 구독이 성공한 다음, 네트워크 연결을 끊었다가 복구합니다. 탭은 그대로 둡니다.
- **기대·실제:** 재연결 후 기존 터미널 출력을 계속 받아야 하지만, 새 연결에 `pty_attach`가 재전송되지 않습니다. 탭을 다시 마운트하거나 화면을 새로 고치기 전까지 출력이 멎을 수 있습니다.
- **원인·위치:** [remote-ws-client.ts:114](/Users/hyunseokbyun/development/TAIDE/src/shared/lib/remote/remote-ws-client.ts:114)는 연결 복구를 상위 구독자에게 알리지 않습니다. [terminal-pane.tsx:199](/Users/hyunseokbyun/development/TAIDE/src/widgets/terminal-pane/terminal-pane.tsx:199)의 구독 효과는 `sessionId` 변경에만 반응합니다. 서버의 출력 채널은 [remote/ws.rs:205](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/remote/ws.rs:205)의 개별 소켓에 묶여 있습니다.
- **검증:** 연결 계층에서 성공한 attach → 끊김 → 재연결 후 attach 재전송이 없음을 재현하고, 화면의 구독 조건과 서버 채널 수명을 대조했습니다. 실제 네트워크 장애를 넣은 네이티브 E2E는 수행하지 않았습니다.
- **수정 방향:** 연결 복구 시 구독을 재설정하고, 끊긴 동안의 출력과 필요한 상태를 다시 가져와야 합니다.

### B06. 같은 위치의 언어 서버 삽입 순서가 뒤집힘 — P2

- **조건·절차:** 열려 있지 않은 파일의 같은 위치에 `first`, `second` 순서의 두 TextEdit을 적용합니다.
- **기대·실제:** `tail`은 `firstsecondtail`이 되어야 하지만 `secondfirsttail`이 됩니다. 여러 import·코드 조각을 함께 추가하는 편집 결과가 달라집니다.
- **원인·위치:** [workspace-edit-applier.ts:181](/Users/hyunseokbyun/development/TAIDE/src/shared/lib/lsp/workspace-edit-applier.ts:181)의 정렬은 같은 시작 위치의 원래 순서를 유지하고, 뒤에서 앞쪽으로 삽입하는 reduce가 그 순서를 반대로 만듭니다.
- **검증:** 실제 `applyTextEditsToContent`로 재현했습니다. 같은 위치의 여러 삽입은 배열 순서가 결과를 결정한다는 [LSP TextEdit 계약](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/textEditArray.md)도 확인했습니다.
- **수정 방향:** 같은 위치의 편집 순서와 삽입·치환이 결합된 경우의 순서를 함께 보존해야 합니다.

### B07. 줄 길이를 넘는 언어 서버 위치를 잘못 해석함 — P2

- **조건·절차:** 열려 있지 않은 `abc\nxyz`에 대해 0번째 줄, 문자 위치 100에 `!`를 삽입합니다.
- **기대·실제:** 줄 끝으로 보정한 `abc!\nxyz`가 되어야 하지만 `abc\nxyz!`가 됩니다. 범위 치환이라면 이후 줄까지 잘못 변경할 수 있습니다.
- **원인·위치:** [workspace-edit-applier.ts:164](/Users/hyunseokbyun/development/TAIDE/src/shared/lib/lsp/workspace-edit-applier.ts:164)는 줄 시작에 character를 그대로 더하며 해당 줄 끝으로 제한하지 않습니다.
- **검증:** 실제 편집 함수로 결과를 확인했습니다. 줄 길이를 넘는 character를 줄 길이로 보정하도록 정한 [LSP Position 계약](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/position.md)에 따라 유효하게 처리해야 하는 입력입니다.
- **수정 방향:** 줄바꿈 종류를 고려한 각 줄의 끝 위치를 계산해 LSP 위치를 보정해야 합니다.

## 4. Git·동기화 오류

### B08. 깨진 심볼릭 링크를 스테이징하지 못함 — P2

- **조건·절차:** Git 저장소에 존재하지 않는 `missing.txt`를 가리키는 `link.txt`를 만들고 스테이징합니다.
- **기대·실제:** 링크 자체를 인덱스에 추가해야 하지만 성공 응답 후에도 인덱스에 없습니다. 이미 추적 중인 링크라면 대상을 찾지 못한다는 이유로 삭제 분기를 타게 됩니다.
- **원인·위치:** [git/service.rs:154](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/git/service.rs:154)의 `exists()`는 링크 대상을 따라갑니다. 대상이 없으면 실제 링크 항목이 있어도 `remove_path`를 실행합니다.
- **검증:** 임시 Git 저장소에서 실제 `stage`의 성공 반환과 인덱스 항목 누락을 함께 확인했습니다.
- **수정 방향:** 항목 존재 여부와 종류를 심볼릭 링크를 따라가지 않는 메타데이터로 판단해야 합니다.

### B09. 읽기 실패한 Git 변경 파일이 빈 파일로 표시됨 — P2

- **조건·절차:** 인덱스에는 `hello\n`이 저장된 파일을, 작업 트리에서는 UTF-8로 해석할 수 없는 비어 있지 않은 내용으로 바꿉니다. 작업 트리 변경 비교를 엽니다.
- **기대·실제:** 인코딩·바이너리·읽기 오류를 구분해야 하지만 수정 후 내용이 빈 문자열로 반환됩니다. 사용자는 내용 전체를 지운 변경처럼 보게 됩니다.
- **원인·위치:** [git/service.rs:359](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/git/service.rs:359)가 모든 `read_to_string` 오류를 빈 문자열로 바꿉니다. 파일 부재와 인코딩 오류·권한 오류를 구별하지 않습니다.
- **검증:** 실제 서비스에 바이트 `FF 0A`인 파일을 제공해 `original = "hello\n"`, `modified = ""`인 성공 결과를 확인했습니다.
- **수정 방향:** 실제 삭제만 빈 파일로 처리하고, 그 외 읽기 실패는 비교 불가 사유를 반환해야 합니다.

### B10. 기존 동기화 Gist를 다시 연결할 경로가 없음 — P2

- **조건·절차:** 기기 A에서 설정을 업로드한 뒤 연결을 해제하고 같은 계정으로 다시 연결합니다. 또는 새 기기 B에서 같은 계정으로 연결합니다. 기존 설정 다운로드를 누릅니다.
- **기대·실제:** 기존 동기화 자료를 찾아 연결하거나 선택할 수 있어야 하지만 Gist ID가 없는 상태로 남아 다운로드가 거부됩니다. 안내대로 먼저 업로드하면 기존 자료에 연결하는 대신 새 Gist를 만듭니다.
- **원인·위치:** [sync/commands.rs:118](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/sync/commands.rs:118)는 토큰만 검증하고 기존 자료를 찾지 않습니다. [연결 해제:148](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/sync/commands.rs:148)는 ID를 지우고, [다운로드:253](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/sync/commands.rs:253)는 ID 없음을 오류로 처리합니다. [sync-section.tsx:80](/Users/hyunseokbyun/development/TAIDE/src/features/settings/sync-section.tsx:80)의 ID는 읽기 전용 표시입니다.
- **검증:** 연결·해제·업로드·다운로드·설정 화면의 전체 경로를 확인했습니다. 실제 계정·토큰·GitHub 자료는 조작하지 않았습니다. 원격 자료 자체가 지워지는 문제는 아니며, 설정 JSON을 수동 편집하는 우회와 별개로 일반 동기화 UI의 복구 경로가 없습니다.
- **수정 방향:** 기존 Gist 탐색·선택 또는 ID 입력을 제공하고 연결 해제 후 재연결 정책을 정해야 합니다.

### B11. 기본값으로 되돌린 글꼴 설정이 다른 기기로 동기화되지 않음 — P2

- **조건·절차:** 로컬 기기에서 편집기 글꼴을 `Fira Code`로 설정합니다. 다른 기기에서 글꼴을 시스템 기본값으로 되돌려 업로드한 자료를 로컬에서 다운로드합니다.
- **기대·실제:** 로컬 글꼴도 기본값으로 돌아가야 하지만 `Fira Code`가 그대로 남습니다. 터미널·UI 글꼴도 같은 경로입니다.
- **원인·위치:** [sync/service.rs:101](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/sync/service.rs:101)는 기본값을 `None`으로 내보냅니다. [동일 파일:285](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/sync/service.rs:285)는 이를 부분 변경으로 적용하고, [settings/service.rs:338](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/settings/service.rs:338)는 `None`을 기존 값 유지로 해석합니다.
- **검증:** 기본 Settings에서 생성한 실제 SyncPayload를 사용자 지정 글꼴 Settings에 적용해 값이 유지되는 것을 확인했습니다.
- **수정 방향:** 동기화에서 미포함과 명시적 기본값 복원을 구분하는 표현이 필요합니다.

## 5. 작업 실행·미리보기·화면 갱신 오류

### B12. 여러 Make 타깃을 하나의 이름으로 실행함 — P2

- **조건·절차:** `build test:`와 그 레시피를 가진 Makefile에서 자동 탐지된 작업을 실행합니다.
- **기대·실제:** `build`와 `test`를 각각 실행할 수 있어야 하지만 `make 'build test'`를 생성합니다. Make는 공백을 포함한 단일 타깃을 찾아 `No rule to make target` 오류로 종료합니다.
- **원인·위치:** [task/service.rs:100](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/task/service.rs:100)의 정규식이 콜론 앞 전체를 한 타깃으로 캡처하고, [동일 파일:117](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/task/service.rs:117)이 전체를 한 인자로 인용합니다.
- **검증:** 실제 `detect_tasks`의 생성 명령을 확인하고 임시 Makefile에서 해당 명령이 종료 코드 2와 함께 실패하는 것을 재현했습니다.
- **수정 방향:** 지원하는 Make 규칙의 타깃 목록을 정확히 나누고, 해석하지 못하는 규칙을 잘못된 실행 항목으로 노출하지 않아야 합니다.

### B13. HTML 미리보기의 상대 경로 리소스가 깨짐 — P2

- **조건·절차:** `index.html`이 같은 폴더의 `style.css`, `image.svg`를 상대 경로로 참조하도록 하고 미리보기를 엽니다.
- **기대·실제:** 허용된 프로젝트 리소스를 기준으로 경로가 해석되어야 하지만 이미지·스타일을 읽지 못합니다. 인라인 스타일이나 절대 URL만 사용하는 HTML에서는 드러나지 않습니다.
- **원인·위치:** [preview-pane.tsx:56](/Users/hyunseokbyun/development/TAIDE/src/widgets/preview-pane/preview-pane.tsx:56)은 HTML 원문 바이트를 Blob URL로 만들고, [html-preview.tsx:9](/Users/hyunseokbyun/development/TAIDE/src/features/preview/html-preview.tsx:9)는 이를 그대로 iframe에 전달합니다. 원래 파일의 기준 경로나 상대 리소스 변환이 없습니다.
- **검증:** 같은 Blob·sandbox iframe 구조를 WebKit에서 실행했습니다. 문서 기준 URL은 `blob:null/...`, 이미지 `naturalWidth`는 0이었고 해당 기준 URL에 대한 상대 URL 해석도 실패했습니다. 앱 전체 미리보기를 실행한 결과는 아닙니다.
- **수정 방향:** iframe 격리를 유지하면서 허용된 프로젝트 파일의 상대 경로를 해결할 자원 제공 경로가 필요합니다. 스크립트 실행 제한은 이번 버그에 포함하지 않습니다.

### B14. 설정 저장 경로에 따라 언어·테마 화면이 갱신되지 않음 — P2

- **조건·절차:** 앱의 설정 JSON 탭에서 `language` 또는 `themeId`를 변경하고 저장합니다. 별도 사례로 두 창을 연 뒤 한 창의 설정 화면에서 언어를 바꿉니다.
- **기대·실제:** 저장한 언어·테마가 해당 화면과 다른 창에 반영되어야 하지만 기존 언어·색상 캐시가 남습니다. 다른 창은 변경된 설정값을 받아도 실제 문구가 이전 언어로 유지됩니다.
- **원인·위치:** [app/commands.rs:60](/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/app/commands.rs:60)는 `SettingsChanged`만 발생하는 공통 저장 경로를 사용합니다. [ipc-sync-provider.tsx:405](/Users/hyunseokbyun/development/TAIDE/src/app/providers/ipc-sync-provider.tsx:405)는 Settings와 설정 JSON만 갱신합니다. 실제 화면이 읽는 [locale.query.ts:9](/Users/hyunseokbyun/development/TAIDE/src/entities/locale/locale.query.ts:9), [theme.query.ts:15](/Users/hyunseokbyun/development/TAIDE/src/entities/theme/theme.query.ts:15)는 별도 키와 무한 staleTime을 사용합니다.
- **검증:** 저장 → 이벤트 → 캐시 → Provider 경로를 대조했습니다. [settings.query.ts:33](/Users/hyunseokbyun/development/TAIDE/src/entities/settings/settings.query.ts:33)의 추가 무효화는 설정 화면에서 변경을 시작한 창에서만 실행됩니다. 별도 ThemeChanged를 내보내는 테마 선택 기능과 SyncStateChanged를 받는 동기화 다운로드는 이 문제에서 제외합니다. 실제 두 창 UI 재현은 수행하지 않았습니다.
- **수정 방향:** 설정 변경을 수신하는 공통 경로에서 언어·테마 관련 변경을 감지해 모든 창의 관련 캐시를 갱신해야 합니다.

## 6. 검증 결과와 범위

### 기존 검사

| 실행 | 결과 |
| --- | --- |
| `bun test` | 2,933개 통과, 실패 0개, 290개 파일, assertion 6,477개 |
| `bun run typecheck` | 종료 코드 0 |
| `cargo test --workspace --offline` | 라이브러리 1,726개 통과, 환경 권한 관련 7개 실패 후 중단 |
| 위 Rust 실패 7개만 권한 허용 환경에서 재실행 | 7개 모두 통과 |
| Rust 나머지 통합 테스트와 CLI 별도 실행 | 통합 15개, CLI 17개 통과 |

Rust의 확인된 기존 테스트는 합계 **1,765개 통과**입니다. 최초 workspace 실행이 한 번에 통과한 것은 아닙니다. 처음 실패한 7개는 프로세스 조회 1개, 로컬 서버 소켓 3개, 휴지통 이동 3개로 권한을 허용한 동일 검사에서 모두 통과했으므로 제품 버그 목록에서 제외했습니다. 기존 테스트 통과는 위 14개 시나리오가 정상이라는 의미가 아닙니다.

나머지 Rust 검사는 다음 명령으로 실행했습니다.

```text
cargo test -p taide --offline --test domain_boundaries --test capability_symmetry --test session_restore
cargo test -p taide-cli --offline
```

### 추가 재현

실제 Rust 서비스, 실제 TypeScript 편집 함수·원격 클라이언트, 저장 훅, WebKit iframe에 임시 입력을 넣었습니다. 임시 테스트의 assertion은 **현재 결함이 나타나는 결과**를 검증한 것으로, 버그 수정 완료를 뜻하지 않습니다. Git 스테이징 재현은 초기의 “오류 반환” 가설이 틀려 실제 관찰값인 “성공 반환·인덱스 누락”으로 정정한 뒤 해당 사례만 재검사했습니다.

재현용 파일은 `/tmp/taide-audit-*`에 두었으며 저장소의 임시 Rust 테스트는 점검 후 제거했습니다. 제품 소스·기존 테스트는 변경하지 않았습니다. 임시 실행 로그는 세션 보조 자료이며, 영구 재현 근거는 이 문서의 입력·기대값·관찰값·코드 위치입니다.

### 검토 범위와 남은 실기 검증

- 파일 탐색·이름 변경·삭제·저장·미저장 복원·레이아웃, 검색·Git, 터미널·언어 서버, 설정·동기화·원격 접근·미리보기의 사용자 경로와 관련 테스트를 검토했습니다. 에이전트·AI·CLI·설치 경로도 코드와 기존 테스트 범위에서 확인했습니다.
- 실제 네이티브 앱의 전체 사용자 시나리오, 두 창 간 조작, 실제 네트워크 장애, 외부 언어 서버 조합은 실행하지 않았습니다. 설치된 환경별 차이는 후속 실기 검증 대상입니다.
- 실제 GitHub 동기화 계정, AI 공급자, 원격 로그인·사용자 시크릿은 읽거나 조작하지 않았습니다. 인증된 실행 앱을 전제로 하는 E2E 묶음은 실행하지 않았습니다.
- 정규식 치환의 캡처 참조와 `**` 검색 패턴은 현재 구현의 동작을 확인했지만 지원 계약이 불명확해 확정 버그에서 제외했습니다. 프로젝트 밖 저장 제한과 HTML 스크립트 차단도 보안 정책과 구분해 제외했습니다.
- 이번 결과는 현재 코드와 위 검증 범위에서 확인한 목록입니다. 모든 운영 환경과 입력에서 추가 버그가 없음을 보장하는 전수 실행 결과는 아닙니다.
