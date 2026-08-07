# Phase 7.5 완료 · 7.6 결정 기록 (2026-08-07)

> 대응 커밋: `bf0a713`. 결정 1건 = 이 문서 안의 절 1개.

## 1. 범위 결정

| 항목 | 결정 | 이유 |
|------|------|------|
| **pptx LibreOffice 폴백** | **제외** | 개요 미리보기로 종결. 외부 바이너리 감지·변환은 의존을 늘리는데, pptx 원본 충실 렌더는 어차피 불가능하다는 것이 이미 조사로 확인됐다 |
| **remote-control** | **보류** | 잠자기 중 동작이 OS 정책상 불가(`docs/research/remote-control.md`). 사용자 기대와 제약이 달라 범위 재합의가 선행돼야 한다 |
| **멀티 윈도우** | **백로그로 이동** | 요구 7번과 탭 메뉴 4항목의 전제이나 **규모가 크고 다른 기능의 전제가 아니다.** 1·2순위(코드를 읽고 고치는 핵심 루프)를 먼저 채우는 것이 실사용 가치가 크다 |
| **3순위 전체** | `docs/backlog.md` 로 분리 | 1·2순위 완료 후 재검토. 항목별 선행 조건을 함께 적어 재검토 시 판단 근거가 남게 했다 |

## 2. `TabKind::Preview` 를 만들지 않기로 한 결정 (7.5-E)

**배경**: `docs/features/preview.md` §1 은 `TabKind::Preview { path, kind }` 신설을 제안했다.

**결정**: 만들지 않는다. 기존 `TabKind::File { path }` 를 그대로 쓰고
`src/widgets/editor-area/pane-node-view.tsx` 에서 확장자로 Monaco/미리보기를 분기한다.

**이유**:
- 레이아웃 스키마는 **영속 데이터**다. 필드를 늘리면 마이그레이션이 따라온다.
- "어떤 렌더러로 그릴지"는 도메인 상태가 아니라 **view 판단**이다(ADR-0004 의 경계).
- 기존 탭을 그대로 쓰므로 preview/pin 규칙(`tabs.md` §3)이 자동으로 유지된다.

**기각한 대안**: 스키마 확장 + 마이그레이션. 얻는 것 없이 영속 데이터 리스크만 늘어난다.

## 3. `theme_get_current` / `locale_get_current` 가 시스템 값을 인자로 받는 형태

**배경**: `followSystemTheme` 이 백엔드에서 **no-op** 이었다 — `theme_get_current` 가
`settings.theme_id` 만 읽어 설정 UI 의 체크박스가 아무 일도 하지 않았다.

**결정**: `theme_get_current(systemTheme)` / `locale_get_current(systemLanguage)` 로
**판단은 Rust, OS 값 감지는 view 가 센서 역할**.

**이유**: Rust 가 웹뷰 없이 OS 테마·언어를 읽으려면 플랫폼 분기가 늘어난다.
view 에는 이미 `prefers-color-scheme` 과 `navigator.language` 가 있다.
ADR-0004 가 요구하는 것은 "상태 소유"이지 "센서 소유"가 아니다.

## 4. IME 어댑터를 유지하기로 한 결정 (7.5-G 후속)

**배경**: WKWebView 가 조합 이벤트를 발생시키지 않아 한글이 깨졌고,
`resolveImeInput` 어댑터(백스페이스 + 재전송)로 해결했다.

**확인된 사실**: tao 0.36.0 / wry 0.56.0 를 받아 diff 한 결과 **IME·first responder·keyDown
관련 변경이 0건**이다. 버전 업으로는 해결되지 않는다.

**알려진 트레이드오프**: 조합 중 중간 글자가 pty 에 입력됐다 지워진다.
셸 라인 편집·vim insert 에서는 결과가 맞지만, 원시 키를 기대하는 전체화면 앱에서는 이상 동작 가능.
대안(조합 경계 추론 후 확정 시 1회 전송)은 자체 누출 케이스가 문서화돼 있어 채택하지 않았다.

## 5. Git 워크플로 (사용자 확정)

`docs/acknowledge/2026-08-06-git-workflow.md` 참조. 요약:
`main`=prod / `dev`=개발, 자동 커밋·푸시 ON(`git config llm-rules.auto-commit/auto-push true`),
`main` 머지는 **직접 머지**(PR 없이) + 지시 시에만.
