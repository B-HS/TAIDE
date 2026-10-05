# M8 코드 기준 화면 재현 우선순위

## 사용자 지시

2026-09-30 사용자는 기존 TypeScript view를 최대한 동일하게 재현하는 구현을 먼저 진행하고, 앞서 대기하던 보완 검토와 실기 검증은 최후 순위로 옮기라고 지시했습니다.

## 적용

- N2~N5 native 구현을 N1의 CPU/GPU 방향 답변이나 사용자 담당 IME·VoiceOver 결과 대기로 막지 않습니다.
- 기존 TypeScript 소스를 화면 구조·상태·상호작용의 기준으로 사용합니다. 새 기능 추가나 디자인 변경은 목적이 아닙니다.
- 기존 egui 0.36.2 후보를 격리 native 구현에 임시 사용합니다. 이는 최종 제품 dependency 채택·root MSRV 변경 승인이 아닙니다.
- 최종 hard gate, 213개 view 동등성, 데이터 호환·성능·보안·배포 검증은 면제하지 않습니다. TypeScript 제거와 M8 완료 commit·push는 전체 완료 뒤에만 수행합니다.
- 사용자 실기 앱과 시스템 입력기·VoiceOver 설정을 유지합니다.

앞선 transition contract/parity plan의 실험 선행 순서와 충돌하는 부분에는 이 최신 구현 순서 지시를 적용하되, 최종 채택·cutover 조건은 유지합니다. 실제 진척은 `docs/PROCESS.md`가 정본입니다.
