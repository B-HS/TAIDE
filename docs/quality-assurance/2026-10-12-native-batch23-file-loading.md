# 배치 23 파일 로딩/오류 소비 경계

기준: 2026-10-12, 코드 afbfe6c4. 서브에이전트/workflow 없이 실제 파일을 읽었습니다. editor-19/59는 partial, editor-64의 삭제 draft는 별도 범위입니다. 기능 체크리스트299/588(50.9%)·미완료289·배치23 잔여22/최종33이며 전체전환율/잔여시간은 미산정입니다.

## 원본과 실제 native 경계

- [x] 원본 화면/조회 계약 — src/widgets/editor-pane/editor-pane.tsx:363의 loading은 빈 본문, 일반 읽기 오류는 localized IPC 메시지입니다. missingSourceMirror가 있으면 읽기 전용 draft와 Save As를 보여 줍니다. :398의 refused는 editor.cannotOpen·editor.binaryOrTooLarge와 editor.openExternally 버튼이며 systemOpenPath로 기존 서비스에 전달합니다. src/entities/file/file.query.ts:53은 queryOptions/무한 staleTime이고 별도의 retry 옵션/수동 Retry 버튼이 없습니다. 새 Retry 버튼을 원본 기능으로 추정하지 않습니다.
- [x] 기존 파일 정책 — crates/taide-model/src/file.rs의 FileSizeTier와 OpenedFile을 단일 출처로 사용합니다. crates/taide-file/src/service.rs:638은 앞부분 NUL을 binary로 판정하고 :651의 refused_file은 빈 content·Refused/read_only DTO를 반환합니다. 정책의 새 임계값/전체 파일 읽기/Refused 문서 모델 생성은 도입하지 않습니다.
- [x] 실제 native 소비 — host.rs:1668의 OpenDocument는 native_file_actions::open_document_file 뒤 document_admission::prepare_opened_document를 반환합니다. document_admission.rs는 소유/metadata와 shutdown을 검증하고 lease/guard를 실제 commit까지 보관합니다. commit_with_notice의 Store는 Refused를 거절하고 AppError::Forbidden으로 변환합니다. application.rs:1532는 모든 결과를 commit_with_notice로 보내므로 refused DTO가 전용 본문까지 도달하지 않습니다.
- [x] 현재 화면/명령 — application.rs:8264의 failed 분기는 error.to_string 한 줄이고 Refused 안내/외부 열기가 없습니다. 모델 없음/loading 큐와 spinner는 기존 동작입니다. 기존 HostCommand::OpenPath (:1280)는 system_actions::system_open_path와 플랫폼 포트를 사용하며 다른 preview에서 이미 소비합니다. locale 리소스에 원본 세 메시지가 있고 crate::toast::describe_error는 기존 localized 오류 경계입니다.
- [x] 실패 탭 수명 — application.rs:2848의 release_tab은 self.files에 모델이 없으면 조기 반환합니다. failed.remove는 실제 모델 release 뒤 :2885에 있어 모델 생성에 실패한 탭의 마지막 닫힘에서 오류가 유지될 수 있습니다. 직접 App 재현 전이므로 확정된 자동 검증 결과로 보고하지 않습니다. 같은 파일의 다른 창/탭이 남으면 공유 결과를 보존하고 마지막 소비자 닫힘/프로젝트 detach·다시 열기의 기존 수명과 대조합니다.

## 구현/검증 체크리스트

- [ ] 실제 App 재현 — 소유 임시 프로젝트의 NUL 파일과 빈 sparse 대형 파일만 사용해 실제 Host 열기→본문 전용 안내 부재와 정책 보호를 확인합니다. 정상 모델/원문/revision/dirty/디스크를 보존하고 실제 앱 데이터/OS 외부 앱 실행/Keychain/클립보드를 건드리지 않습니다.
- [ ] typed refused 소비 — validated PreparedDocument의 typed 등급을 commit 전 경계에서 구분하고 실제 Refused 상태를 본문에 전달합니다. 문서 모델/가짜 DocumentId/오류 문자열 파싱으로 정책을 우회하지 않습니다. operation lease/guard/shutdown·소유 검증과 문서 observation/disk choice 소비를 유지합니다.
- [ ] 원본 전용 안내/외부 열기 — 기존 locale/본문 배치와 OpenPath 큐를 사용하고 부수 효과는 주입 포트/기존 Host 검사로 검증합니다. 일반 오류는 기존 localized 경계와 원본 재열기/새 파일 상태 갱신을 사용합니다.
- [ ] 오류/거절 상태 수명 — 실패 모델의 실제 마지막 닫힘/정상 파일로 재열기와 동일 파일 mirror/보조 영역·프로젝트 detach/오래된 응답을 필요한 최소 검사로 확인합니다. 실패의 원문 손실/반복 열기 폭주/소유 누수를 막습니다.
- [ ] 관련 위험 비례 검사·기록/선별 Git — 실제 실패 뒤 수정 영향 검사만 실행하고 기존 성공을 재사용합니다. 기능표/배치/PROCESS는 닫힌 기능 묶음만 갱신하며 전체 OS/성능/출시를 완료로 올리지 않습니다.

이번 조사는 읽기 전용 코드 대조이며 제품 코드 변경/새 테스트 실행은 없습니다. d3의 서로 다른560건 성공은 위치 저장/복원의 기존 근거이며 이 로딩 화면의 통과 근거로 대체하지 않습니다.
