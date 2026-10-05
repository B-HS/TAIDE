# Native LSP 송신 JSON의 사후 byte 검사

## 원인·관찰

`crates/taide-lsp/src/native/session.rs::enqueue`는 완전한 JSON String을 먼저 만들고 나서 frame body와 aggregate outgoing byte 상한을 검사했습니다. 따라서 oversized 요청이 최종 거절돼도 직렬화 사본의 할당이 먼저 발생합니다. batch 앞부분을 queue에 반영한 뒤 뒤 프레임을 거절하기도 했습니다.

small+oversized batch 거절 뒤 queue/counter 무변경을 요구하는 신규 unit은 outgoing_bytes 60/기대값 0으로 실패했습니다(exit 101, 0.00초). 실제 process 전체 RSS 증가량을 계측한 것은 아니며 사후 문자열 할당 경계는 source로 확인했습니다.

## 수정·검증

기존 body/outgoing 한도를 private std::io::Write sink의 각 write와 fallible buffer reservation에 적용합니다. byte overflow·상한을 copy 전에 거절하고 기존 serde_json의 UTF-8/escaping을 유지합니다. batch 전체를 local로 준비해 성공 뒤에만 queue/counter에 반영합니다. 새 dependency·상한 정책·기존 Tauri caller·GUI 후보는 바꾸지 않았습니다.

- 신규 outgoing_payload_budget unit 1건: 원자적 batch 거절·UTF-8/escaping·정확한 상한·기존 queue 보존·추가 write 거절, 0.00초 통과
- 변경 경로를 쓰는 runtime host actual child 검사 1건: initialize·활성 요청·명시적 stop·root 강제 회수, 0.37초 통과
- 제품 LSP lib/tests strict clippy: exit 0, 0.54초

caller Value·command·mirror/URI·metadata·allocator/OS pipe와 전체 RSS는 이 송신 byte 상한에 포함되지 않습니다. actual OOM 주입과 제품 memory/latency gate는 미검증입니다. 세부 소유 표·명령은 `docs/quality-assurance/2026-09-30-m8-lsp-coordinator-spike.md`의 Payload 소유 항목을 따릅니다.
