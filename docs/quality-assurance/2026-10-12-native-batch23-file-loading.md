# 배치 23 파일 로딩/오류 소비 경계

기준: 2026-10-12, 코드 afbfe6c4. 서브에이전트/workflow 없이 실제 파일을 읽었습니다. editor-19/59는 partial, editor-64의 삭제 draft는 별도 범위입니다. 기능 체크리스트299/588(50.9%)·미완료289·배치23 잔여22/최종33이며 전체전환율/잔여시간은 미산정입니다.

## 원본과 실제 native 경계

- [x] 원본 화면/조회 계약 — src/widgets/editor-pane/editor-pane.tsx:363의 loading은 빈 본문, 일반 읽기 오류는 localized IPC 메시지입니다. missingSourceMirror가 있으면 읽기 전용 draft와 Save As를 보여 줍니다. :398의 refused는 editor.cannotOpen·editor.binaryOrTooLarge와 editor.openExternally 버튼이며 systemOpenPath로 기존 서비스에 전달합니다. src/entities/file/file.query.ts:53은 queryOptions/무한 staleTime이고 별도의 retry 옵션/수동 Retry 버튼이 없습니다. 새 Retry 버튼을 원본 기능으로 추정하지 않습니다.
- [x] 기존 파일 정책 — crates/taide-model/src/file.rs의 FileSizeTier와 OpenedFile을 단일 출처로 사용합니다. crates/taide-file/src/service.rs:638은 앞부분 NUL을 binary로 판정하고 :651의 refused_file은 빈 content·Refused/read_only DTO를 반환합니다. 정책의 새 임계값/전체 파일 읽기/Refused 문서 모델 생성은 도입하지 않습니다.
- [x] 실제 native 소비 — host.rs:1668의 OpenDocument는 native_file_actions::open_document_file 뒤 document_admission::prepare_opened_document를 반환합니다. document_admission.rs는 소유/metadata와 shutdown을 검증하고 lease/guard를 실제 commit까지 보관합니다. commit_with_notice의 Store는 Refused를 거절하고 AppError::Forbidden으로 변환합니다. application.rs:1532는 모든 결과를 commit_with_notice로 보내므로 refused DTO가 전용 본문까지 도달하지 않습니다.
- [x] 현재 화면/명령 — application.rs:8264의 failed 분기는 error.to_string 한 줄이고 Refused 안내/외부 열기가 없습니다. 모델 없음/loading 큐와 spinner는 기존 동작입니다. 기존 HostCommand::OpenPath (:1280)는 system_actions::system_open_path와 플랫폼 포트를 사용하며 다른 preview에서 이미 소비합니다. locale 리소스에 원본 세 메시지가 있고 crate::toast::describe_error는 기존 localized 오류 경계입니다.
- [x] 실패 탭 수명 — application.rs:2848의 release_tab은 self.files에 모델이 없으면 조기 반환합니다. failed.remove는 실제 모델 release 뒤 :2885에 있어 모델 생성에 실패한 탭의 마지막 닫힘에서 오류가 유지될 수 있습니다. 직접 App 재현 전이므로 확정된 자동 검증 결과로 보고하지 않습니다. 같은 파일의 다른 창/탭이 남으면 공유 결과를 보존하고 마지막 소비자 닫힘/프로젝트 detach·다시 열기의 기존 수명과 대조합니다.

## 구현/검증 체크리스트

- [x] 실제 App 재현 — 소유 임시 프로젝트의 NUL 파일과 빈 sparse 대형 파일만 사용해 실제 Host 열기→본문 전용 안내 부재와 정책 보호를 확인합니다. 정상 모델/원문/revision/dirty/디스크를 보존하고 실제 앱 데이터/OS 외부 앱 실행/Keychain/클립보드를 건드리지 않습니다.
- [x] typed refused 소비 — validated PreparedDocument의 typed 등급을 commit 전 경계에서 구분하고 실제 Refused 상태를 본문에 전달합니다. 문서 모델/가짜 DocumentId/오류 문자열 파싱으로 정책을 우회하지 않습니다. operation lease/guard/shutdown·소유 검증과 문서 observation/disk choice 소비를 유지합니다.
- [x] 원본 전용 안내/외부 열기 — 기존 locale/본문 배치와 OpenPath 큐를 사용하고 부수 효과는 주입 포트/기존 Host 검사로 검증합니다. 일반 오류는 기존 localized 경계와 원본 재열기/새 파일 상태 갱신을 사용합니다.
- [x] 오류/거절 상태 수명 — 실패 모델의 실제 마지막 닫힘/정상 파일로 재열기와 동일 파일 mirror/보조 영역·프로젝트 detach/오래된 응답을 필요한 최소 검사로 확인합니다. 실패의 원문 손실/반복 열기 폭주/소유 누수를 막습니다.
- [ ] 관련 위험 비례 검사·기록/선별 Git — 실제 실패 뒤 수정 영향 검사만 실행하고 기존 성공을 재사용합니다. 기능표/배치/PROCESS는 닫힌 기능 묶음만 갱신하며 전체 OS/성능/출시를 완료로 올리지 않습니다.

