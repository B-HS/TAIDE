# Git 워크플로 · 커밋 정책 (2026-08-06)

> 사용자 확정. 이후 이 레포의 모든 작업에 적용한다.

## 1. 리모트 · 브랜치

| 항목 | 값 |
|------|-----|
| 리모트 | `origin` = `https://github.com/B-HS/TAIDE.git` (비공개) |
| 기본 브랜치 | `main` |
| `main` | **prod**. 릴리스 대상 |
| `dev` | **개발 브랜치**. 모든 작업이 여기 쌓인다 |

- 작업은 `dev` 에서 한다. **`main` 에 직접 커밋하지 않는다.**
- 병합은 **직접 머지**(사용자 확정). PR 을 만들지 않고 지시 시 `dev` → `main` 머지 후 푸시한다.

## 2. 커밋 · 푸시 자동화 (사용자 확정)

레포 설정에 기록했다.

```
git config llm-rules.auto-commit true
git config llm-rules.auto-push true
```

- 논리 단위 작업이 끝나고 `bun run verify` 가 통과하면 **요청 없이 `dev` 에 커밋·푸시한다.**
- 자동이어도 아래는 그대로 지킨다.
  - Conventional Commits, type 은 영어 소문자 + 설명은 한국어
  - author 는 사용자 단독. `Co-Authored-By` / Claude 트레일러 **금지**
  - `git add -A` 금지 — 경로를 명시해 선별 스테이징
  - `git push --force` 금지
  - 커밋 전 `git status` / `git diff` 로 의도치 않은 파일이 섞였는지 확인
- **`main` 으로의 머지·푸시는 자동에서 제외한다.** 사용자가 지시할 때만 한다.

## 3. 커밋 메시지 스타일

기존 히스토리를 따른다. `type: 한국어 설명` (scope 미사용).

```
feat: TAIDE Phase 0~7.5 구현
docs: Phase 7.5 구현 결과·WKWebView IME 버그 원인 기록
```
