# Wave H 구현 계약 — 키맵 엔진 (chord·when) (2026-08-16)

> 정찰 wf_21fd174b-f0d(opus+high 4축: 현행 키맵 전수·chord 엔진·when 컨텍스트·shift+기호 캡처).
> 하중 주장 전건 메인 실물 재검증 완료. **갭 분석 §7 의 "shift+기호 부분 지원" 은 stale 로 판명** —
> normalizeKeymapEventKey(기능확장 2차 k)가 이미 기호를 event.code 로 유도한다(CLEAN 패턴 미통과 →
> code 경로, 실측 확인). 캠페인 계약: `2026-08-14-remaining-features-pro-qa-plan.md`.
> **선행 결정 존중**: "event.code 무조건 우선"은 기능확장 2차 k 에서 기각 — 재론하지 않는다.

## 1. 사용자 결정 (2026-08-16)

| # | 결정 | 선택 |
|---|------|------|
| ① Wave G prod | **병합** | `c3e60c4` 까지 main fast-forward(A~F 와 동일 기준) |
| ② Sparkles 정책 | **unstaged 폴백** | Wave G 검토 보류분 해소 — staged 0건이면 워킹트리 전체 변경 diff 로 폴백(VS Code 동형). Wave H 구현에 동승 |
| ③ H 범위 | **재정의 + 결함 상환** | 본체=chord+when 엔진. shift+기호는 회귀 테스트+문서 정정으로 축소. 기존 결함 전부 상환: 깨진 monaco chord 4건·terminal-pane 오버라이드 미적용·커스텀 taide.* 액션 카탈로그 편입 |
| ④ chord·when | **추천 패키지 + 마이그레이션 보수** | chord ⌘K 프리픽스·에디터 포커스 시 monaco 위임·2단 무조건 삼킴·5000ms 미러·인디케이터·전역 스토어·chord? 선택 필드 / when ContextKeyExpr 딥임포트+pull 게터·내부 전용 / **마이그레이션은 전략 1**(terminal-jump 만 when 명문화, !modalVisible 등 행동 변화 유발 분기는 보류 — 기존 21 엔트리 행동 변화 0) |

## 2. 확정 사실 (메인 재검증 완료 — 근거 실물)

1. **shift+기호는 이미 event.code 기반** — CLEAN_SINGLE_KEY_PATTERN 이 기호(%·?·+ 등)를 통과시키지
   않아 code 유도 경로(keymap.ts:174-208)를 탄다. 갭 §7 은 normalizeKeymapEventKey 도입 커밋(23c3fb2)
   이전 기술. 이 축은 회귀 테스트+문서 정정 규모.
2. **window capture 가 monaco 보다 항상 선행** — 앱 캡처는 window capture 단계(use-keydown-capture.ts
   capture:true), monaco 는 에디터 domNode 리스너(standaloneServices.js:258-269). 앱이 ⌘K 를 삼키면
   monaco chord 21건(전부 ⌘K 1단) 전멸.
3. **기존 회귀 4건 구조 확정** — APP_KEYMAP 의 ⌘B(toggle-sidebar)·⌘F(find)·⌘=(font-size-up)·
   ⌘-(font-size-down)가 monaco chord(⌘K ⌘B/F/=/-)의 **2단 키**를 window capture 에서 삼킨다
   (keymap.ts:50-51,65-66 실측).
4. **monaco chord 는 5000ms 타임아웃+500ms 폴링**(abstractKeybindingService.js:83-87), standalone 의
   chord 상태 표시는 no-op 스텁(standaloneServices.js:223-225) — 인디케이터는 필수 신설.
5. **KeymapEntry.when 은 dead field**(keymap.ts:32 선언만, 데이터 0·평가기 0). 유일한 when 유사물은
   terminal-pane 의 isFocused 핸들러 undefined 게이팅(terminal-pane.tsx:74-77) — 이 호출은 entries
   미전달로 **사용자 오버라이드를 무시**하는 기존 결함.
6. **오버라이드 직렬화** — KeymapOverrideEntry{actionId,key,mods} JSON 문자열, Rust 는 불투명
   Option<String>(마이그레이션 무관). isKeymapOverrideEntry 는 얕은 가드라 chord 선택 필드가
   구버전 파서를 조용히 통과(전방호환). unbind 는 key:'' 센티널.
7. **APP_KEYMAP 21 엔트리·window capture 소비처 5곳 독립 리스너**(app-shell·editor-area·
   command-palette·terminal-pane·status-bar-content) — chord 대기 상태는 전역 단일이어야 함.
8. **monaco ContextKeyExpr 는 번들에 실존**(esm/vs/platform/contextkey/common/contextkey.js —
   deserialize/evaluate). 딥임포트 선례·절차: `2026-08-14-monaco-command-service-deep-import.md`.