이번 조사는 읽기 전용 코드 대조이며 제품 코드 변경/새 테스트 실행은 없습니다. d3의 서로 다른560건 성공은 위치 저장/복원의 기존 근거이며 이 로딩 화면의 통과 근거로 대체하지 않습니다.

## 실제 App 재현과 소비 연결

첫 file-loading-red.log는 검사 코드의 PaneNode import 누락(E0433)으로 실행0건·exit101입니다. import를 수정한 red-fixed.log는0통과/2실패·30.87초·exit101(컴파일27.89초)이지만 이후 파일 응답/닫힘 응답 전 조기 단언 가능성을 발견했으므로 최종 제품 재현 근거와 구분합니다. 작성한 helper의 egui 직접 import도 실제 App manifest에 없는 의존성을 가정해 첫 green.log가 E0432·실행0건·exit101이었고 기존 eframe::egui 경계로 수정했습니다. 새 의존성/lock은 추가하지 않습니다.

green-fixed.log도0통과/2실패·37.41초입니다. 실제 loading 상태를 한번 관찰한 뒤 회수됐는지, 닫힘의 pending_tab_close까지 종료됐는지 확인하도록 검사 수명 조건을 수정했습니다. 변경된 App source를 소유 임시 파일에 보존하고 원래 HEAD App 소비자에 수정한 검사를 붙여 red-lifecycle.log에서 실제 기존 누락을 재확인 중입니다. 기존 사용자 변경은 포함하지 않았습니다. 같은 조기 단언 결과를 성공/확정 재현으로 세지 않습니다.

현재 연결은 PreparedDocument::commit_file의 typed Document/Refused 결과를 추가하며 기존 commit/observation/disk-choice 계약은 유지합니다. Refused는 lease/guard와 shutdown 검사를 거쳐 모델 없이 회수하며 일반 모델 commit을 우회하지 않습니다. App은 거절 상태를 전용 locale 안내/기존 OpenPath 큐로 전달하고 generic 오류는 기존 localized 경계를 사용합니다. 마지막 모델 없는 닫힘·live layout 소비자 제거와 기존 파일 변경/프로젝트 rescan에서 오류/거절 캐시를 회수합니다. 실제 재현/성공·mirror/큰 tier/외부 입력/회수 근거는 아직 확인 중입니다.

수명 조건을 보강한 red-lifecycle.log의 원래 App 소비자는0통과/2실패·32.95초·exit101(컴파일33.39초)입니다. 실제 loading 관찰/회수와 pending 닫힘 완료 후에도 원본 전용 안내가 없고 마지막 실패 탭의 오류 캐시가 남았습니다. 이것을 최종 제품 누락 재현 근거로 사용합니다. 보존한 변경 source를 복원하고 원본처럼 centered/error 색의 localized 일반 오류도 연결해 green-lifecycle.log를 실행 중입니다. helper 검사의 unused FullOutput 경고는 let _로 실제 반환을 처리했으며 검사기를 끄지 않았습니다.

green-lifecycle.log는 보강한 응답 완료 조건에서2통과·33.17초·exit0(컴파일32.30초)입니다. NUL/sparse Refused의 전용 세 안내·모델 미생성/원문 보존과 실제 마지막 읽기 실패 탭의 닫힘/정상 파일 재열기를 확인했습니다. additional.log는 새 실제 App2통과/버튼 helper1실패·28.31초·exit101입니다. 보조 mirror가 남은 첫 닫힘의 공유 Refused 상태, FsChanged 뒤 정상 모델/내용 복원과 실제 project_close의 거절 캐시 회수, 실제 2MiB Large/20MiB ReadOnly 텍스트 Host 로딩·본문 입력 허용/거절·banner/원문/dirty/작업0이 통과했습니다. 버튼 helper의 FullOutput texture delta를 처리하지 않아 epaint drop 검사가 실패했으며 기존 App paint처럼 textures_delta.clear로 소유 출력 회수를 수정합니다. App 성공2건을 반복하지 않고 helper 실패 한 건만 재검사합니다.

