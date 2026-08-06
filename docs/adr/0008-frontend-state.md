# ADR-0008 — 프론트 상태: TanStack Query(Rust 데이터) + 로컬 표시 상태

- 상태: 승인 (2026-08-06)
- 관련: ADR-0004, `docs/research/react-frontend-stack.md`, `docs/ipc-contract.md`

## 맥락

view 는 Rust 상태의 프로젝션이다(ADR-0004). "Rust 상태"를 컨벤션(query.md)의 "서버 상태"로 취급할 수
있는지, view 상태 도구를 무엇으로 할지 결정한다.

## 결정

1. **Rust 소스 데이터는 TanStack Query v5 로 소비한다.** `invoke` 래퍼를 `queryFn` 으로 쓰고,
   Rust 이벤트 수신 시 `invalidateQueries` 로 갱신한다. 컨벤션(query.md)의 queryOptions 팩토리·
   QUERY_KEY 중앙 관리를 그대로 적용한다.
2. **Tauri 환경 보정을 전역 기본값으로 고정한다** (research §3 확정안):
   - `networkMode: 'always'` (브라우저 오프라인 판정이 IPC 를 멈추는 것 방지 — 필수)
   - `retry: 0`, `refetchOnWindowFocus: false`, focusManager 는 Tauri 윈도우 포커스 이벤트로 재연결
   - `staleTime` 은 이벤트 기반 invalidate 가 정확성을 책임지므로 길게(기본 60s+) 둔다
3. **고속 스트림(터미널 출력, LSP 진단 스트림)은 Query 를 쓰지 않는다.** Tauri `Channel` 로 받아
   소비자(xterm/Monaco)에 직접 전달한다.
4. **zustand 는 도입 보류.** 도메인 상태(탭·레이아웃 포함)는 전부 Rust 소유이므로 view 전역 store 의
   실수요가 아직 없다. 드래그 중 표시 같은 표시 상태는 컴포넌트 로컬 useState 로 충분.
   교차 위젯 고빈도 표시 상태가 실증되면 컨벤션 절차(frontend.md §6, acknowledge 기록)로 도입한다.
5. **localStorage·zustand persist 로 도메인 상태를 저장하지 않는다** (ADR-0004 §5).
   react-resizable-panels 의 `useDefaultLayout({ storage: localStorage })` 도 사용 금지 —
   레이아웃은 Rust layout 도메인이 정본이므로 controlled(defaultLayout=query 값,
   onLayoutChanged→mutation) 로 연결한다.

## 주의 (research 와의 차이)

`docs/research/react-frontend-stack.md` 의 "책임 경계" 표는 탭·스플릿을 zustand+persist 로 두는
일반 패턴을 제시하지만, **TAIDE 는 ADR-0004 에 따라 이를 채택하지 않는다.** 레이아웃의 단일 출처는
Rust 다. 구현 시 research 의 해당 절을 이 ADR 로 교정해 읽는다.

## 결과

- 이벤트→invalidate 브릿지는 도메인별 1개 Provider(또는 entities query 파일 내 구독 훅)로 표준화한다.
  Rust 는 이벤트를 디바운스·배치·echo 플래그와 함께 발행한다(research 함정 §6).
- `listen()` 의 Promise 해제 함정(research 함정 §7)은 `useTauriEvent` 공용 훅으로 흡수한다
  (`architecture.md` §6).
