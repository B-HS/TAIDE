# 배치 23 — AI 인라인 완성

원본 `src/shared/lib/ai/inline-completion.ts`의 자동 요청 300ms, prefix/suffix 4000/2000 UTF-16 단위, 최대 50개 캐시, provider/model/token 조건, 취소·빈 응답/오류 처리와 `code-editor.tsx`의 aiAutoTabEnabled·inlineSuggest 소비를 확인했습니다. 설치된 Monaco의 `inlineSuggestionItem.js`/`inlineCompletionsSource.js`의 forward stability와 기존 native completion/ghost/입력·스니펫/문서 저널 소비를 대조했습니다. 기존 Rust `ai_inline_complete`, `ai_token_status`, `AiRequestStore`와 설치된 Tokio watch/oneshot 및 TaskSupervisor의 transient 작업을 재사용하며 새 의존성·manifest/lock·engine·동결 소스를 추가하지 않습니다.

새 `editor-ai-completion.rs`는 현재 문서/revision/언어·뷰/owner/선택·AI 설정과 요청 ID를 함께 보유합니다. 변경과 닫힘/비활성/IME·readonly·종료에서 이전 응답을 버리고 watch 취소로 대기·실행 future를 회수합니다. backend future가 취소될 때 AiRequestToken의 기존 RAII가 요청을 해제합니다. 실제 앱에는 기존 completion State/Provider 및 Application reconcile을 통해 연결합니다. 캐시의 키는 해시 충돌 없는 제한된 원문·경로/언어·provider/model/base URL이며 최대 50개, 각 응답은 32KiB 이하입니다. 캐시 응답도 runtime token 상태 확인 이후에 표시합니다.

기존 GhostText/ViewData와 본문 projection을 사용해 제안의 여러 줄·다중 커서를 표시하며 수락 전 원문을 수정하지 않습니다. 일반 suggest preview 설정과 AI 표시를 분리했습니다. 기존 후보와 스니펫이 Tab 입력에서 우선하며 실제 후보가 없는 준비 상태에서는 AI 수락으로 전달합니다. Tab 수락은 기존 replace_selections/undo를 이용하고 Escape는 원문을 보존합니다. 제안과 일치하는 한글/보충 문자 입력은 bounded change journal의 삽입만 검증해 남은 제안을 유지합니다. 후보 목록 내부 정리가 AI 제안을 취소하지 않도록 명시적 취소·포커스/IME 취소를 분리했습니다.

초기 focused.log는 private byte_to_char 접근으로 컴파일 exit101·검사0건이었습니다. 공개 Rope 경계 검사로 수정했습니다. focused-fixed.log는 6통과/1실패·1.92초·컴파일30.28초이며 실패는 원본 CRLF를 LF로 가정한 fixture 기대값입니다. 원문 보존을 확인해 검사 기대값만 수정했습니다. extended.log/consumer.log의 Preedit struct 형식/Id.response 참조 오류는 검사 작성 단계의 컴파일 실패이며 제품 성공으로 집계하지 않습니다.

final-focused.log는 11통과/2실패·38.45초·컴파일27.83초입니다. 실제 앱의 주입 응답→표시/Tab/undo·디스크 원문·종료 task0와 문맥/설정/취소·IME/캐시/다중 커서는 통과했습니다. readonly raw 입력의 기존 ReadOnly 오류를 fixture가 검사 오류로 거부한 것은 소비 계약을 잘못 구성한 준비 오류라 readonly 명령/요청/비삽입 경계로 수정했습니다. 일치 입력의 제안 손실은 한 입력의 여러 저널 항목과 후보 목록 close를 대조해 수정했습니다. consumer-fixed.log는 8통과/1실패·3.83초이며 남은 제안은 유지했으나 빈 후보 준비 상태가 Tab을 들여쓰기로 전달했습니다. 실제 후보 부재일 때 AI 입력으로 전달하도록 수정했고 consumer-final.log의 9건이 3.79초·컴파일12.90초·exit0으로 통과했습니다.

