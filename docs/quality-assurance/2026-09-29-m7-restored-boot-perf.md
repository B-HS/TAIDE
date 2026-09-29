# M7 합성 프로젝트 복원 부팅 단일 표본

## 대상과 방법

2026-09-29 macOS 26.5.2 arm64의 전용 identifier 계측용 release 앱을 사용했습니다. 이전 PID 41408에서 파일·검색·Git·터미널·다중 창·메뉴 검증을 마친 뒤 `⌘Q`로 한 번 종료했고, 기존 IDE listener `127.0.0.1:33832`의 종료를 확인했습니다. 같은 앱 번들을 `ZDOTDIR=/private/tmp/taide-m6-isolated.byLYaMWs/zdot`, `CLAUDE_CONFIG_DIR=/private/tmp/taide-m6-isolated.byLYaMWs/claude-release`, `TMPDIR=/private/tmp/taide-m7-fixture.csNotl`, `TAIDE_PERF=1`로 한 번 재실행했습니다. 새 프로세스는 PID 44412였습니다. 사용자 프로젝트·IDE 인증 파일 내용은 읽지 않았습니다.

## 관찰 결과

새 앱의 접근성 트리에 프로젝트 두 개, 합성 프로젝트의 `.git`·`large.txt`·`small.txt` 탐색기 항목, 재개된 Settings 탭과 TAIDE Dark 선택 상태가 나타났습니다. 합성 프로젝트 파일의 본문·미저장 탭·다른 창 전수 복원까지 확인한 결과는 아닙니다.

WebKit Inspector에서 `performance.getEntriesByName('boot.reveal', 'measure')`는 `duration: 59`인 단일 항목을 반환했습니다. `perf_snapshot.enabled`는 `true`였고 Rust 구간은 아래와 같습니다. 모두 새 PID에서 `count: 1`인 단일 표본이며 중앙값·분포가 아닙니다.

| Rust 구간 | 시간 |
| --- | ---: |
| `setup.main_window` | 83.352292ms |
| `setup.locale_warm` | 0.459417ms |
| `setup.state_restore` | 0.772667ms |
| `setup.deferred_restore` | 2.360209ms |

같은 스냅샷의 `git_status`는 1회 23.703667ms였고 `project_open`·`project_activate`는 0회였습니다. 네 setup 구간의 합계는 약 86.945ms이지만, 이를 `boot.reveal`에 더한 값을 화면 전체 부팅 시간이라고 단정하지 않습니다.

새 PID의 `ps` RSS는 기동 후 약 33초에 946,096KiB였고 앱 상태 표시 RAM은 약 923MB였습니다. 같은 프로세스에서 Inspector 조작을 이어간 뒤 상태 표시 RAM은 138MB로 내려갔습니다. 일시적인 높은 점유와 하락은 관찰했지만 할당 주체·회수 원인은 분리하지 않았습니다. 별도 WebKit Memory 스냅샷과 Monaco 모델·쿼리 캐시 수는 아직 없습니다.

## 판정

합성 프로젝트를 복원한 부팅의 프런트·Rust 지표별 유효 표본 한 번과 화면의 대표 복원 상태를 확보했습니다. 앱은 종료하지 않고 새 PID에서 계속 실행 중입니다. 정확한 메모리 구성·전체 GUI/데이터 복원·활성 원격 자원을 포함한 직접 Exit는 미완료입니다.
