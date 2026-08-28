# d-45 — 테마 프리뷰 드래그 중 앱 전체 간헐 무반응(외관 IPC 홍수) 수정 계약 (2026-08-28)

> 발견: qa6 실기(사용자) — "테마 create 에서 컬러 피커 드래그(특히 팝오버 밖으로) 중 앱 전체가
> 무반응, 간헐적으로 회복". 재현 양상 확정 문답: 앱 창 밖 아님(피커 밖)·전체 무반응·회복 반복.

## 0. 근본 원인 (메인 실증 — 원격 대조 계측 + tao 소스 실물)

- 컬러 피커 드래그는 pointermove 마다 `onChange` → `setDraft` → 프리뷰 effect →
  `useThemePreview.setPreview`(전체 테마 재해석 `toResolvedThemeFromDraft`) →
  `ThemeProvider` layout effect 가 **매 move**: ① `applyThemeVariables`(루트 CSS 변수 전량)
  ② `applyShikiTheme`(highlighter.loadTheme + monaco API 재패치 + 재토크나이즈 —
  `runExclusive` 큐 밖이라 홍수 시 경합 여지도 있음) ③ **`applyWindowAppearance` =
  `getCurrentWindow().setTheme(type)` — move 당 네이티브 IPC**.
- 결정 증거: tao 0.35.3 `set_theme → set_ns_theme` 은 **동일값 단락 없이 무조건
  `NSApplication.setAppearance:`(앱 전역 외관 재적용)** 를 수행한다(레지스트리 소스
  `platform_impl/macos/window.rs:384` 실물 확인). 피킹 중 `type` 은 불변인데도 move 당
  전역 외관 패스가 메인 스레드에 적체 → **앱 전체 무반응·큐 소진 시 간헐 회복** — 증상 정합.
- 대조 계측: 원격 페이지(이 IPC 축이 실효 없는 환경)에서 240무브 스크립트 드래그 —
  프레임 갭 max 47ms·롱스톨 0. 웹 측 파이프라인 단독으로는 스톨 미재현 → 네이티브 축 특정.
  (단 실창은 모나코 대형 모델이 열려 있어 shiki 재토크나이즈 부하도 실재 — ②의 상한도 필요)

## 1. 수정 방향 (근본 — 3층)

1) **외관 IPC 를 type 변화 시에만**: `applyWindowAppearance` 호출을 마지막 적용 type 과 다를
   때만 수행(호출자 실사 후 가드 위치 결정 — 현 소비처가 ThemeProvider 뿐이면 effect ref 가드,
   복수면 모듈 가드). 피킹 중 IPC 0회가 된다 — 이 건의 직접 원인 제거.
2) **프리뷰 적용 rAF 코얼레싱**: `useThemePreview.setPreview` 를 프레임당 1회로 코얼레스
   (최신 draft 만 유지, rAF 에서 `toResolvedThemeFromDraft`+store set). 전체 재해석·CSS 변수·
   구독자 리렌더에 상한.
3) **shiki 재테마 leading+trailing 디바운스**: 단발 호출(부팅·테마 전환)은 즉시, 연속 홍수는
   정지 후 트레일링 1회(상수화된 지연). 모나코 재토크나이즈가 드래그 중 반복되지 않게.
- 검토 질문(렌즈): `attachShikiTokensProvider` 의 `runExclusive` 편입 여부(홍수 억제 후에도
  단발×재초기화 경합이 남는지), clearPreview 와 rAF 펜딩의 순서(닫을 때 펜딩 프리뷰가 늦게
  적용되는 역전 금지 — rAF 취소 필수).
- 범위 외: ColorPicker 자체의 이벤트 스로틀(소비처 층에서 제어가 정공)·tao 업스트림 수정.
- 검증: typecheck/eslint/prettier/bun test + 원격 계측 재실행(전후 비교) + 사용자 실기 재현
  확인(실창 무반응 소멸)이 최종 오라클. spec 09(테마 전환) 회귀 확인.

## 2. 실행·검토

- 구현 fixer(sonnet+xhigh, TS 만) → 검토 1렌즈(opus+xhigh, 근본성·회귀 — 축소 근거: 원인을
  메인이 소스·계측 이중 실증, 수정 후 사용자 실기 확인이 붙음) → major 시 적대적 → 메인 2차.

## 3. 기록 (구현·검토·검증)

- **구현**: wf_ea8e2032 fixer(sonnet+xhigh) — 3층 + 순수 유틸 2종(frame-coalescer·
  leading-trailing-debouncer, 스케줄러 주입식 단위 테스트 11건) 신설. 주요 결정: 가드는
  유일 소비처인 ThemeProvider ref(React Compiler 의 렌더-중 ref 접근 금지로 useState 지연
  초기화 채택 이력)·shiki 적용부는 기존 `runExclusive` 큐에 편입(applyShikiTheme 이 원래 큐
  밖이라 잠재 레이스였음 — 근본 봉쇄)·트레일링 후 즉시 유휴 복귀(다음 단발 즉시성 보존).
