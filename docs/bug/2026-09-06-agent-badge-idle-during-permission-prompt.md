# 에이전트 배지가 권한 프롬프트 중에도 유휴(빈 원)로 남는다 (2026-09-06)

## 증상

내장 터미널의 Claude Code 가 `Do you want to proceed?` 권한 다이얼로그를 띄운 채 기다리는데, 사이드바 프로젝트 아이콘·터미널 탭의
에이전트 배지는 초록 빈 원(`idle`) 이다. 작업 중에도 몇 초 뒤 유휴로 떨어졌다가 돌아오는 등 상태 반영이 늦거나 틀린다.

## 원인

- 판정 신호가 `ps -o state` 의 `R`(CPU 실행 중) 하나뿐이었다. Claude Code 는 API 응답 대기·권한 대기·유휴 모두 `S`(sleep) 라 6초 뒤 전부
  `idle` 이 된다.
- `awaitingInput` 은 hooks 브리지 전담인데 hooks 는 기본 off 의 opt-in 이라 대부분 사용자에게 구조적으로 도달 불가능했다. hooks 를 켜도
  Claude 의 `Notification(permission_prompt)` 은 6초 지연 발화이고 override 가 프로젝트·에이전트명 단위라 세션 구분이 없었다.
- `TERM_PROGRAM=TAIDE` 는 Claude Code 의 알림 채널 자동 선택에서 미지원 터미널로 분류돼 BEL·OSC 9/777 알림이 전혀 나오지 않았다.

정본: `docs/acknowledge/2026-09-06-d54-agent-activity-signals-contract.md` §0 (expect 탐침으로 확인한 타이틀·출력 흐름·다이얼로그 프레임 실물 포함).

## 해결

세션 단위 신호 4종(타이틀 글리프 `◐◑✳`·정규화 텍스트의 다이얼로그 시그니처·실질 출력 흐름·사용자 입력)으로 `classify_session` 이 판정하고,
hooks 는 세션의 PTY 안으로 이벤트를 넣는 인밴드 OSC 777 command hook(`PermissionRequest` 즉시 발화) 으로 바꿨다. 상세와 검증은 계약 §1·§3.
