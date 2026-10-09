# 배치 14 — 전체 기능 대응표 재감사

기준: 2026-10-09, `to_rust_native`, HEAD `80669ec3`. 배치 13까지 구현·검증·일반 푸시를 마쳤으며 서브에이전트·workflow 없이 메인이 직접 재감사합니다. 현재 Source와 실제 앱 도달 경로·동작 검사 근거를 연결하기 전에는 이전 done/partial 판정을 승계하지 않습니다.

## 목록과 완료 기준

기존 8개 화면 감사의 상세 기능 표를 직접 추출한 결과는 599행입니다. 2026-10-06 요약의 600행과 한 행 다릅니다. 탐색기/검색 상세에는 탐색기 30·검색 8·아웃라인 2·문제 5로 45행이 있고 당시 요약은 46행으로 집계했습니다. 누락된 기능을 임의로 만들거나 과거 212/573 비율을 사용하지 않습니다.

| 상세 감사       | 실제 표 행 |
| --------------- | ---------: |
| shell           |         99 |
| editor          |         87 |
| terminal-agent  |         78 |
| explorer-search |         45 |
| preview         |         40 |
| git             |         65 |
| settings        |         83 |
| commands-ui     |        102 |
| 합계            |        599 |

이는 한 행에 여러 하위 동작이 묶인 요구사항 목록이며 원자적인 기능 수가 아닙니다. 같은 구현을 여러 표에서 참조하는 경우 해당 화면/명령 진입 경로를 교차 연결해 중복과 실제 별도 요구사항을 판정합니다. 웹 기술 전용·동결 범위·실기/출시 게이트는 명시적으로 구분하고 최종 기능 분모는 전체 재판정 후 집계합니다. IPC 206종·구조 49항목·TS 파일 수와 테스트 수를 화면 기능 분모에 더하지 않습니다.

`git diff --name-only 2824005 HEAD -- src`는 출력 없음·exit 0입니다. TS 소스는 최초 감사 기준 이후 변경되지 않았으므로 기존 TS 경로 지도를 재사용하되 현재 파일의 주요 진입/설정/상태 경계를 직접 대조합니다. native 판정은 이 사실만으로 승계하지 않습니다. 현재 `code-editor.tsx`의 실제 옵션과 `editor-pane.tsx`의 저장/뷰/IDE/터미널 액션, `explorer-panel.tsx`의 slot focus와 네 뷰 bridge, `bootstrap-commands.ts`의 명령 등록을 확인했습니다.

완료는 해당 행의 TS 사용자 동작이 native에 구현돼 있고 실제 앱에서 도달하며 해당 동작 묶음의 자동 검사 근거가 연결된 경우입니다. 모든 원본 분기·pixel·OS 입력을 검사했다는 뜻은 아닙니다. 일부 하위 동작/검증이 남으면 partial, 모델/서비스/계약이 있으나 앱에서 도달하지 않으면 unwired, 사용자 구현이 없으면 missing입니다. 실제 화면/OS 입력·접근성·대형/soak·보안/패키징/출시 완료는 별도 게이트이며 자동 기능 완료로 대신하지 않습니다.

## 현재 확인과 후속

실제 `application.rs::AppSurfaces::tab_content`는 Settings/AppFile/Terminal/Untitled/File을 연결하며 Diff/ClaudeDiff/SearchEditor 등은 현재 로컬라이즈된 unavailable 표면입니다. `lsp.rs`의 앱 typed 요청은 저장 Formatting/ExecuteCommand/CodeAction/Resolve 경로이고 완성·hover·signature·심볼 요청은 앱 경로에 없습니다. 카탈로그/프로토콜 계약만 있는 기능을 완료로 세지 않습니다. 배치 7의 편집 명령·배치 8~13의 찾기/표시 연결은 현재 Source·최신 전체 검증 근거로 따로 연결합니다.

## 현재 집계

[599행 현재 대응표](2026-10-09-native-function-matrix.md)와 [판정/근거 스냅샷](2026-10-09-native-function-audit.json)을 작성했습니다. 과거 native 상태를 복사하지 않고 현재 코드의 실제 소비/요청 경로와 관련 검사 묶음을 32개 근거 그룹·존재 확인한 181개 경로에 연결했습니다. 같은 구현을 참조해도 화면/명령 진입과 소비자가 다르면 해당 요구사항을 유지합니다. 복합 요구사항·교차 참조를 포함한 행의 완료율이며 원자적인 기능 수가 아닙니다.