검증 로그 접두사는 `/private/tmp/taide-ai-inline-`입니다. Cargo는 앞 실행의 terminal 종료를 확인한 뒤 직렬로 진행하며 시간 초과를 이유로 실행을 재시작하지 않습니다. 실제 앱과 세부 입력은 임시 프로젝트·주입 port·egui 이벤트만 사용하며 실제 키/Keychain/사용자 데이터/OS 설정/합성 OS 입력은 접근하지 않습니다.

현재 editor-77은 부분 구현입니다. 최초 포커스/클릭 이동 및 명시적 인라인 요청, 추가 split/peek/선택/포커스 수명과 원본 소비 분기·전체 게이트를 더 확인해야 합니다. editor-78 인라인 편집과 editor-80 Emmet은 미구현이며 Markdown editor-72/행·열/HTML·접근성·전체 App 준비 실패와 실기/성능/신뢰성/전환/제거 게이트도 유지합니다. 완료율은 302/588(51.4%), 전체 잔여 시간은 미산정입니다.

completion-regression.log는 editor_completion 관련 62건·8.94초·exit0입니다. 기존 후보/문서/스니펫/CLIPBOARD 공급·키맵·실제 child/취소/종료와 새 AI 소비자 9건을 포함하며 반복 성공을 더하지 않습니다. ui-all.log의 --all-targets --features inspection --no-fail-fast는 29대상/520통과·0실패·0ignored·exit0입니다. 문맥/설정3·App1의 변경 없는 성공과 App completion62를 합친 App의 서로 다른66건, UI520건의 서로 다른586건입니다. 변경한 Rust10개 fmt check와 소유 diff check도 exit0입니다. 전체 App 대상 실행/이전 준비 실패 해결이나 배치 완료로 확대하지 않습니다.

frozen-host.log의 --tests --features inspection는23.50초·exit0, frozen-wasm.log의 --lib --features canvas --target wasm32-unknown-unknown은8.70초·exit0입니다. 동결 소스/manifest/lock/engine 변경0이며 Wasm 테스트 실행이나 기존 시간 API 불일치 해결로 보고하지 않습니다. 디스크658GiB·사용65%를 확인했으며 청소하지 않았습니다. 현재 기능표599행/431근거 경로·complete302/partial91/unwired110/missing85·기능 분모588입니다. 미완료286 ID의 누락/중복/미확인0, 배치23~30 잔여19/52/23/64/32/39/51/6·최종33을 확인했습니다. 기존 전체App877통과/22준비실패/보호3제외/exit101의 원인 미확정과 전체 목표는 유지합니다.

소유 구현/검사/QA와 PROCESS 상단12경로를 e03aece9로 선별 커밋했습니다. 사용자 PROCESS 하단2hunk와 HANDOFF/architecture/이전 결정·피드백/메모리 변경을 보존했으며 AI 트레일러/force push는 사용하지 않습니다. 기능표는 이 실제 제품 커밋을 baseline으로 갱신하고 editor-77만 부분 구현으로 변경합니다. 전체 완료302/588(51.4%)와 미완료286은 유지합니다.

## 자격 증명 변경의 현재 제안·요청·캐시 회수 (2026-10-13)

삭제 뒤 현재 ghost가 남는 실패를 credential-red.log에서 실제 native consumer의0통과/1실패·0.37초·컴파일20.25초·exit101로 재현했습니다. taide-infra의 SecretStoreState는 기존 tuple/clone/new/typed port 계약을 유지하면서 성공한 set/delete의 계정별 변경 식별자만 공유합니다. revision 조회는 값/Keychain을 읽지 않으며 실패한 쓰기·다른 계정은 기존 식별자를 보존합니다. SecretStore 기존 구현에는 기본 revision None을 제공하고 production new와 주입 with_store는 같은 관찰 port를 사용합니다. 이 계약은 이 shared store를 통한 변경이며 외부 OS의 직접 credential 변경 감시로 확대하지 않습니다.

