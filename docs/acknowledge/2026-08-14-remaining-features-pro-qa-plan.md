# 잔여 기능·전문 QA 캠페인 계획 (2026-08-14, 새 세션)

> 세션 복원 직후 사용자 결정 4건 + 역할 배정 갱신. 직전 계약: `2026-08-14-hotexit-remote-password-contract.md`.
> 이 계약은 "무엇을 어떤 순서로" 만 확정한다. 각 단계의 상세 설계는 착수 시점에 별도 계약으로 확정한다.

## 1. 사용자 결정 (2026-08-14 확정)

| # | 결정 지점 | 선택 |
|---|-----------|------|
| 1 | 잔여 기능 범위 | **갭 P0 잔여 + P1 전부** = P0 잔여 9건(Code Action/Quick Fix·팔레트 @/: + Workspace Symbol·Semantic Tokens·Breadcrumbs·Git 4종(3-way 머지·줄 stage·커밋 상세·파일 히스토리)·터미널 OSC133·태스크 러너·chord/when 키맵(+shift+기호 캡처)·스니펫) + Remote 하드닝 minor 9건 + Hot Exit 미세 3건 + **P1 10건**(AI Inline Edit ⌘K·Go to Impl/TypeDef/Decl·CodeLens+LSP Folding·F8 순회·Search Editor+검색 히스토리+gitignore 토글·git revert/tag/원격 브랜치+파일 blame 뷰·AI 커밋 메시지·Zen/멀티윈도우·Emmet+Format on Type·설정 파일 탭 편집+플러그인 설치 UI+VSIX grammar). backlog 소품(북마크·접근성 등)은 제외 — 전문 QA 후 재검토 |
| 2 | 착수 전 실기 확인 | **qa6 체크리스트 전체 선행** — W1~W7 + 후속 1차 + 확장 1~3차 전량을 사용자가 실기 검증한 뒤 기능 착수 (스모크만/생략안 기각) |
| 3 | Phase 8(서명·공증) | **전문 QA 통과 후** 마지막 (배포 직전 1회) |
| 4 | e2e 리서치 | **지금 병렬 착수** — 실기 QA 와 동시에 실행 경로(tauri-driver macOS 실태·remote-control 웹 미러 경유 자동화·러너 선택지) 조사. 의존성 추가는 조사 후 승인 요청 |

## 2. 전체 순서

1. ~~실기 QA 일괄~~ **취소** (2026-08-14 후속 지시) — "구현 완료분은 간단 구동해봤다"(스모크 수준
   치명 결함 없음) 가정으로 대체. qa6-checklist 전 항목은 전문 QA 로 이월, 실기 미검증 KNOWN
   ISSUE 는 존속. 결정 2(전체 선행)를 사용자가 직접 번복한 것 — 재론 아님
2. **잔여 기능 구현** — 결정 1 범위. 웨이브별 계약(정찰 → 추천안 패키지 → 확정) 반복.
   **웨이브 편성 A→I 확정**(2026-08-14 사용자 승인):
   A LSP 인텔리전스(Code Action/Quick Fix+on Save·Impl/TypeDef/Decl·CodeLens·LSP Folding·Peek/F8 확인) →
   B 하드닝 마감(Remote 9·Hot Exit 3) → C Git 확장(3-way 머지·줄 stage·커밋 상세·파일 히스토리·
   revert/tag/원격 checkout·파일 blame 뷰) → D 탐색·검색(팔레트 @/:·Workspace Symbol·Breadcrumbs·
   Search Editor·검색 히스토리·gitignore 토글) → E 터미널·태스크(OSC133·태스크 러너·Run Selected
   Text) → F 에디터 표현(Semantic Tokens·스니펫·Format on Type/Paste·Emmet) → G AI(Inline Edit
   ⌘K·AI 커밋 메시지) → H 키맵 엔진(chord/when·shift+기호 event.code) → I 셸·워크스페이스(Zen·
   멀티 윈도우·설정 파일 탭 편집·플러그인 설치 UI·VSIX grammar).
   순서 근거: A=최대 공백, B=보안 조기 마감, H=키맵 전면 개정이라 타 웨이브 안정화 후,
   I=최대 구조 변경이라 최후
3. **전문 QA** — 기능 전수 리스트업 → 체크리스트 신설 → 기능별 심층 검토 + e2e +
   아키텍처·추상화 전수 감사 (§3.1)
4. **Phase 8** — 서명·공증

## 3. 역할 배정 (이번 캠페인 — 2026-08-14 "완벽 우선" 지시로 전면 상향)

| 작업 | 배정 |
|------|------|
| 기능 구현 웨이브 | 계약(스파인)·구현 **sonnet+xhigh**·렌즈 검토 **opus+xhigh**(4렌즈 상설 — §3.1)·적대적 검증 **opus+high**·메인 2차 |
| QA·로직 점검·버그 확인 | **Workflow opus+xhigh**, 기능 심층 QA 는 **opus+max** |
| 버그 파악·디버그 | **fable+high** (입력 관련 한정이던 것을 캠페인 전체로 확대) |
| QA 발 코드 수정 | **sonnet+xhigh** |
| 정찰·리서치 | **opus+high** (medium 에서 상향) |

### 3.1 품질 원칙 (2026-08-14 사용자 추가 지시)

> **"효율보다는 최대한 완벽한 프로덕트. 추상화 등도 제대로 신경쓸 것."**

- 속도·토큰 절약을 이유로 검토·검증 단계를 축소하지 않는다. 반쪽 기능 금지 재확인.
- **설계·추상화 렌즈 상설화**: 모든 기능 웨이브 검토는 4렌즈(계약 정합 / 정확성 / 보안 /
  **설계·추상화**) — 설계 렌즈는 FSD 배치·타입 유도(원본 파생)·도메인 경계(Route/Service/compose,
  Rust service 순수성)·조기/누락 추상화·중복·네이밍 정확성을 본다.
- **전문 QA 에 아키텍처·추상화 전수 감사 축 편입**: 기능별 동작 QA 와 별개로, 기존 코드베이스
  전체(프론트 FSD·entities 계층·shared 승격 누락 / Rust 도메인 경계·compose 격리·타입 파생)를
  감사하는 축을 정식 단계로 둔다.

- 위임은 규모 무관 Workflow 로만. 에이전트 보고 불신 — 핵심 주장은 메인 실물 재검증.
- 실행(`bun run tauri dev`·`bun run tauri build`)은 사용자만. 에이전트 셸 앱 실행 금지.

## 4. 기각·유보

| 안 | 처리 |
|----|------|
| 핵심 4건 스모크 후 즉시 기능 착수 | 기각 — 사용자가 전체 실기 QA 선행 선택 |
| backlog 소품(북마크·Zen 제외 소품·접근성 등) 이번 포함 | 유보 — 전문 QA 후 재검토 (Zen/멀티윈도우는 P1 로 포함됨) |
| e2e 를 QA 설계 시점에 리서치 | 기각 — 지금 병렬 착수로 앞당김 |
