# ⌘P 퀵오픈 UX 정비 계약 (2026-08-20, d-26)

> 사용자 실기 보고(2026-08-20): ① 방향키로 selection 이 안 됨 ② 파일 결과가 full path
> (`/Users/{u}/{proj}/app`)로 나와 가독성 나쁨 — 파일명 + "프로젝트 상대 경로" 부제(`/app`)
> 형태여야 함 ③ 파일명 매칭 위치 하이라이트 부재. "몇몇 디테일이 너무 아쉽네".
> 착수 전 메인 실사(2026-08-20, 전건 실코드):
> - 팔레트 = `widgets/command-palette/command-palette.tsx`(409줄, shadcn Command/cmdk 1.1.1·
>   `shouldFilter={false}`+자체 `fuzzyFilter`). 파일 행 렌더 = `:361-364` **`{item.path}` 단일
>   truncate span**(풀 경로·하이라이트 없음) — ②③의 직접 원인.
> - **③ 의 데이터는 기존재**: `shared/lib/fuzzy-match.ts` 의 `fuzzyMatch` 가 `indices`(매칭
>   문자 위치 배열)를 반환하고 `fuzzyFilter` 가 `{item, match}` 로 전달 중 — 렌더만 미구현.
> - **② 의 헬퍼도 기존재**: `shared/lib/relative-path` 의 `toRelativePath`·`fileNameOf`
>   (브레드크럼 사용 중). fileRows = `treeRowsQueryOptions` rows(path 형태 실사 필요 — 절대
>   경로 정황).
> - **① 원인 후보 서열**(정순 키맵 바인딩은 무관 판명 — Arrow 엔트리는 mod+Arrow·
>   terminalFocus 한정): (1순위) 선택 표시 비가시 — CommandItem 선택 스타일이
>   `data-[selected=true]:bg-accent` 인데 `--accent: var(--taide-list-hover-background)`
>   (global.css:143) — hover 색과 동일해 팔레트 배경 대비가 미미할 수 있음(선택이 움직이는데
>   안 보이는 체감). (2순위) cmdk 선택 자체 미동작 — 캡처 리스너 2중(useGlobalKeymap+팔레트
>   자체 useKeydownCapture)·CommandItem value 부재·cmdk 1.1.1×React 19 상호작용.
>   사용자 판별 정보(방향키 후 Enter 가 다른 파일을 여는지) 수신 시 계약에 반영.

## 1. 범위

### 1.1 방향키 선택 (①)

- 원인 확정이 선행 — 구현이 cmdk 소스(node_modules)·캡처 리스너 체인·선택 스타일 대비를
  실사해 (1)/(2) 중 확정 후 근본 수정. (1)이면: 팔레트 선택 전용 대비 토큰 적용(기존 테마
  토큰 체계 내 — 신규 토큰 남발 금지, `--taide-list-active-*` 계열 실사 후 재사용 우선).
  (2)이면: 원인 지점 근본 수정(캡처 리스너의 삼킴이면 해당 경로, cmdk 계약 위반이면 value
  등 정합). 추측 수정 금지 — 판정 근거 기록.

### 1.2 파일 결과 2단 렌더 (②)

- 파일 행 = **파일명(주 라벨) + 프로젝트 상대 디렉토리 경로(부제·소프트 색)** 2단(VS Code
  Quick Open 형태). `fileNameOf`·`toRelativePath` 재사용. 프로젝트 루트 직속 파일은 부제
  생략. treePage rows 의 path 가 절대 경로인지 실사 후 프로젝트 root 기준 상대화(멀티
  프로젝트 시 활성 프로젝트 root — 실사).
- cmdk value 정합: 렌더 분리 후에도 CommandItem 의 선택/식별 value 는 고유 유지(path).

### 1.3 매칭 하이라이트 (③)

- `fuzzyFilter` 의 `match.indices` 를 렌더에 연결 — 매칭 문자를 강조 표시(테마 토큰 경유·
  기존 검색 매치 강조 선례 실사 후 동일 계열). **인덱스 매핑 주의**: indices 는 매칭 대상
  라벨 기준인데 ②의 2단 분리 렌더와 정합해야 함 — 매칭 대상을 무엇으로 할지(§1.4)와 함께
  설계.
- 커맨드·심볼 모드도 동일 패턴이 국소면 포함, 아니면 파일 모드만 하고 이월 기록.

### 1.4 매칭 대상 실사 (② ③ 과 결합)

- 현재 `fuzzyFilter(searchTerm, fileRows, (row) => row.path)` — **절대 경로 전체**가 매칭
  대상(사용자 홈 경로 문자까지 매칭됨 — ②와 같은 근원). 매칭 대상을 "프로젝트 상대 경로"로
  교체(파일명 가중치 재설계는 범위 외 — 이월 기록). 하이라이트 인덱스는 이 상대 경로 기준으로
  일관.

### 1.5 범위 외

- fuzzy 알고리즘 자체(가중치·순위) 재설계. 팔레트 다른 모드(@·#·:) 구조 변경. 커맨드 표면·
  이벤트·bindings·원격 정책 무변경.

## 2. 실행·검증

- **선행 조건: d-24 검토 반영 완료 후 착수**(동일 트리 순차 — 동시 쓰기 금지).
- 구현 Workflow(sonnet+xhigh 단독, TS 전용) → 메인 2차 → Phase E 4렌즈(정확성: 선택 원인
  판정·인덱스 매핑 / 회귀: 팔레트 전 모드·Enter/preview 열기·keybinding 행 / 설계: 렌더
  구조·토큰 / 계약: 표면·i18n·컨벤션) → 적대적(major 이상) → 수정 → 커밋 → 병합.
- 실기 확증(사용자): 방향키 선택 가시 이동·상대 경로 부제·하이라이트 표시.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

(작성 예정)
