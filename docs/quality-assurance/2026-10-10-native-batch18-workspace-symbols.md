# 배치 18 — 워크스페이스 심볼 검색·팔레트 이동

현재 상태: 배치 17 구현 `38ce4725`·근거 `be85b7b7` 일반 푸시·0/0 확인 뒤 다음 필수 범위의 실제 TS/SDK/팔레트 경계를 조사합니다. 체크리스트 0/7, 기능 대응표 279/588(47.4%)입니다. 전체 출시 전환율/잔여 시간은 미산정이며 main이 직접 직렬 수행합니다.

## 범위와 기준

실제 `use-workspace-symbol-search.ts`·`adapters/workspace-symbol.ts`·`command-palette-workspace-symbol-group.tsx`·팔레트 선택은 프로젝트의 여러 준비된 LSP 세션에서 `workspace/symbol`을 받아 성공 응답을 합치고 해결된 file URI 위치만 표시합니다. 이름/컨테이너·서버 결과 순서를 보존하고 로컬 fuzzy는 이름 강조를 복구하며 결과를 다시 숨기거나 정렬하지 않습니다. # 입력이 비었거나 프로젝트가 없을 때의 안내, 현재 입력 응답만 표시하는 loading과 preview/UTF-16 위치 이동을 확인합니다.

native는 기존 typed SDK와 앱 전용 세션/취소·팔레트 native-host 경계를 재사용합니다. 200ms trailing의 의도와 실제 원본의 두 timer 층을 대조하며 원본 버그/내부 수치를 강제하지 않습니다. 여러 세션의 독립 오류·미지원/빈/해결 안 된 URI, 검색/프로젝트/서버 교체·닫힘·늦은 응답과 루트 guard를 검증합니다. 새 기능/디자인·의존성/정규식 엔진·OS 합성 입력을 추가하지 않으며 browser source/manifest/lock·실제 앱 데이터/clipboard/Keychain/Trash를 보존합니다.

## 체크리스트

- [ ] a. 실제 TS·공식 protocol/로컬 typed API·native 공급/소비 대조
- [ ] b. 응답 모델·좌표/순서·검색/프로젝트/세대·debounce/취소 수명
- [ ] c. 실제 typed 여러 세션 요청·부분 오류/미지원·준비/재시작/취소
- [ ] d. # 팔레트 이름/컨테이너/Hash·강조/안내·키/마우스/IME 선택
- [ ] e. 실제 앱/host preview·기존 탭·UTF-16 reveal·경계/늦은 선택 통합
- [ ] f. 위험 회귀·변경 크레이트 전체 대상·동결/포맷/diff·디스크
- [ ] g. 실제 QA/기능표/PROCESS·선별 커밋·일반 푸시·다음 범위

## 검증 상태

아직 배치 18의 구현/검사를 실행하지 않았습니다. 변경 없는 배치 17 app 전체 67대상·648건과 추가 실패 수정·menu 12건의 최종 서로 다른 649건, batch16 editor 201/UI inspection 358, 이전 syntax/SDK 결과는 해당 경계가 바뀌지 않을 때 재사용합니다. 보호 Trash 3·기존 ignored 5와 실제 OS/접근성·pixel/대형/출시 부채는 미검증입니다.
