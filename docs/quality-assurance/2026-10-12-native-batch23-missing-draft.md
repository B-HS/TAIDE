# 배치 23 삭제 source draft 구제

기준: 2026-10-12, 코드7d8f9724. 서브에이전트/workflow 없이 실제 원본/코드를 대조합니다. editor-64 partial·기능 체크리스트301/588(51.2%)·미완료287·배치23 잔여20/최종33이며 전체전환율/잔여시간은 미산정입니다.

- [x] 원본 소비 — editor-pane.tsx:202의 Save As는 취소 때 상태를 유지하고 mirror 내용을 저장합니다. 성공 뒤 source mirror를 정리/조회 무효화하고 다른 경로면 기존 source 탭을 유지해 파일 탭을 엽니다. 오류는 toast이며 삭제 source 표면은 readonly pre입니다. layout.query.ts:248의 useOpenFileTab은 현재 창/기존 파일 탭 pane을 명시적으로 해석합니다. 원본의 비동기 mirror clear 오류 무시를 새 native에 강제 복제하지 않습니다.
- [x] 기존 native 경계 — missing_draft.rs의 prepare는 소유/실제 missing source/일치 mirror·크기·guard/lease/shutdown을 검증합니다. save는 source/mirror/tab/target 소유·destination mirror/dirty 탭을 확인하고 atomic write→open_file→source mirror clear→layout publish를 수행합니다. 실제 Host 경계 검사는 이미 있으나 App의 picker/표면·취소/실패/재열기까지 증명하지 않습니다.
- [x] 실제 소비/남은 차이 — App은 MissingDraft를 raw readonly 내용·sourceDeleted/saveDraftAs로 그립니다. save_missing_draft는 OS picker를 직접 열고 target live dirty 모델/소유/UTF-8·flush를 검증한 뒤 Host를 호출합니다. save 성공은 source missing 캐시를 정리하고 source 탭을 유지합니다. backend의 새 대상은 layout.focused_pane(주 영역)에 열리며 source 탭의 보조 창/기존 destination 탭과 같은 canonical 경로를 제대로 구분하는지 실제 재현이 필요합니다.
- [x] 실제 실패/표면 — 임시 프로젝트에 mirror를 저장하고 source만 삭제해 실제 App의 보조 영역 읽기 전용 표면·Host 저장 결과의 대상 pane을 확인합니다. 실제 source/사용자 데이터/OS 대화상자/클립보드/Keychain/Trash를 건드리지 않습니다.
- [x] picker/취소·오류/성공 — 두 기존 저장 호출의 동일한 picker 요청을 주입 가능하게 연결하고 실제 egui 버튼/취소·dirty destination·root 거절/변경 mirror·정상/동일 source 복구·기존 대상 재사용을 확인합니다. 기존 OS 호출의 부모/경로/이름/title 설정과 저장 검증을 유지합니다.
- [x] 관련 검증·완료 기록 — 현재 App/Host/missing_draft·untitled 및 필요 범위의 위험 비례 검사를 실행하고 같은 성공을 재사용합니다. 실제 기능이 닫힌 경우만 editor-64/배치 잔여를 갱신하고 선별 커밋/일반 푸시합니다. 전체 OS/접근성/성능/출시 게이트는 유지합니다.

첫 missing-draft-red.log는 컴파일6분29초 뒤 missing source draft 준비에서0통과/1실패·43.66초·exit101입니다. fixture가 helper 실행 경로에서 raw temp 경로를 재구성해 실제 canonical peek 경로와 다른 캐시 키를 사용한 가능성을 확인하고, 전달받은 canonical current 파일의 sibling 경로를 사용하도록 수정했습니다. 저장 대상 pane 단언에는 도달하지 않았으므로 제품의 창 누락 재현으로 세지 않습니다. 컴파일 중 소유 Cargo/rustc가 실제 살아 있고 elapsed4분47초/CPU2.06초인 상태를 읽었으며 재시작/OS 변경/캐시 수정은 하지 않았습니다. 짧은 rustc sample은 symbols 처리 단계에서 아직 완료되지 않아 원인 증명으로 사용하지 않습니다.

missing-draft-red-canonical.log는 실제 readonly 초안 표면/원문 보존과 저장 성공 뒤 대상 탭이 source 보조 창 밖에 열린 단언에서0통과/1실패·23.16초·exit101입니다. 현재 창의 source pane/기존 canonical 목적지 탭을 찾고 canonical 동일 경로는 source 탭 자체를 복구하도록 수정했습니다. 같은 App의 green.log는1통과·23.57초·exit0(컴파일26.07초)로 실제 보조 영역의 대상/원래 탭·디스크/새 본문/정리/원문을 확인했습니다. 완료된 rustc sample에서는 proc-macro dlopen→dyld mapSegments/fcntl 대기가 관찰됐으나 OS/캐시 원인을 해결한 것으로 확대하지 않습니다.

두 기존 Save As의 동기 picker 호출은 동일한 요청/부모 frame과 기본 rfd 실행을 유지하는 private 입력 포트로 연결했습니다. picker.log의 실제 버튼 취소·동일/다른 경로 저장/원래 탭·본문/닫힘/재열기/작업0은1통과이고, dirty live 목적지/root 거절·디렉터리 쓰기 오류/바뀐 mirror를 보호한 뒤 latest draft refreshed 준비가 실패해1실패·33.88초·exit101입니다. 이는 picker 후 새로운 mirror를 backend가 거절했으나 App이 오래된 readonly 내용을 계속 보유하는 실제 복구 누락입니다. 오류 뒤 기존 OpenDocument의 소유/guard 경계로 초안을 재조회하고 재조회 중 picker를 막아 최신 상태의 재시도를 연결합니다. 검사에서는 재조회 종료를 확인한 후 다음 mirror 변경을 접수하며 오류 한 건만 재검사 중입니다.

