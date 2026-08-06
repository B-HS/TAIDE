# 2026-08-06 — 기술 결정 확정 (문서화 세션 종료 시점)

> 리서치·검수 후 남은 결정 지점을 사용자가 전건 추천안으로 확정. 근거 상세는 각 문서 참조.

| 결정 | 확정안 | 근거·반영 위치 |
|------|--------|---------------|
| 파일 트리 | **Rust 소유 트리 + flat rows + @tanstack/react-virtual** (외부 트리 라이브러리 없음) | ADR-0004 정합·수만 파일 성능. `features/explorer-sidebar.md` §2.1 |
| LSP 클라이언트 | **자체 경량 클라이언트** (Monaco provider 어댑팅, monaco-languageclient 포크 경로 기각) | 번들·수명주기·버전 통제. ADR-0007 §결정3, `features/lsp.md` §3 |
| IPC 타입 생성 | **tauri-specta =2.0.0-rc.25 핀 고정** (RC 감수) | 드리프트 원천 차단. ADR-0011, `tech-stack.md` |
| 커밋 대상 규칙 | **staged 있으면 그것만, 없으면 "전체 스테이지 후 커밋?" 확인** (현행 VSCode 실동작) | `features/git.md` §3 |
| 탭 DND | **dnd-kit 레거시 라인**(core 6.3.1 + sortable 10.0.0) — 신 라인 @dnd-kit/react 는 pre-1.0 | `tech-stack.md` |

## 구현 진행 루프 합의 (2026-08-06 추가)

- **실기기 테스트·UI/UX 판단은 사용자 피드백 루프로 진행한다**: AI 는 문서 스펙+참조 제품 기준의
  기본값으로 구현·기계 검증(typecheck·lint·unit·cargo test)까지 완료하고, 실행 확인·시각 피드백은
  사용자가 앱을 돌려본 뒤 주는 피드백을 받아 수정한다. 디자인 취향 질문으로 구현을 멈추지 않는다.
- **Phase 8(배포·서명)은 Phase 0~7 전체가 로컬에서 테스트·구현 확인 완료된 후에만 진행한다.**
- 따라서 Phase 0~7 은 사용자 의사결정 대기 없이 무중단 구현 가능. 중간 체크포인트는
  "질문"이 아니라 "실행해 보시고 피드백 주세요" 형태로만 존재한다.

## 진행 방식 합의 (이 세션에서 확립)

- Workflow 리서치는 **opus + effort medium** 서브에이전트로, Fable 상속 에이전트(general-purpose 등)
  사용 금지. 지휘·문서 작성·핵심 결정은 메인 모델이 직접.
- 커밋은 수동(사용자 요청 시에만). 이 레포는 단독 작업 레포로 **main 직접 커밋 허용**
  (`git config llm-rules.allow-main true`, 사용자 커밋 지시 기반 — 브랜치 워크플로 도입 시 재검토).
- 구현 세션은 `docs/roadmap.md` Phase 0 부터, 버전 정본은 `docs/tech-stack.md`,
  API 정본은 `docs/research/*.md`(미확인 항목은 `docs/quality-assurance/docs-review-checklist.md` §4
  실측 후 진행).
