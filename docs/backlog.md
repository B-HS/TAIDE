# 백로그 — 3순위 후보 (Phase 7.5 이후 재검토)

> 2026-08-06 확정. **1·2순위가 전부 끝난 뒤 다시 검토한다.**
> 지금 구현하지 않는다. 우선순위·범위는 재검토 시점에 다시 정한다.
> 1·2순위 목록과 진행 상황은 `docs/roadmap.md` Phase 7.6.

## 판단 근거

전부 "있으면 좋지만 없어도 IDE 로 쓸 수 있는" 것들이다.
1·2순위(찾기/바꾸기·진단·브랜치·아웃라인 등)는 **코드를 읽고 고치는 핵심 루프**라 먼저 간다.

## 후보 목록

| 항목 | 내용 | 선행 조건 / 메모 |
|------|------|------------------|
| **멀티 윈도우** | Move/Copy into New Window, 새 OS 창으로 탭 분리 | 요구 7번과 탭 context menu 4항목(Move/Copy into New Window, Reveal in Explorer View 일부)의 전제. `capabilities/main.json` 이 단일 `main` 창 전제라 권한 구조부터 손봐야 한다. **규모가 크고 다른 기능의 전제가 아니라서 뒤로 미뤘다** |
| **Zen / 포커스 모드** | 사이드바·탭바·상태바 숨김 토글 | layout 도메인에 표시 상태 필드 추가 필요 |
| **스니펫** | 사용자 정의 코드 조각 + Monaco completion 기여 | LSP completion 어댑터와 우선순위 조정 필요 |
| **작업 러너 패널** | `package.json` scripts / `Cargo.toml` / Makefile 감지 후 실행 | 터미널 세션 재사용. pty 도메인 위에 얹으면 된다 |
| **git 충돌 해결 UI** | 3-way merge 뷰 | Monaco DiffEditor 는 있으나 3-way 는 별개. `git.md` 2차 범위 |
| **임의 두 파일 비교** | 파일 선택 → diff 탭 | `DiffPane` 재사용 가능. 파일 선택 UX 만 필요 |
| **북마크 / 북마크 패널** | 줄 단위 북마크 + 목록 | 영속화 위치(project.json vs 별도 파일) 결정 필요 |
| **에디터 설정 확장** | word wrap · minimap 토글 · 들여쓰기 가이드 · 줄 번호 스타일 | 현재 `code-editor.tsx` 에 **하드코딩**돼 있다. settings 필드 추가 + 설정 UI |
| **접근성** | 키보드만으로 전체 조작, 포커스 트랩, 스크린리더 라벨 | Radix 가 상당 부분 제공. 전수 점검이 필요한 성격 |
| **pptx LibreOffice 폴백** | 외부 `soffice` 감지 후 PDF 변환 | **제외 확정**(2026-08-06). 재검토 시에만 되살린다 |
| **remote-control** | 웹에서 프로젝트 화면 미러링 | **보류 확정**. 잠자기 중 동작이 OS 정책상 불가 — 범위 재합의가 선행돼야 한다. `docs/research/remote-control.md` |

## 재검토 시 확인할 것

- 1·2순위 구현 중 이 목록의 선행 조건이 이미 충족됐는지 (예: layout 필드 추가 여부)
- 실사용에서 실제로 아쉬웠던 것이 무엇인지 — 목록 순서보다 그것이 우선한다