picker-errors-fixed.log는 같은 실제 버튼 오류 검사에서1통과·23.02초·exit0(컴파일22.65초)입니다. dirty live 목적지/root 거절과 쓰기 오류는 원래 파일/초안을 유지합니다. 바뀐 mirror는 덮어쓰지 않고 재조회 뒤 최신 readonly 내용을 보여 주며 다시 실제 버튼을 눌러 최신 내용을 저장합니다. 재조회 중에는 picker를 열지 않아 진행 중 admission guard를 보존합니다. 성공한 동일/다른 경로 UI 검사와 보조 창 저장 검사는 source/입력이 같은 성공을 재사용합니다.

새 문서의 같은 picker 입력 포트 검사 첫 untitled-picker.log는 NewUntitled.pane을 기존 typed PaneId 대신 None으로 작성한 E0308·실행0건·exit101입니다. 실제 focused pane과 live view 준비 조건을 사용하도록 수정했으며 제품의 저장 실패로 세지 않습니다.

untitled-picker-fixed.log는 저장 후 App files 캐시의 준비 조건에서0통과/1실패·49.11초·exit101(컴파일20.38초)입니다. 기존 UntitledSaved 소비자는 canonical destination으로 files를 등록하므로 fixture의 raw temp 목적지 대신 전달받은 canonical 파일의 sibling 경로로 수정했습니다. source 계약/제품 저장 동작을 변경하거나 준비 기한을 늘리지 않았으며 재검사 중입니다.

untitled-picker-canonical.log는 실제 새 문서의 같은 picker 설정/취소·dirty 내용 유지·정상 저장·본문 전환·원문/작업0이1통과·22.93초·exit0(컴파일14.62초)입니다. 첫 경로 준비 실패를 성공으로 합산하지 않습니다. 현재 실제 App의 보조 창 저장/버튼 취소·동일/다른 경로·재열기/오류 보호·최신 mirror 재조회/재시도와 untitled 공유 picker의 서로 다른4건이 통과했으며 source/입력이 같은 앞선 성공은 재사용했습니다. 소유6Rust fmt/diff는 exit0입니다. 추가 canonical 동일 경로/현재 창의 기존 목적지 탭 재사용과 기존 missing_draft/untitled 서비스 검사를 확인 중입니다. 새 dependency/manifest/lock/engine/frozen source 변경은 없습니다.

services.log는 기존 missing_draft4통과/새 기존 대상 재사용1실패·0.09초·exit101입니다. canonical 동일 경로의 source 탭 복구와 기존 목적지의 id/pin/view_state·중복0은 성공했으나 기존 open_tab API가 해당 pane을 focus하지 않아 기대 pane이 달랐습니다. 동일 창에서 반환된 대상 tab을 기존 activate_tab으로 활성화하도록 연결했습니다. 원래 저장/소유/mirror 보호는 유지하며 실패 한 건만 확인합니다. 첫 대상 실패로 untitled target은 실행되지 않았으므로 별도 실행 후 보고합니다.

reuse-fixed.log는 canonical 동일 source 경로와 현재 창의 기존 목적지 탭 재사용/활성화·id/pin/view_state/중복0/원문·mirror 정리/작업0이1통과·0.04초·exit0(컴파일19.04초)입니다. 기존 missing_draft4성공은 변경 없는 실패 경계를 재사용합니다. untitled-service.log는 이전 target 실패 때문에 미실행이었던 기존 새 문서 전체3통과·0.06초·exit0(컴파일4.93초)입니다. 현재 실제 App4/missing_draft5/untitled3의 서로 다른12건·소유6Rust fmt/diff가 통과했습니다. 같은 검사/재검사/준비 실패/0건 결과를 중복 집계하지 않습니다.

editor-64의 원본 readonly 삭제 source 표면과 Save As 취소/오류·현재 창/기존 대상·동일 canonical source 복구·원래 탭/원문 유지·성공 mirror 정리·최신 draft 재조회/재시도·본문/닫힘/재열기·소유 회수는 실제 App/서비스 근거로 complete입니다. 두 기존 picker의 OS 부모/요청 설정/default rfd는 보존하고 실제 OS 대화상자는 주입 포트로 대신했습니다. 물리 OS picker·전체App 준비 실패/접근성/성능/신뢰성/출시와 나머지 목표는 미완료입니다. 기능표 반영 뒤 complete302/partial89/unwired110/missing87·588대상·미완료286·기능 체크리스트51.4%·배치23 잔여19/최종33이며 전체전환율/신뢰할 잔여시간은 미산정입니다.

삭제 초안 코드/검사·QA/소유 PROCESS의8경로를 `fc732905`로 선별 커밋했고 author/최종 메시지에 AI 트레일러가 없음을 확인했습니다. 기능표599행/409근거 경로·302완료/286미완료·원래302 ID 배정의 누락/중복/미확인ID0·배치23~30 잔여19/52/23/64/32/39/51/6·최종33을 실제 JSON으로 대조했습니다. 소유 staged diff exit0이며 사용자 기존 문서와 PROCESS 하단 두 hunk는 스테이징하지 않았습니다. 남은 목표 전체와 기존 준비/실기/출시 게이트는 유지합니다.