AI의 현재 요청/제안/캐시는 선택한 계정 식별자에 묶고 observe/reuse·Application reconcile·표시/Tab 직전에 확인합니다. 이전 식별자 응답은 버리고 대기 future를 회수하며 다른 계정 변경은 보존합니다. 최초 응답/캐시의 runtime token 상태 확인도 유지합니다. 실제 앱의 secret port를 초기화 시 주입할 수 있도록 기존 new를 동일 초기화 경로에 위임했습니다. 메모리 port를 주입한 별도 생성 경로와 기존 생성 경로를 실제 앱 두 검사에서 확인했으며 공개 new 인수와 IPC DTO는 변경하지 않았습니다.

credential-consumer.log의12통과·5.07초·컴파일76초·exit0은 삭제/교체·다른 계정·캐시·대기 응답 취소 및 기본 AI9건입니다. credential-infra.log는 기존 메모리4와 값 읽기 금지·실패 쓰기/삭제·clone 공유/계정 독립1의5통과·0.00초·컴파일56.88초·exit0입니다. credential-app.log는 실제 App2/문맥·설정3/consumer12의17통과·48.28초·컴파일23.72초·exit0입니다. 기존 생성의 표시/Tab/undo/디스크와 새 메모리 생성의 runtime ai_clear_token→표시 회수/원문·디스크 보존/정상 종료task0을 확인했습니다. 현재 서로 다른22건이며 consumer12의 반복 성공을 더하지 않습니다. 실제 Keychain/키/사용자 데이터/OS/클립보드를 읽거나 변경하지 않았습니다.

소유7Rust fmt/diff는exit0이고 manifest/lock/동결 source 변경0입니다. e1b의 credential 수명은 이 검증으로 닫으며 e1/editor-77은 최초 포커스/클릭/명시 요청·추가 split/peek/선택/포커스·전체 게이트가 남아 partial입니다. editor-78/80, Markdown HTML/표/접근성, 이전 전체App 준비 실패와 실기/성능/신뢰성/전환/제거·최종배치33도 미완료입니다. 전체302/588(51.4%), 미완료286, 잔여시간 미산정을 유지합니다.

공개 계약 검사 중 Cargo54236/rustc54244의 생존과 약6분의 S/CPU0 상태를 읽기 전용으로 확인했습니다. 해당 rustc의 1초 sample 요청 시 이미 종료되어exit255였고 스택 파일은 생성되지 않았습니다. 대기 원인이나 무한 실행으로 결론 내리지 않으며 원래 Cargo handle96561을 계속 관찰했습니다. 새 실행·기한 확대·프로세스 종료·OS 변경은 하지 않았습니다.

credential-contract.log의 App --tests --features inspection compile는15분13초·exit0입니다. 공개 new/tuple/clone/SecretStore 기본 메서드와 외부 검사 대상 타입 계약을 확인했습니다. 기존 editor-code-actions.rs is_applying 미사용 경고1과 vendor 경고를 구분하며 전체 App 테스트 실행/준비 실패 해결로 보고하지 않습니다.

credential-frozen-host.log의 --tests --features inspection과 credential-frozen-wasm.log의 --lib --features canvas --target wasm32-unknown-unknown은 각각0.08/0.10초·exit0입니다. 캐시된 현재 그래프를 검사했으며 동결 소스/manifest/lock/engine 변경0입니다. UI 제품 소스와 API는 이번 수정에서 변경하지 않았고 기존 UI 전체29대상/520의 해당 소비 근거를 유지합니다. 현재 직접 실행한 서로 다른22건과 과거520건을 새 전체 실행으로 합산하지 않습니다. 디스크655GiB·사용65%이며 청소하지 않았습니다.

자격 증명 수정·검사·QA와 소유 PROCESS 상단9경로를0330abc0로 선별 커밋했습니다. 하단 PROCESS2hunk와 사용자 문서/결정/피드백/메모리는 보존했습니다. 기능표 baseline을 실제 제품 커밋으로 갱신하고 editor-77의 credential 미완료 문구만 현재 검증에 맞게 제거하며 partial/전체302완료는 유지합니다.
