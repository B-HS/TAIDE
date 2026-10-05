# M8 native workbench 구현

## 대상·기준

대상은 `native/taide-native-ui/{Cargo.toml,src/*,tests/*}`입니다. 기존 shell 실험의 합성 파일/탭 모델을 재사용한 것이 아니라 실제 `taide-model` DTO와 `taide-runtime` mutation을 사용하는 새 native 화면 구현입니다. GUI 후보의 최종 채택과 native executable 완성을 의미하지 않습니다.

화면 기준은 다음 실제 TypeScript 코드입니다.

- `src/widgets/app-shell/{app-shell,project-shell,shell-slot-tree-view}.tsx`
- `src/widgets/editor-area/{pane-node-view,pane-tab-bar}.tsx`
- `src/features/tab/tab-item.tsx`, `src/features/window/{title-bar,status-bar}.tsx`
- `src/widgets/app-sidebar/app-sidebar.tsx`, `src/features/shell-slot/shell-slot-header.tsx`
- `src/features/welcome/welcome-screen.tsx`, `src/shared/constants/{layout,window-chrome}.ts`

egui 0.36.2의 설치된 공식 source에서 Panel·UiBuilder·Id·widget·pointer drag API를 확인했습니다. eframe host의 ui/logic/repaint 경계는 [공식 App 문서](https://docs.rs/eframe/0.36.2/eframe/trait.App.html)를 확인했습니다. Tokio watch의 `borrow_and_update`와 Ref의 변경 여부는 설치된 공식 source를 확인했습니다.

## 구현 경계

- snapshot은 기존 AppState의 mutation guard에서 session/project/layout/settings를 읽습니다. native UI가 새로운 canonical project/tab 상태를 소유하지 않습니다.
- main의 slot tree, focused project의 제목·상태 영역, Zen의 focused slot 표시와 sidebar/status 숨김을 구분합니다. auxiliary는 지정 project/window slot의 pane tree만 표시하며 main Zen·rail·status를 적용하지 않습니다.
- title 28px, status 24px, project rail 56px, slot header 24px, tab 36px, explorer 기본 240px·최소 180px·최대 40%, pane 최소 120px와 traffic-light inset 78px은 TS 기준값에서 가져왔습니다. 실제 픽셀 동등성 검증은 아닙니다.
- project/group rail, tab 활성화·preview 유지·pin 해제·dirty dot, slot/pane split과 pointer/keyboard resize의 입력을 typed intent로 전달합니다. dirty 탭 닫기는 `RequestCloseTab`이며 이 계층에서 직접 폐기하지 않습니다. pinned middle close는 거절합니다.
- 기존 runtime의 project/slot/group/chrome/sidebar/tab/pane mutation을 재사용합니다. native controller는 64개 command count 상한과 기존 TaskSupervisor의 소유 작업자를 사용합니다. queue 포화·종료를 호출자에게 반환하고 외부 EventSink 변경 후 snapshot을 갱신합니다. repaint/command 동안 파일 처리나 canonical state 변경을 GUI frame에서 직접 실행하지 않습니다.
- 오류 watch는 후속 성공 명령으로 아직 읽지 않은 오류를 없애지 않으며, 읽은 오류는 같은 값으로 반복 반환하지 않습니다.
- explorer/editor/terminal/status와 번역·branch 조회는 명시적 `ShellSurfaces` 조립 경계입니다. 실제 surface 구현은 아직 연결하지 않았습니다. 검사에서만 합성 surface를 사용했습니다.
- 격리 package의 MSRV는 후보에 맞춘 1.95입니다. 제품 workspace MSRV 1.89·기존 앱 manifest/lock은 변경하지 않았습니다. 독립 lock은 root의 기존 package 선택을 기반으로 생성했고 egui graph 및 기존 Tokio edge만 사용합니다.

## 실행 결과

- [x] `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test workbench --locked --offline --target-dir experiments/terminal-core-spike/target`: 3 passed, 0 failed, 0.05초. 두 project/Zen/auxiliary 표시 범위, split 좌표/최소 크기, 실제 runtime revision/event/sidebar와 pinned/dirty close intent를 확인했습니다.
- [x] 표시 구성·focus/thickness 변경 뒤 `--test workbench 실제_레이아웃_렌더`만 재실행: 1 passed, 0 failed, 0.07초. 나머지 unchanged 검사 성공은 재사용했습니다.
- [x] `--test controller`: 실제 runtime 명령, 외부 이벤트의 snapshot 갱신, 오류 전달, sender 종료 뒤 owned worker join와 추적 0을 확인했습니다. 최초 1건 0.00초, 후속 오류 보존 변경 뒤 해당 1건 0.00초입니다.
- [x] 최신 `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --all-targets --locked --offline --target-dir experiments/terminal-core-spike/target -- -D warnings`: exit 0, 0.43초.

초기 manifest의 존재하지 않는 egui feature를 제거했고, egui 0.36.2의 Id/UiBuilder Debug 경계와 정적 검사에서 발견한 함수 인자 구성을 수정했습니다. 성공 이전 compile 실패를 성공으로 세지 않았습니다. headless output의 texture delta는 renderer가 없는 검사에서만 명시적으로 clear합니다. OS 앱 실행·화면 조작·실기 bundle 교체는 하지 않았습니다.

## 남은 항목

- [ ] 실제 native executable의 AppServices/종료·영속화·OS 창 수명 연결, 프로젝트 열기/복원과 watcher/capability 조립
- [ ] 실제 explorer/editor/terminal/status, theme/locale·최근 프로젝트·welcome shortcut, dirty 확인/dialog 연결
- [ ] 전체 keymap·tab cycling·context menu·DnD·split drop·group/project 메뉴·아이콘/색·focus ring과 포인터 focus 정책의 TS 동등성
- [ ] 최신 canonical resize와 임시 drag의 충돌/취소, snapshot 전체 복제 비용·command byte budget·root shutdown 중 admission 등 제품 통합 위험
- [ ] 213개 view 대응, 실제 픽셀·CJK IME·VoiceOver·GPU 복구·software fallback·성능·배포 gate

이 문서의 통과 범위는 새 shell renderer/controller의 코드 경계입니다. N2 전체·213개 화면·제품 GUI·M8 완료로 확장하지 않습니다.
