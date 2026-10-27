# Rust-native 전환 자율 진행 지시 (2026-10-26)

## 사용자 지시

"워크플로우도 잘 활용해서 태스크가 없어질때까지 끝까지 진행해, 내 허락이 필요한건 보류해두고 모든 작업을 다 진행해. 또한 결정사항은 큰 변경사항아니면 다 권장사항대로 하면 될거야"

## 적용

- workflow 사용을 이 요청에서 선택한 것으로 봅니다. 모델 배정은 전환 작업의 기존 지시(`~/.claude` memory `workflow-model-assignment`)를 따라 구현·수정 opus·xhigh, 리뷰 sonnet·xhigh로 둡니다.
- 남은 배치 계획(`docs/quality-assurance/2026-10-10-native-remaining-batch-plan.md`)의 배치 24 잔여부터 30까지 기능을 단계 단위 workflow로 진행합니다. 단계마다 메인이 검증 결과를 확인하고 선별 커밋·일반 푸시합니다.
- 작은 결정은 추천안(원본 TS 동작에 가장 가까운 안)으로 정하고 QA 문서에 근거를 남깁니다.
- 허락이 필요한 것은 보류 목록에 모으고 진행하지 않습니다: 실제 사용자 데이터 경로·keychain 승계(shell-15, 컷오버 정책), 허용되지 않은 새 외부 의존성, 파괴적 작업, 배치 32·33의 기본 앱 전환과 TS·Tauri 제거, 사용자 실기(CJK IME, VoiceOver).

## 단계 구성 (배치 24 잔여)

| 단계 | 기능 ID |
| --- | --- |
| A 프로젝트 rail | shell-33·34·35, shell-36·37·53·38 |
| B 그룹·drag | shell-45·46·47·48, shell-39·49·52 |
| C 탭 | shell-75·78, shell-81·82 |
| D 보조 창 | shell-4·5·6·8, shell-7·61 |
| E 앱 수준 | shell-3·25, shell-9, shell-12·22 |
| F 셸 기타 | shell-21·24·54, shell-59·94·98 |
| G 통합 | shell-90·92·93, shell-91·96 |

보류: shell-15(사용자 허락 필요), shell-89(배치 25·26의 diff·검색 편집기 구현 뒤 처리).

## 단계 E 결정 (2026-10-27 저녁, 사용자 "재개, 전부 추천안대로")

| 결정 | 채택 |
| --- | --- |
| shell-12 장애 격리 | A안: 영역 render를 'snapshot 읽기 + 영역 소유 상태 + intent 출력'으로 바꾼 영역에만 `catch_unwind` 경계를 둡니다. panic 시 그 영역 상태와 그 pass의 intent를 버리고 fallback을 그리며 재시도는 새 상태로 다시 그립니다. Welcome·Activity Bar·상태 표시줄부터 적용하고, 편집기·사이드바 패널은 공유 상태 변경을 render 밖 트랜잭션으로 옮긴 뒤 적용합니다 |
| shell-3 모니터 목록 | A안: 이미 lock에 있는 `winit =0.30.13`을 앱의 일반 의존으로 올리고 `eframe::create_native`로 이벤트 루프를 만들어 모니터 위치·크기·배율을 읽습니다 |
| Edit 메뉴 단축키 표시 | A안: 메뉴에 단축키를 달지 않고 기존 egui 키 입력 경로를 유지합니다. 메뉴 클릭만 포커스 입력에 전달합니다 |
