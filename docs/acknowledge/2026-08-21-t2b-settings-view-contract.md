# T2-B 1호 — settings-view 섹션 위젯 분해 계약 (2026-08-21, d-29)

> 정본: 감사 `2026-08-18-architecture-audit.md` §5 T2-B 행("settings-view(섹션 위젯)").
> goal("추천안대로 배치 계속진행") 하 HANDOFF §8-2 순번 — T2-B 는 파일별 단독 배치
> (editor-pane 관심사 분해 d-13 선례).
> 착수 전 메인 실사: `widgets/settings-view/settings-view.tsx` 926줄(감사 927 실측 유지 —
> d-28 이 상수 4개만 소거). 동거: 상수 블록(:75~) + 옵션 목록 3종 + hooks 19회
> (useState/useQuery/useMutation) + SettingsSection(features) 소비. 형제 파일 선례 실재
> (agent-cli-status-row 등 3파일 — 부분 분해 이미 시작된 형태).

## 1. 범위·원칙

- **섹션 단위 위젯 분해** — editor-pane 선례(관심사별 파일·1파일 1컴포넌트): 각 설정 섹션
  (에디터·터미널·자동 저장·AI·원격·키맵·테마·에이전트 등 — 실사로 섹션 경계 확정)을
  `widgets/settings-view/` 하위 파일로 분리. 섹션 전용 상수·옵션 목록은 해당 섹션 파일로
  동행(2곳 이상 공유분만 shared — 임의 승격 금지).
- **동작·마크업 무변경 절대 조건**: 렌더 트리 동등(스크롤 앵커·SETTINGS_SCROLL_OFFSET_PX
  내비게이션·검색/점프가 있으면 그 배선 보존), settings 쿼리/뮤테이션 배선 의미 동일. props
  는 상위(컨테이너)에서 주입 — 섹션 파일이 직접 쿼리를 갖는지 여부는 기존 형제 파일
  (agent-hooks-project-list 등) 선례 실사 후 일관 선택·근거 기록.
- 컨테이너(settings-view.tsx)는 조립·공유 상태만 남김(목표: 대폭 축소 — 수치 목표는 실사
  후 자연 결정, 억지 분할 금지). FSD·barrel 금지·named export.
- 범위 외: 설정 항목 추가/제거·UI 리디자인·T2-B 다른 파일·d-28 이월(불리언 기본값 9종은
  이 분해에서 자연 해소되면 겸사, 아니면 이월 유지).

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독, TS 전용) → 메인 2차 → Phase E 4렌즈(구조 동등성 —
  렌더 트리·배선 diff / 실사충실성 — 섹션 경계·상수 배치 / 설계 — 분해 입도·선례 정합 /
  계약 — 표면·컨벤션) → 적대적(major 이상) → confirmed 수정 → 커밋 → prod 병합.
- 검증: `bun run verify` + `bunx vite build` exit 0 + bindings 무변경. 실기 이월(qa6):
  설정 화면 전 섹션 렌더·값 변경 반영·스크롤 내비게이션.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

(작성 예정)