| 범위            |  행 | 완료 | 부분 | 미연결 | 미구현 | 웹 기술/내부 지표 | 동결 |
| --------------- | --: | ---: | ---: | -----: | -----: | ----------------: | ---: |
| shell           |  99 |   44 |   21 |      9 |     22 |                 3 |    0 |
| editor          |  87 |   41 |   12 |      3 |     29 |                 2 |    0 |
| terminal-agent  |  78 |   44 |   12 |     13 |      7 |                 2 |    0 |
| explorer-search |  45 |   20 |    6 |      4 |     15 |                 0 |    0 |
| preview         |  40 |   32 |    5 |      0 |      1 |                 1 |    1 |
| git             |  65 |    1 |    1 |     34 |     29 |                 0 |    0 |
| settings        |  83 |   43 |   10 |     28 |      1 |                 1 |    0 |
| commands-ui     | 102 |   47 |   26 |     24 |      4 |                 1 |    0 |
| 합계            | 599 |  272 |   93 |    115 |    108 |                10 |    1 |

웹 기술/내부 지표 전용 10행과 사용자 동결 Wasm 프리뷰 1행을 제외한 기능 분모는 588행, 자동 검사 근거까지 연결한 완료는 272행(46.3%)입니다. 부분 항목에 임의 점수를 주지 않았으며 미완료는 316행입니다. 배포까지 포함한 전체 전환 완료율이 아니고 테스트 수/배치 수를 환산한 값도 아닙니다. 현재 스냅샷의 기능 비율로 계속 표시하며 이후 실제 구현/검증/앱 연결이 바뀐 행을 갱신합니다.

집계 초안의 47.1%는 확정 전 수치였습니다. 실제 `command-score.rs`의 font picker 소비자는 존재하므로 웹 기술로 제외하지 않고 부분으로 판정했고 terminal clipboard 가용성은 붙여넣기 gate 요구사항으로 포함했습니다. 글꼴 picker 전체 UI·공용 스타일/상태·메뉴 paste gate의 부족한 검증/동작도 부분으로 정정했습니다. 최종 599행은 ID 중복/누락 0이며 최초 TS 요구사항의 feature/ts/section/source와 모두 일치합니다.

편집기/LSP, 패널·터미널/프리뷰, 셸/설정/명령 전체를 분리해 재판정했습니다. 핵심 잔여 소비자는 LSP 완성·hover·signature·이동·peek/rename/action/심볼/semantic/inlay, 진단·overview/minimap·SCM 공급, 탐색기 4뷰·검색/SearchEditor·Git/그래프/diff/history/blame/충돌, 탭/프로젝트 메뉴·drag/drop·보조 창, 설정 LSP/AI/Plugin/Sync/Remote와 TaskRunner/IDE save/diff/선택입니다. 서버/원격·순수 엔진만 있는 항목을 로컬 화면 완료로 세지 않았습니다.

## 구조·실기·출시 게이트

기존 architecture 49항목/IPC 계약 206종을 화면 기능 분모에 더하지 않습니다. 이번 점검은 구조의 현재 핵심 경계를 직접 읽었으며 과거 구조/IPC 수치를 현재 통과율로 승계하지 않습니다.

| 게이트                            | 현재 코드/증거                                                                                                                                                                                                                                                                 | 판정                                                                                  |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| workspace/CI/vendor/크레이트 경계 | native app/UI/terminal은 독립 workspace·MSRV 1.95/edition 2024이며 app eframe/vte와 mock-server가 experiments 경로에 의존합니다. CI/release는 root/Tauri 경로입니다. HTTP/IDE/hooks·프리뷰 파서가 app에 있고 source include/자원 역참조·큰 앱/터미널 소유 객체 정리가 남습니다 | 미완료. 동결 remote-web graph/lock은 유지해야 하며 구조 정리도 전체 목표의 일부입니다 |
| 실제 제품 실행/데이터·CLI         | `LaunchConfig`는 격리 절대 --data-dir를 요구하고 secret namespace는 native-isolated입니다. 실제 사용자 데이터/Keychain·CLI/단일 인스턴스·PATH/창 복원/롤백·로깅 초기화·제품 번들은 완성되지 않았습니다                                                                         | 미완료. 보호 실제 데이터/Keychain에 접근하지 않았습니다                               |
| 다중 창/OS 메뉴/drop/GPU          | 실제 앱은 Main만 생성합니다. deferred viewport 검색은 tooltip 테스트에만 있고 앱 생성 소비자는 없습니다. OS 메뉴·외부 dropped/hovered file·GPU fallback/로깅 초기화 검색에 실제 제품 경로가 없습니다                                                                           | 미완료. 논리 auxiliary 모델/테스트로 실기 통과를 주장하지 않습니다                    |
| 실기/품질                         | CJK IME·RTL/컬러 문자·접근성/VoiceOver·실제 화면 전체 상태·대형/soak/VT matrix·메모리/대기 시간·보안·지원 OS 게이트의 전체 실행 증거가 없습니다                                                                                                                                | 미검증/미완료. 메모리 egui RawInput 자동 검사는 실제 OS 합성 입력이 아닙니다          |
| 패키징/출시·TS 제거               | 보호 spike bundle은 제품 패키징 근거가 아니며 Developer ID/공증·설치/업데이트·beta/데이터 롤백·전체 cutover 게이트가 남습니다. 기존 TS/Tauri/Monaco/xterm은 유지합니다                                                                                                         | 미완료. 전체 게이트 이후만 제거합니다                                                 |

