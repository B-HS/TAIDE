# native 프로젝트 hooks-enabled 무조건 거절

## 대상

`native/taide-native-app/src/projects.rs`의 open/restore_watchers/check_hooks와 `projects-tests.rs`입니다.

## 증상과 원인

agent_hooks_enabled=true이면 프로젝트를 열거나 watcher를 복원하기 전에 `Forbidden("native agent hook reconciliation is not connected")`을 반환했습니다. 원본 Tauri capability는 enabled 자체로 프로젝트를 거절하지 않고 IDE lockfile build 뒤 감독된 installed-hooks reconcile을 실행합니다. native의 미조립 guard가 실제 지원을 대신하고 있었습니다.

## 재현과 해결

필수 typed callback을 주입한 최소 합성 enabled open exact 검사가 실제 Forbidden으로 RED(17.69초/0.01초)였습니다. 같은 services·감독자의 agent-hooks-attach에 실제 native reconcile을 연결하고 blanket gate를 제거했습니다. callback은 guard-free build에서 spawn하고 기다리지 않으며 root Settings/installed-file gate를 유지합니다. production new는 실제 callback이고 fixture는 합성 callback이라 사용자 home/hooks를 조작하지 않습니다.

변경 뒤 신규2·관련 remote project3 PASS(8.05초/0.49초), native strict13.72초·authored2fmt PASS입니다. callback pending 비대기/동일 services·mutation guard·중복 open/restore·shutdown/task0을 확인했습니다. 전체 capability/App/실제 enabled 사용자 홈 통합은 미완료이며 상세는 `docs/quality-assurance/2026-10-04-m8-native-project-hooks.md`입니다.
