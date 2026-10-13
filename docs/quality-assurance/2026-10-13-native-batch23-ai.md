# 배치 23 — AI 인라인 완성

원본 `src/shared/lib/ai/inline-completion.ts`의 자동 요청 300ms, prefix/suffix 4000/2000 UTF-16 단위, 최대 50개 캐시, provider/model/token 조건, 취소·빈 응답/오류 처리와 `code-editor.tsx`의 aiAutoTabEnabled·inlineSuggest 소비를 확인했습니다. 설치된 Monaco의 `inlineSuggestionItem.js`/`inlineCompletionsSource.js`의 forward stability와 기존 native completion/ghost/입력·스니펫/문서 저널 소비를 대조했습니다. 기존 Rust `ai_inline_complete`, `ai_token_status`, `AiRequestStore`와 설치된 Tokio watch/oneshot 및 TaskSupervisor의 transient 작업을 재사용하며 새 의존성·manifest/lock·engine·동결 소스를 추가하지 않습니다.

새 `editor-ai-completion.rs`는 현재 문서/revision/언어·뷰/owner/선택·AI 설정과 요청 ID를 함께 보유합니다. 변경과 닫힘/비활성/IME·readonly·종료에서 이전 응답을 버리고 watch 취소로 대기·실행 future를 회수합니다. backend future가 취소될 때 AiRequestToken의 기존 RAII가 요청을 해제합니다. 실제 앱에는 기존 completion State/Provider 및 Application reconcile을 통해 연결합니다. 캐시의 키는 해시 충돌 없는 제한된 원문·경로/언어·provider/model/base URL이며 최대 50개, 각 응답은 32KiB 이하입니다. 캐시 응답도 runtime token 상태 확인 이후에 표시합니다.

기존 GhostText/ViewData와 본문 projection을 사용해 제안의 여러 줄·다중 커서를 표시하며 수락 전 원문을 수정하지 않습니다. 일반 suggest preview 설정과 AI 표시를 분리했습니다. 기존 후보와 스니펫이 Tab 입력에서 우선하며 실제 후보가 없는 준비 상태에서는 AI 수락으로 전달합니다. Tab 수락은 기존 replace_selections/undo를 이용하고 Escape는 원문을 보존합니다. 제안과 일치하는 한글/보충 문자 입력은 bounded change journal의 삽입만 검증해 남은 제안을 유지합니다. 후보 목록 내부 정리가 AI 제안을 취소하지 않도록 명시적 취소·포커스/IME 취소를 분리했습니다.

초기 focused.log는 private byte_to_char 접근으로 컴파일 exit101·검사0건이었습니다. 공개 Rope 경계 검사로 수정했습니다. focused-fixed.log는 6통과/1실패·1.92초·컴파일30.28초이며 실패는 원본 CRLF를 LF로 가정한 fixture 기대값입니다. 원문 보존을 확인해 검사 기대값만 수정했습니다. extended.log/consumer.log의 Preedit struct 형식/Id.response 참조 오류는 검사 작성 단계의 컴파일 실패이며 제품 성공으로 집계하지 않습니다.

final-focused.log는 11통과/2실패·38.45초·컴파일27.83초입니다. 실제 앱의 주입 응답→표시/Tab/undo·디스크 원문·종료 task0와 문맥/설정/취소·IME/캐시/다중 커서는 통과했습니다. readonly raw 입력의 기존 ReadOnly 오류를 fixture가 검사 오류로 거부한 것은 소비 계약을 잘못 구성한 준비 오류라 readonly 명령/요청/비삽입 경계로 수정했습니다. 일치 입력의 제안 손실은 한 입력의 여러 저널 항목과 후보 목록 close를 대조해 수정했습니다. consumer-fixed.log는 8통과/1실패·3.83초이며 남은 제안은 유지했으나 빈 후보 준비 상태가 Tab을 들여쓰기로 전달했습니다. 실제 후보 부재일 때 AI 입력으로 전달하도록 수정했고 consumer-final.log의 9건이 3.79초·컴파일12.90초·exit0으로 통과했습니다.

검증 로그 접두사는 `/private/tmp/taide-ai-inline-`입니다. Cargo는 앞 실행의 terminal 종료를 확인한 뒤 직렬로 진행하며 시간 초과를 이유로 실행을 재시작하지 않습니다. 실제 앱과 세부 입력은 임시 프로젝트·주입 port·egui 이벤트만 사용하며 실제 키/Keychain/사용자 데이터/OS 설정/합성 OS 입력은 접근하지 않습니다.

현재 editor-77은 부분 구현입니다. 현재 표시된 제안의 credential 변경 감지, 최초 포커스/클릭 이동 및 명시적 인라인 요청, 추가 split/peek/선택/포커스 수명과 원본 소비 분기·전체 게이트를 더 확인해야 합니다. editor-78 인라인 편집과 editor-80 Emmet은 미구현이며 Markdown editor-72/행·열/HTML·접근성·전체 App 준비 실패와 실기/성능/신뢰성/전환/제거 게이트도 유지합니다. 완료율은 302/588(51.4%), 전체 잔여 시간은 미산정입니다.

completion-regression.log는 editor_completion 관련 62건·8.94초·exit0입니다. 기존 후보/문서/스니펫/CLIPBOARD 공급·키맵·실제 child/취소/종료와 새 AI 소비자 9건을 포함하며 반복 성공을 더하지 않습니다. ui-all.log의 --all-targets --features inspection --no-fail-fast는 29대상/520통과·0실패·0ignored·exit0입니다. 문맥/설정3·App1의 변경 없는 성공과 App completion62를 합친 App의 서로 다른66건, UI520건의 서로 다른586건입니다. 변경한 Rust10개 fmt check와 소유 diff check도 exit0입니다. 전체 App 대상 실행/이전 준비 실패 해결이나 배치 완료로 확대하지 않습니다.

frozen-host.log의 --tests --features inspection는23.50초·exit0, frozen-wasm.log의 --lib --features canvas --target wasm32-unknown-unknown은8.70초·exit0입니다. 동결 소스/manifest/lock/engine 변경0이며 Wasm 테스트 실행이나 기존 시간 API 불일치 해결로 보고하지 않습니다. 디스크658GiB·사용65%를 확인했으며 청소하지 않았습니다. 현재 기능표599행/431근거 경로·complete302/partial91/unwired110/missing85·기능 분모588입니다. 미완료286 ID의 누락/중복/미확인0, 배치23~30 잔여19/52/23/64/32/39/51/6·최종33을 확인했습니다. 기존 전체App877통과/22준비실패/보호3제외/exit101의 원인 미확정과 전체 목표는 유지합니다.

소유 구현/검사/QA와 PROCESS 상단12경로를 e03aece9로 선별 커밋했습니다. 사용자 PROCESS 하단2hunk와 HANDOFF/architecture/이전 결정·피드백/메모리 변경을 보존했으며 AI 트레일러/force push는 사용하지 않습니다. 기능표는 이 실제 제품 커밋을 baseline으로 갱신하고 editor-77만 부분 구현으로 변경합니다. 전체 완료302/588(51.4%)와 미완료286은 유지합니다.