button-fixed.log는 실제 egui Pointer 누름/뗌에서 OpenPath 한 번·다음 프레임 명령0이1통과·0.01초·exit0(컴파일13.81초)입니다. OS opener는 실행하지 않습니다. admission.log는 기존 문서 commit/drop·root/metadata 변화/shutdown과 새 Refused typed commit의 NUL/threshold sparse/종료 거절·lease/guard/모델0/원문 보존3통과·0.01초·exit0(컴파일1분13초)입니다.

아주 큰 Refused 파일은 기존 OpenedFile.byte_size가 u32 최댓값으로 포화되는데 native admission은 u64 metadata.len과 그대로 비교해 정상 거절 안내를 막았습니다. 4GiB sparse 소유 파일의 saturated-red.log는 필터 작성 오류로 실행0건이며 제외합니다. 정확한 필터의 saturated-red-fixed.log는 기존 포화 크기/Refused 응답 뒤 prepare 거절에서0통과/1실패·0.00초·exit101입니다. 실제 DTO의 포화 표현으로 metadata 크기를 대조하고 기존 mtime/소유/guard 검증을 유지해 saturated-green.log가1통과·0.01초·exit0(컴파일11.13초)입니다. 파일 본문을 읽거나 모델을 생성하지 않고 lease/task0·원래 sparse 크기를 보존합니다. 실제 App 안내에도 같은 큰 사례를 추가했고 확인 중입니다.

saturated-app.log는 실제 App의 NUL/50MiB sparse/4GiB sparse 세 거절 사례에서 전용 안내·모델0/원문 보존1통과·25.10초·exit0(컴파일25.21초)입니다. 이 검사 한 건은 앞선 App 성공2건에 포함된 동일 test이므로 추가 합산하지 않습니다. byte 표현 검증 수정 뒤 admission-regression.log는 포화 사례를 제외한 원래 소유/일반 metadata/shutdown·새 Refused commit3통과·0.01초·exit0이며 포화1성공을 재사용한 서로 다른4건입니다.

host-system.log는 주입 platform의 OpenPath/URL 검증·복사 오류/worker 회수2통과·0.01초와 전체 Host9통과·0.13초·exit0(컴파일24.17초)입니다. 실제 OS opener/clipboard를 실행하지 않고 기존 승인된 경로 검증/포트를 사용합니다. 저장·symlink/소유·mirror/disk conflict·미소비 admission token·프로젝트 파일/팔레트/옆 열기·초기 watcher 대기/오류/dirty flush를 확인했습니다. 현재 새 App4/버튼1/admission4/Host9/system2의 서로 다른20건이며 기존 본문 저장/복원15건의 실제 App 회귀를 확인 중입니다. 전체 App의 기존 준비 실패/실기/성능/출시 성공을 뜻하지 않습니다.

view-state.log는 현재 실제 App 저장/복원 전체15통과·25.24초·exit0입니다. 최근 닫힘/preview·IDE/remote/project/slot·주/보조 batch·UTF-16/scroll/접기·readonly/원문/디스크·새 App 재시작과 디스크 오류 유지/재시도를 확인했습니다. 이번 실제 App4/버튼1/admission4/Host9/system2/기존 App15의 서로 다른35건·소유6Rust fmt/diff가 통과했습니다. 앞선 초기/추가/포화/재검사의 같은 test 이름은 합산하지 않습니다. 이번 검사 개수를 기존 d3 누계에 임의로 추가하지 않습니다.

editor-19의 대형 tier와 editor-59의 파일 열기/로딩/오류는 실제 원본 소비 경계와 이 자동 근거로 complete입니다. 정상/큰 편집 가능/큰 읽기 전용·NUL/threshold/포화 Refused의 비동기 처리·localized 안내/오류·기존 외부 열기 명령/경로 검증·파일 변경 복구/마지막 닫힘 재열기·공유 보조 영역/프로젝트 제거와 소유 회수를 확인했습니다. editor-64의 삭제 source draft/Save As 전체 실제 App 흐름과 전체App 준비 실패·OS launch/접근성/성능/출시 게이트는 별도 미완료입니다. 완료 판정 반영 뒤 기능표는 complete301/partial90/unwired110/missing87·588대상·미완료287·기능 체크리스트51.2%·배치23 잔여20/최종33입니다. 전체전환율/신뢰할 잔여시간은 미산정입니다.
