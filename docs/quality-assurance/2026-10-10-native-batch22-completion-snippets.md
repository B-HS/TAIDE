# 배치 22 — 자동완성과 사용자 스니펫 소비

현재 상태: 배치 21 구현/완료 근거의 일반 푸시·0/0을 확인했고 현재 기능 대응표는 286/588(48.6%)입니다. 체크리스트 0/7이며 원본/현재 공급을 대조합니다. 전체 출시 전환율/잔여 시간은 미산정입니다. main이 직접 수행하며 서브에이전트/workflow를 사용하지 않습니다. Cargo/fmt는 앞 process의 실제 종료 확인 뒤 하나씩 실행합니다.

## 체크리스트

- [ ] a. TS/Monaco·공식 typed API·후보/자동 trigger/필터/삽입/키/테마·native 공급 대조
- [ ] b. 후보/범위/UTF-16·일반/스니펫 삽입·요청 교체/취소·빈/오류 모델 검증
- [ ] c. 실제 typed LSP·사용자/플러그인 스니펫/원본 단어 공급·프로젝트/본문/peek 수명
- [ ] d. 원본 후보 popup/문서/preview·명령/직렬 입력·선택/수락/취소·snippet session 소비
- [ ] e. 실제 앱·dirty/readonly/두 뷰·언어/서버/종료·입력 소유 회귀와 실기 부채 구분
- [ ] f. 변경 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/완료 근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 범위와 보호 경계

실제 TS completion adapter·사용자 snippet provider/설정·code-editor 옵션과 Monaco suggest의 후보 목록·필터/정렬·선택/수락·취소·문서/preview·키를 기존 native typed LSP/본문/peek/스니펫 session에 연결합니다. 새 기능/디자인·원본 버그/내부 수치 강제 재현·engine/의존성 추가·동결 browser 변경·OS 합성 입력은 하지 않습니다.

기존 TextMate 엔진·실제 앱 데이터/OS 설정/clipboard/Keychain/Trash·보호 app bundle·TS/Tauri/Monaco/xterm을 유지합니다. 테마의 7자리 HEX 결정, 실기 pixel/IME/접근성·대형 성능/soak/보조 창/패키징과 나머지 LSP/AI/출시는 잔여 전체 목표입니다. 변경 없는 batch21의 성공 근거만 재사용하며 변경 크레이트는 전체 대상을 직접 실행합니다. batch21에서 기존 child 초기화 시간 초과가 관찰돼 다음 전체 검증은 테스트 내부 동시 실행을 제한하고 실제 결과를 기록합니다.

## 현재 근거

batch21의 121대상/1412건·최초 실패와 영향 재검사·동결/포맷/경계·디스크 623GiB/66%는 별도 QA에 보존했습니다. 자동완성/스니펫의 실제 본문 소비 완료 근거로 전용하지 않습니다. TS adapter/사용자 snippet provider·code-editor의 suggest preview 설정은 범위 선택 때 읽었으며 상세 Monaco 계약과 현재 native API를 a에서 대조합니다.
