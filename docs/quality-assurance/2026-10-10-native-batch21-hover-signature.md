# 배치 21 — 일반 호버와 시그니처 도움말

현재 상태: 구현 착수 전 원본/공급 경계 조사입니다. 체크리스트 0/7이며 현재 기능 대응표는 284/588(48.3%)입니다. 전체 출시 전환율/잔여 시간은 미산정입니다. main이 직접 수행하고 Cargo/fmt는 하나씩 실행합니다.

## 체크리스트

- [ ] a. 실제 TS/Monaco·공식 typed API·Markup/지연/trigger/키/테마·현재 native 공급 대조
- [ ] b. 호버·시그니처/Markdown 모델·UTF-16 인자·요청 교체/취소·빈/오류 검증
- [ ] c. 실제 typed LSP·현재 프로젝트/provider·peek mirror·편집/닫힘/재시작 공급
- [ ] d. 원본 호버/서명 표시·명령/직렬 입력·본문/peek 포커스 소비
- [ ] e. 실제 앱·문서/뷰/서버 수명·dirty/readonly/대형·종료 통합
- [ ] f. 변경 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/완료 근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 범위와 보호 경계

일반 문서 호버와 시그니처 도움말을 실제 TS adapter/Monaco 화면·상태·상호작용과 현재 native typed LSP/표면에 연결합니다. 기존 정의 hover와 중복 요청·포커스·입력 소유를 확인하고 순수 peek 공급은 실제 소유/문서 수명으로 연결합니다. 원본 버그/내부 수치 강제 재현·새 기능/디자인·동결 browser 변경·OS 합성 입력을 추가하지 않습니다.

actual 사용자 데이터/OS 설정/clipboard/Keychain/Trash와 보호 app bundle, TS/Tauri/Monaco/xterm을 유지합니다. editor/UI에 regex·구문 engine을 추가하지 않습니다. 테마의 7자리 HEX 결정·실기 pixel/IME/접근성/성능/soak/패키징·전체 workspace/CI 부채와 기타 LSP 기능은 잔여 전체 목표입니다.

## 검증 근거

아직 새 검사나 구현을 실행하지 않았습니다. 변경 없는 경계의 배치 20 성공 근거만 재사용하고 변경한 크레이트는 f에서 전체 대상을 직접 1회 실행합니다.