egui/eframe 0.36.2와 ferriki-textmate 0.12.0 + ferroni 1.8.1은 사용자 확정 방향입니다. 과거 문서의 임시 egui 채택/구문 엔진 부재를 현재 차단으로 승계하지 않습니다. HTML/media의 제한된 Rust 소유 helper/WebView 경계와 frozen Wasm은 그대로 보존합니다. 원본 버그/내부 수치 재현을 새 완료 조건으로 만들지 않습니다.

예상 잔여 시간은 미산정입니다. 미완료 복합 행 316개는 난이도가 같지 않으며 LSP/SCM/다중 창/구조·실기/출시는 선행 의존과 외부 조건도 다릅니다. 자동 테스트의 초 단위 실행 시간을 전체 개발 기간으로 환산할 근거가 없습니다. 다음 필수 구현과 실제 수행/검증 시간 근거를 누적해 범위를 좁힙니다.

## 검증과 다음 작업

문서/읽기 전용 점검으로 native 제품 코드·의존성·lockfile은 바뀌지 않습니다. 성공한 배치 13 editor 192·syntax 159·UI 337·app 612, 전체 124대상·1300건과 변경 없는 SDK 배치 12 단위 52/문서 167의 증거를 재사용하며 Cargo를 반복하지 않습니다. 보호 Trash 3건·ignored editor/syntax 4건/SDK 문서 1건·실기 부채는 미검증입니다.

- [x] 원본 TS 599행·현재 ID·feature/ts/section/source 일치, 중복/누락 0
- [x] 모든 분류 1개·근거 그룹 32개·경로 181개 존재 확인, 현재 기능 분모/분자 재집계
- [x] utility/스냅샷/표 포맷·문서 diff·동결 경계·디스크 확인 — 아래 실제 실행 결과 참조
- [x] 선별 커밋·일반 푸시 및 로컬/원격 차이 확인 — `700b6084`, 일반 push 성공·차이 0/0
- [x] 후속 필수 구현 체크리스트를 작성해 전체 전환 계속 진행 — PROCESS 상단 배치 15 진단 공급/표시·문제 이동·overview 범위

후속은 편집기에서 이미 수신하는 LSP 진단을 문서/revision/좌표를 확인하는 표시 공급자로 연결하고 진단 밑줄·문제 이동·overview ruler를 구현하는 범위로 정합니다. 이 기반 뒤에 LSP hover/완성·심볼/메뉴·SCM 등 미완료 행을 직렬로 계속 구현합니다. 배치 종료는 전체 목표의 중단이 아닙니다.

검증 실행 결과: `bun docs/utils/2026-10-09-native-function-audit.js`는 최초 TS 원본·스냅샷·렌더 표 599행의 모든 필드/분류/고유 ID·181개 실제 경로와 현재 272/588(46.3%) 집계를 확인해 exit 0입니다. formatter 이전/이후 모두 일치합니다. 첫 formatter 이후에는 표 정렬 padding을 고정한 행 정규식 때문에 행 수 검사가 실패했고, 이를 cell/ID 파싱으로 수정한 뒤 원본 TS 참조 9행의 단일 tilde 범위를 GFM 취소선으로 바꾼 포맷 문제가 드러났습니다. 코드 밖 tilde를 escape하고 decode 경계를 함께 적용한 뒤 formatter 이후 전체 599행이 다시 일치했습니다. 판정·분모/분자 데이터는 이 포맷 정정으로 바뀌지 않았습니다.

Prettier 대상 JS/JSON/표/QA/완료 근거 5파일은 최종 write와 check에서 exit 0입니다. `git diff --check -- docs/PROCESS.md docs/quality-assurance/2026-10-09-native-completion-evidence.md`는 exit 0이고 `git diff --name-only 80669ec3 -- native crates src src-tauri Cargo.toml Cargo.lock`는 출력 없음·exit 0입니다. 새 파일의 whitespace는 선별 staged diff에서 확인합니다. 디스크는 660GiB 여유·64% 사용입니다. 빌드 정리를 하지 않았으며 실행 중인 Cargo/도구 세션은 없습니다. 기존 `docs/architecture.md`의 말미 공백 및 HANDOFF/합의·운영·PROCESS 하단의 이전 변경은 이번 선별 Git 범위에서 제외합니다.
