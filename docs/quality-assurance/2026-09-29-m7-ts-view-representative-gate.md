# M7 TS view 대표 통합 경로 판정

## 범위

M7-C4b의 모집단은 비테스트 `.tsx` 212개입니다. [경로 fixture](../../src-tauri/tests/fixtures/rust-native/ts-view-components-v1.json)와 [정적 inventory](2026-09-29-ts-shared-widget-inventory.md)는 212개 경로의 위치·책임·자동 근거·실기 공백을 연결합니다. 이 문서는 기존 격리 앱 실측만 재사용해 대표 통합 경로를 판정하며 같은 조작을 반복하지 않습니다.

| 대표 경로 | 재사용한 실측 근거 | 판정 |
| --- | --- | --- |
| 메뉴 | [단일 세션](2026-09-29-m7-one-session-perf-gui.md)의 탭 우클릭·네이티브 File/Open Recent·Add Project 메뉴 | 메뉴 열림과 대표 선택 경계 확인 |
| 대화상자·오류 | [단일 세션](2026-09-29-m7-one-session-perf-gui.md)의 Open by Path·존재하지 않는 경로·`Path not found`·Cancel과 [release 부분 실기](2026-09-29-m7-release-gui-smoke.md)의 Open Folder | 대표 성공·실패 진입과 취소 확인 |
| 키보드 | [단일 세션](2026-09-29-m7-one-session-perf-gui.md)의 `⌘⇧P` 팔레트 열기·필터·닫기 | 대표 단축키·입력 확인 |
| 테마·로케일 | [설정 단일 실측](2026-09-29-m7-live-settings-memory-remote.md)의 Dark→Light→Dark, System→English→System | 값·밝은 화면·원복 확인 |
| 다중 OS 창 | [단일 세션](2026-09-29-m7-one-session-perf-gui.md)의 파일 탭 새 창 이동·본창 복귀 | 실제 보조 창과 탭 수명 확인 |
| 시각·접근성 | [release 부분 실기](2026-09-29-m7-release-gui-smoke.md)의 프로젝트·에디터·검색·Git·Settings, [단일 세션](2026-09-29-m7-one-session-perf-gui.md)의 250개 트리·5,000건 검색·Git 20건·터미널 및 [IDE 탭 실측](2026-09-29-m7-ide-ws-authenticated-open-file.md)의 파일 탭을 접근성 트리·실제 화면으로 확인 | 대표 핵심 화면의 시각·AX 값 확인 |

## 한계

212개 경로의 정적 연결은 212개 화면의 실행·시각 동등성·접근성 전수를 뜻하지 않습니다. 대표 경로 밖 설정 17개 section, 입력 22개 feature 및 나머지 개별 상태·오류·키보드 조합은 미실측으로 유지합니다. VoiceOver 낭독·IME·native UI 동등성은 M8 착수/이행 gate이며 여기서 통과로 주장하지 않습니다. M7-C4b의 대표 기준선만 완료하고, 전체 M7·Phase 0 기능 gate와 직접 Exit·원격 인증 실기는 별도 미완료입니다.
