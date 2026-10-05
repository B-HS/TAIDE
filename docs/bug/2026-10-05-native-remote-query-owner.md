# 원격 PTY query 응답 중복

## 대상과 증상

대상은 `native/taide-native-app/src/terminal_dispatch.rs`, `terminal_host.rs`, `remote-terminal.rs`입니다. 원본 Tauri PTY는 raw 출력과 metadata/agent 관찰만 제공하며 xterm renderer가 자신의 화면으로 query를 처리합니다. native 원격 spawn은 네이티브 actor까지 응답해 같은 query에 두 응답이 생겼습니다.

## 재현과 원인

실제 합성 `/bin/cat` PTY에서 DSR5n 뒤 OSC title 관찰 완료를 기다리고, raw 채널의 query를 받은 client의 응답을 전송했습니다. server 자동 응답과 client 응답을 합쳐 기대1/관찰2로 실패했습니다. kill·Hub idle·task0 뒤 개수를 판정했습니다. subscriber 유무가 아닌 `exit_code.is_none()`만 보던 기존 can_reply는 원격 renderer 소유권을 판별하지 못합니다.

초기 fixture는 Title을 TerminalEvent로 기다려 timeout이었지만 실제 Core는 ScanEvent::Title로 전달합니다. stream observer로 정정한 뒤 위 제품 RED를 확인했습니다. 구현 중 move closure가 SessionPorts 전체를 가져가던 E0382는 CommandColors를 closure 밖에서 복사해 수정했으며 의존성이나 Clone 우회는 추가하지 않았습니다.

## 해결

Native EffectPorts와 renderer 관찰 전용 ObservePorts를 타입으로 분리했습니다. 후자에는 색상·pixel geometry 공급자가 없습니다. 원격 spawn은 동일 Hub의 관찰 전용 actor를 사용해 상태·색상·화면 크기 응답을 생성하지 않습니다. Core/parser/raw 출력·replay·metadata·command clock·agent·Bell/stream·repaint와 revision/실패 gate는 유지합니다. native spawn은 실제 화면 query 포트와 종료 후 응답 억제를 유지합니다.

## 검증과 한계

`remote_terminal::tests::원격_query는_renderer_응답만_전송하고_서버에서_중복생성하지_않는다`는 compile12.04초/suite0.03초·1 PASS입니다. 상세 영향 검사는 remote-terminal QA의 2026-10-05 절이 정본이며 같은 성공은 반복하지 않습니다.

원격 spawn의 query 생성 방지만 확인했습니다. native-origin 세션을 원격 renderer가 구독하거나 remote-origin 세션을 native View가 이어받는 전체 응답 소유권 handoff는 아직 연결되지 않았습니다. 실제 Rust web renderer·생산용 assets·App caller/GUI/start/Exit·N1~N8 완료 근거로 확대하지 않습니다. [Rust enum 계약](https://doc.rust-lang.org/reference/types/enum.html)을 확인했고 기존 Rust 타입 패턴을 유지했습니다.
