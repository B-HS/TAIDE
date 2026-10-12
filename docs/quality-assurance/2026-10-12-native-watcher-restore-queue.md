# 초기 워처 복원과 호스트 요청 처리

현재 상태: 단일 Host 큐의 워처 복원 대기를 재현해 감독되는 배경 작업으로 분리했습니다. 결정적 Host 회귀 1건과 수정 뒤 실제 IDE App 3건이 통과했습니다. App 전체는 67대상/877통과/22실패·보호 3제외·exit 101이며 그중 Host 9건은 통과했습니다. 전체 준비 단계 실패와 기존 프로젝트 open 자체의 OS 등록 변동은 별도 미해결입니다.

- [x] a. RestoreWatchers의 후속 요청 차단 재현
- [x] b. 감독되는 배경 복원·오류/종료 회수·준비 공백 rescan 연결
- [x] c. 등록 보류 중 후속 요청·작업 회수 및 실제 App 확인
- [x] d. 공용 Host/초기화의 전체 회귀 결과와 선별 Git 저장

## 원인과 수정

`NativeApplication::new`는 첫 화면 전에 `RestoreWatchers`를 먼저 제출합니다. 기존 `host.rs`의 단일 worker는 `NativeProjects::restore_watchers`가 OS watcher 생성과 등록을 마칠 때까지 다음 명령을 처리하지 않았습니다. `projects.rs`는 watcher 생성 자체를 mutation guard 밖에서 수행하지만 Host 처리 순서는 계속 막혀 `OpenDocument` 같은 후속 요청도 기다렸습니다. 선택 검사들의 공용 document 단계 실패와 IDE 전송의 실패를 구분합니다.

복원은 기존 TaskSupervisor의 transient 작업으로 감독하며 실패는 같은 HostReply::Failed 채널에 전달합니다. 종료/프로젝트 닫힘은 기존 attach의 생존 재검증과 감독 작업·watcher stop 회수를 사용합니다. 준비가 끝난 프로젝트에 FsRescanRequired와 Git 상태 무효화를 발행해 초기 파일 읽기 뒤 watcher 준비 전까지의 변경을 기존 관찰 경로로 다시 확인합니다. 기한 확대·OS 설정 변경·실제 데이터/클립보드 사용·새 의존성은 없습니다.

## 실제 검증

`tests/host.rs`는 임시 프로젝트의 등록 mutation guard를 보류하고 RestoreWatchers 뒤에 주입 클립보드 요청을 제출합니다. OS watcher 속도와 관계없이 구 버전은 큐 순서 때문에 후속 요청을 전달할 수 없습니다. 수정 뒤에는 같은 3초 상한 안에 후속 요청을 전달하고 guard 해제·Host 종료·TaskSupervisor/WatcherStopTracker 회수·작업 0을 확인합니다.

`/private/tmp/taide-batch23-watcher-restore-host-red.log`: 0통과/1실패·12.20초·exit 101입니다. `watcher registration held the Host queue` 단언에서 실패했습니다. 검사 후 정리는 완료했습니다.

첫 green 시도의 필터 이름 오타는 0건 실행이라 성공 근거에서 제외합니다. 정확한 이름으로 실행한 `watcher-restore-host-final.log`는 1통과/0실패·7.17초·exit 0(컴파일 0.31초)입니다. 모든 Cargo는 locked/offline과 기존 공용 target을 사용했습니다.

수정 뒤 `ide-selection-final-related.log`는 Cargo 실행에서도 실제 readonly/입력 잠금·보조 창/peek/AppFile/untitled·대기/거절/숨긴 뷰/Drop/동일 파일 소유를 포함한 3통과/0실패·36.81초·exit 0(컴파일 24.65초)입니다. 수정 전 같은 검사나 직접 cwd 성공은 다시 합산하지 않습니다. 문서 초기화 실패의 후속 큐 대기는 해결했지만 `NativeProjects::open`이 자체적으로 OS 등록을 기다리는 기존 f1의 모든 원인이 해결됐다고 주장하지 않습니다.

`ide-selection-app-all-targets.log` 전체 실행은 종료됐습니다. Host 9통과/0실패·6.76초·exit 0에 새 결정적 회귀가 포함되므로 앞선 성공을 추가 합산하지 않습니다. 전체 App은 877통과/22실패이며 provider 준비 16·프로젝트 준비 6건의 실패를 유지합니다. 두 target의 exit 101을 전체 성공으로 바꾸지 않고 대표 provider 준비 상태를 추가 진단합니다.

구현·회귀·QA 4경로는 `143bb85b`로 선별 커밋했습니다. IDE Host variant와 사용자 기존 문서 변경은 포함하지 않았습니다. 대표 provider의 같은 입력/기한 단독 검사는 1통과·17.88초·exit 0이며 최초 준비 실패의 원인은 미확정입니다. 전체 성공이나 프로젝트 open의 모든 지연 해결로 바꾸지 않습니다. 나머지 관련 저장이 끝난 뒤 일반 푸시합니다.