9. 커스텀 taide.* monaco 액션(aiInlineEdit·toggleBlame·openFileHistory·saveFile·toggleMinimap·
   gitStageSelection·runSelectedTextInTerminal 등)은 MONACO_ACTIONS 정적 카탈로그 밖 — 재바인딩·
   충돌 탐지 대상이 아님.

## 3. 확정 설계

### 3.1 chord 엔진 (프론트)

- **전역 chord 스토어**(shared/lib — keymap-capture 선례 형태): 대기 상태(1단 캡처 시각·프리픽스)
  단일 소스. 5곳 리스너가 공유.
- **1단 진입**: 활성 엔트리 중 chord 를 가진 것의 1단(key+mods)과 매칭 && **when 게이트 통과**
  (`!editorTextFocus` — 에디터 포커스 시 monaco 에 위임해 21건 보존, 결정 ④) 시
  preventDefault+stopPropagation 후 대기 진입 + **상태바 chord 인디케이터** 표시("(⌘K) 다음 키 대기").
- **2단**: 대기 중 다음 keydown 은 **무조건 삼킨다**(매칭 실패 포함 — 오입력 방지, 불일치 시
  인디케이터에 무매칭 피드백 후 해제. 수식어 단독 keydown 은 제외). 매칭 시 핸들러 실행.
- **이탈**: 5000ms 타임아웃(상수 — monaco 미러)·window blur/문서 포커스 상실 시 즉시 해제.
- **monaco chord 유예 창(기존 회귀 4건 상환)**: 에디터 포커스 상태에서 ⌘K keydown 을 **관찰**
  (삼키지 않음)하면 "monaco chord 대기" 플래그를 세우고, 다음 keydown 1회(또는 5000ms/포커스 상실)
  동안 **앱 캡처 전체(단일 바인딩 포함)를 양보** — ⌘K ⌘B/F/=/- 가 monaco 에 도달한다.
- **직렬화**: KeymapEntry·KeymapOverrideEntry 에 `chord?: { key, mods }` 선택 필드(1단은 기존
  key/mods — 구버전 파서 100% 통과·무음 소실 없음). 표시는 formatKeymapShortcut 확장(두 조합 공백
  join — monaco defaultBindingLabel 과 동일 표기). findKeybindingRowById·팔레트 표시 동반 갱신.
- **캡처 UI**(keybinding-row·keybindings-editor): 1타 후 "대기" 배지 → 2타로 chord 확정, 1단만
  원하면 명시 확정(Enter/버튼). onBlur 취소 정리. APP_KEYMAP 표본 chord 1건 신설:
  **⌘K ⌘S = 키바인딩 에디터 열기**(requestOpenKeybindingsEditor 재사용 — VS Code 관성. monaco
  기본 chord 21건 목록에 ⌘K ⌘S 없음 확인. 단 에디터 포커스 시에는 위임 정책상 미발동 — 문서 명시).

### 3.2 when 컨텍스트 (프론트)