- **검토 1렌즈**(opus+xhigh, wf_68ae3057): major 0·minor 5·info 5 — 전건 수용(F-10 기록만).
  수용 반영(wf_52cfd038): F-01 다중 창 전역 외관 어긋남 → 창 focus 시 ref 리셋 / F-02 실패
  시 가드 롤백(+프로미스 반환·로깅) / F-03 공허 테스트 실테스트화 / F-04 코얼레서를 스토어
  옆 모듈 싱글턴으로 승격(수명 일치 — 늦은 flush 역전 구조적 불가, payload 에 queryClient
  동승) / F-05 applyShikiTheme non-async 화+도달 불가 catch 제거 / F-07 미사용 cancel 제거 /
  F-08·F-06 JSDoc 사실화(150ms 는 미계측 창·즉시성은 유휴 한정) / F-09 as 단언 3건 제거.
  verified_ok 요지: 가드 정당 경로 전수 통과·네이티브 외관 기록자 단일·React Compiler 실컴파일로
  클린업 아이덴티티 검증·runExclusive 자기 데드락 없음·트레일링 stale 불가(FIFO)·부팅/재초기화
  즉시성 보존·spec 09 무회귀.
- **F-06 잔여 트레이드오프(기록)**: 직전 적용 150ms 내 재호출은 트레일링으로 접혀 CSS 변수
  (즉시)와 모나코 토큰(최대 150ms)이 그 구간만 분리될 수 있음 — 최종 상태는 항상 최신.
- **F-10(기록)**: 유틸 2종은 현재 소비처 각 1곳 — 스케줄러 주입 단위 테스트 목적의 분리이며
  2번째 소비처 발생 시 shared 승격 확정(각 파일 JSDoc 에 명시).
- **검증**: typecheck·eslint 0err·prettier·bun **1514/0**(신규 유틸 테스트 포함) / spec 09
  통과 / 원격 재계측 — 240무브 드래그 프레임 갭 max 44ms·롱스톨 0 + 드래그 중 루트 인라인
  스타일 변이 관측(프리뷰 실적용 = 회귀 없음). **최종 오라클 = 사용자 실기**(네이티브 창
  피커 드래그 — IPC 홍수 제거 효과는 네이티브에서만 체감).

## 4. 후속 v2 — 사용자 결정: 릴리스 시 적용 (2026-08-28)

- 실기 확인 결과 "많이 줄었지만 아직 프리징이 있다" + **사용자 결정: 드래그 중 실시간 전역
  프리뷰 불필요 — 마우스를 놓을 때(click up) 적용으로 변경**.
- 설계: `ColorPicker` 내부에 드래그-로컬 HSV 상태를 두어 팝오버 내 썸/그라디언트만 라이브로
  추적하고, `onChange`(→ setDraft → 전역 프리뷰 파이프라인)는 **pointerup 에서 1회** 커밋.
  키보드 스텝·hex 입력은 기존대로 즉시 커밋(저빈도 이산 이벤트). pointercancel/
  lostpointercapture 는 로컬 상태만 폐기(커밋은 up 만). §1 의 3층(가드·코얼레스·디바운스)은
  키보드 연타 등 잔여 버스트 방어로 유지.
- 효과: 드래그 중 전역 작업(재해석·CSS 변수·shiki·리스트 리렌더) 0회 — 잔여 프리징의 원천
  제거. 실기 재확인이 최종 오라클.
- **v2 구현·검토 기록**: wf_89ee1747 fixer — 드래그-로컬 HSV(순수 전이 함수 3종+테스트 6건)·
  pointerup 1회 커밋·cancel/lostcapture 폐기. 통합 렌즈(wf_3174823a) minor 2·info 1 전건
  수용(wf_6af753e2): A-1 팝오버 닫힘 시 dragHsv 잔존(캡처 요소 언마운트로 up/lostcapture
  미도달) → onOpenChange 폐기 트리거 추가 / A-2 JSDoc "hex preview 라이브" 오서술 정정 /
  A-3 비주버튼 드래그를 pointerdown 에서 차단(우클릭 릴리즈 조기 커밋 봉쇄). verified_ok:
  캡처 밖 up 커밋·up→lostcapture 멱등·react-dom 실물로 stale 클로저 부재 검증·클릭 커밋·
  transparent 폴백·키보드/hex 무회귀. bun 1521/0. 잔여 = 사용자 실기 재확인.
- **실기 확인(2026-08-28, 사용자): 피커 드래그 프리징 완전 소멸 — 최종 오라클 통과, d-45 종결.**
  커밋 `9e4a822`(fix — 3층+v2 일괄).