- **평가기 = monaco ContextKeyExpr 딥임포트**(파싱·평가·serialize — ambient shim + 이 계약이
  acknowledge 기록. 업그레이드 점검 절차는 딥임포트 선례 문서 준수). **bun:test 로드 검증 게이트**:
  구현 첫 단계에서 bun 테스트로 로드 확인, 실패 시 배열 AND/NOT 자체 평가기(a')로 전환하고 보고.
- **컨텍스트 소스 = pull 게터 맵**(keydown 시점 동기 평가): 1차 화이트리스트 `editorTextFocus`
  (activeElement 가 monaco 에디터 컨테이너 내부)·`terminalFocus`(xterm 컨테이너 내부). 확장은 후속.
- **노출 범위 = 내부 전용**: APP_KEYMAP 엔트리에만 when 사용. KeymapOverrideEntry 에는 미노출(보류 §4).
- **마이그레이션 = 보수(전략 1)**: terminal-jump 2건만 `when: 'terminalFocus'` 로 데이터화(핸들러
  undefined 게이팅 제거). 나머지 19 엔트리는 when 없음 유지 — **기존 행동 변화 0**.
- **충돌 판정 when-aware**(같은 커밋): findKeymapConflict — key+mods 동일해도 양쪽 when 이 모두
  존재하고 다르면 비충돌. 키맵 에디터에 **컨텍스트 인스펙터**(현재 활성 컨텍스트 키 목록 표시 —
  when 디버깅 수단) 추가.

### 3.3 기존 결함 상환

- **terminal-pane 오버라이드 비대칭**: useGlobalKeymap 을 훅 내부에서 오버라이드 구독(설정 쿼리
  캐시)하도록 반전 — 5곳 소비처의 중복 파싱 제거·entries 파라미터 폐지. terminal-jump 재바인딩이
  실제 반영되게.
- **monaco chord 4건** — §3.1 유예 창으로 해소(실기 확증은 qa6).
- **커스텀 taide.* 액션 카탈로그 편입**: MONACO_ACTIONS 에 커스텀 액션 섹션 등재(id·라벨·기본
  바인딩 표기 — 구현 시 전수 나열)해 키바인딩 에디터 노출·재바인딩(monaco-keybinding 경로)·충돌
  탐지 대상화. addAction 기본 바인딩과 오버라이드 규칙의 공존 방식은 기존 monaco 행 선례를 따른다.

### 3.4 Sparkles unstaged 폴백 (Wave G 보류 해소)

- Rust `git_diff_staged_text`: staged 델타 0건이면 **HEAD↔워킹트리 diff 로 폴백**(동일 제외·상한
  규칙, 반환에 `usedFallback: bool` 추가 — bindings 재생성). 프론트: 버튼 활성 조건을 "staged 또는
  변경 존재"로, 폴백 사용 시 토스트에 명시(locale 키 추가). staged 0건·변경 0건이면 기존대로 비활성.

### 3.5 shift+기호 축 (축소 확정)

- 신규 구현 없음. normalizeKeymapEventKey 기호 21종 회귀 테스트 보강 + 갭 분석 §7 stale 행 정정
  (구현 완료 표기) + qa6 실기 항목(⌘⇧5 류 캡처·발동).

### 3.6 실행 구조

- **Phase A(Rust 소규모, sonnet+xhigh)**: §3.4 폴백 + locale 신규 키(인디케이터·chord 캡처·무매칭
  피드백·인스펙터·폴백 토스트 — 4곳 동기) + bindings 재생성 + cargo fmt/clippy/test.
- **Phase B(프론트 키맵 코어 단독, sonnet+xhigh)**: keymap.ts(chord 타입·직렬화·매칭·when 필드
  활성화)·전역 chord 스토어·ContextKeyExpr 어댑터(+로드 게이트)·pull 게터·useGlobalKeymap 반전·
  use-keydown-capture chord/유예 창 통합·충돌 판정 when-aware. **코어는 파일 겹침이 커 단독 소유.**
- **Phase C 병렬 2(sonnet+xhigh)**: C1 = 캡처 UI·키바인딩 에디터(chord 캡처·대기 배지·인스펙터)·
  formatKeymapShortcut·표시 소비처 / C2 = 소비처 마이그레이션(5곳 entries 제거·terminal-jump when)·
  커스텀 액션 카탈로그 편입·상태바 인디케이터·Sparkles 폴백 프론트·⌘K ⌘S 표본 엔트리.
- **Phase D 통합(단독)**: 배선 잔여·문서(키맵 문서·features/git.md 폴백·ipc-contract·data-model·
  qa6 Wave H 실기·갭 §7 정정) + 전체 verify + vite build.
- **Phase E 검토**: 4렌즈(opus+xhigh — 정확성 렌즈: 유예 창·2단 삼킴·오버라이드 하위호환 집중) →
  적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차 → 커밋(dev).

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| 마이그레이션 전략 2(!modalVisible)·전략 3(find 분기) | 기각(이번) — 보수 채택(행동 변화 0). 후속 재검토 가능 |
| chord 중재 B(softDispatch 딥임포트)·D(monaco chord 흡수)·C(⌘K 회피) | 기각 — A안(에디터 포커스 위임+유예 창) 채택 |
| chords 배열 직렬화(F) | 기각 — 선택 필드(E)·구버전 무음 소실 방지 |
| when 자체 미니 파서·배열 AND/NOT | 조건부 폴백만 — ContextKeyExpr 로드 게이트 실패 시 |
| when 오버라이드 노출(사용자 편집) | 보류 — 1차 내부 전용 |
| 비US 역매핑 테이블·shift 한정 code 승격·네이티브 레이아웃 맵(FFI) | 기각/보류 — 이미 code 경로 동작(실익 0)·범위 초과 |
| monaco ContextKeyService 전면 위임(컨텍스트 등록까지) | 기각 — 평가기만 재사용(c'), 소스는 pull 게터 |
| chord 인디케이터 생략 | 기각 — monaco status 가 no-op 이라 필수 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 커맨드 없음 — 폴백은 기존
  커맨드 반환 확장, bindings 재생성 확인).
- 4렌즈+적대적 검증+메인 2차 통과. 초점: **monaco chord 21건 실보존**(유예 창 코드 논증+qa6 실기)·
  2단 무조건 삼킴의 오입력 방지와 수식어 단독 예외·오버라이드 하위호환(구버전 파서에 chord 필드
  통과·기존 저장분 무손상)·ContextKeyExpr bun:test 게이트 결과·기존 21 엔트리 행동 변화 0(보수
  마이그레이션 검증)·useGlobalKeymap 반전의 5곳 회귀·인디케이터 표시/해제 타이밍·Sparkles 폴백
  제외·상한 규칙 동일성.
- 문서: 키맵 문서(chord·when·인스펙터)·features/git.md(폴백)·ipc-contract·qa6 Wave H 실기 항목·
  갭 분석 §7 정정·§7 chord/when 종결 표기.
