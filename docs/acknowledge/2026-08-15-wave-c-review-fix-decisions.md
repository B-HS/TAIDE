# Wave C 검토 확정 결함 수정 — 처리 결정 (2026-08-15)

> `2026-08-15-wave-c-git-contract.md` Phase C(검토) 산출물 중 확정/불확실 결함 전건을 수정하고,
> minor 결함은 이 세션에서 직접 재판정했다. 결함 id 별 처리와 근거를 남긴다.

## 수정함

| id | 요약 | 처리 |
|----|------|------|
| L1-0 (major) | 워킹트리 에디터 Unstage가 index 좌표 백엔드에 workdir 좌표를 전달 | 워킹트리 에디터 컨텍스트 메뉴에서 **Unstage Changes 제거**(Stage Changes만 유지). Unstage 는 이미 정합하는 `DiffPane`(staged=true, indexVsHead 좌표)로만 제공한다. |
| L1-1 | 미추적 파일의 hunk/line stage 가 항상 실패 | `build_patch_text` 가 `---`/`+++` 를 무조건 `a/`·`b/` 로 쓰던 것을, 델타의 old/new 존재 여부(`PatchFileSides`)에 따라 `/dev/null` + `new file mode`/`deleted file mode` 확장 헤더를 함께 쓰도록 수정. (libgit2 `check_filenames` 가 `/dev/null` 을 인정하려면 `new file mode` 라인으로 status 를 ADDED 로 먼저 확정해야 함 — 재현 테스트로 실제 에러 메시지 확인 후 수정) |
| L1-2 | revert 자동 커밋이 무관한 staged 변경을 함께 커밋 | `revert_commit` 앞에 `ensure_clean_index` 가드 추가 — index 가 HEAD 대비 깨끗하지 않으면 즉시 거부 |
| L1-3 | 여러 hunk에 걸친 라인 선택이 문서와 달리 첫 hunk만 부분 적용 | `build_partial_patch` 에 `hunk_overlaps_another` 체크 추가 — 두 번째 hunk 와 겹치면 명시적 에러로 거부(문서화된 의도와 일치시킴) |
| L0-1 | revert 충돌 시 toast 만, 능동 라우팅 없음 | `RevertOutcome` 에 `conflictedPaths: Vec<String>` 추가(index.conflicts() 열거). `CommitGraph.handleRevert` 가 충돌 시 첫 충돌 파일을 `onOpenFile` 로 직접 연다 |
| L3-2 | CommitGraph 가 projectId 를 자체 쿼리(activeProjectQueryOptions)로 재조회 | `CommitGraph` 가 `projectId` 를 prop 으로 받도록 통일(GitPanel 이 이미 보유한 값을 전달), 자체 구독 제거 |
| L3-3 | COMMIT_FILES·SHOW 의 staleTime:Infinity 가 coarse invalidation 으로 무력화 | `useGitMutation` 의 `invalidateQueries` 에 `predicate`(`isGitQueryScopeMutable`) 추가 — `QUERY_KEY.GIT.REV_IMMUTABLE_SCOPES`(commit-files·show) 는 project-prefix 무효화에서 제외 |
| L3-4 | 확장자→언어 매핑이 Rust 테이블을 프론트에 손복제 | `LANGUAGE_ID_BY_EXTENSION` 값 타입을 `TaideLanguageId`(`shiki/lang-map.ts` 유도 타입)로 좁혀, 최소한 TS 두 사본 간 드리프트는 컴파일 타임에 잡히게 함. Rust 사본과의 드리프트는 여전히 수동 유지(아래 후속 참고) |
| L3-5 | CommitFileDiff 가 git-panel 슬라이스 내부에 있으면서 file-history 위젯도 참조 | `widgets/commit-file-diff/commit-file-diff.tsx` 로 독립 위젯 슬라이스 승격, 양쪽 import 갱신 |

## 기각함

| id | 요약 | 기각 사유 |
|----|------|-----------|
| L1-4 | file_log 가 모든 로컬 브랜치를 순회 | `log()`(메인 커밋 그래프)도 동일하게 `push_glob("refs/heads/*") + push_head()` 를 쓴다(service.rs:247-249) — 이 앱의 커밋 그래프는 원래 "현재 브랜치만"이 아니라 "전체 로컬 브랜치 그래프"를 보여주도록 설계되어 있고, `file_log` 는 그 설계와 **일관되게** 작성됐다. `push_head()` 만 쓰도록 바꾸면 오히려 같은 커밋을 그래프와 파일 히스토리가 다르게 보여주는 새로운 불일치를 만든다. 오탐으로 판단, 코드 변경 없음. |
| L1-5 | hunk 단위 stage 도 EOF-무개행 hunk 를 거부 — "구현 보고"와 상충 | 코드 자체는 의도된 동작이며 이미 `select_hunk_lines` 독스트링에 정확히 문서화되어 있다(라인 단위 재구성이 범위 밖이라는 설계 결정). `build_whole_hunk_patch` 도 내부적으로 `select_hunk_lines` 를 거치므로 hunk 단위도 동일 제약을 받는 것이 맞다. 지적된 "구현 보고"(hunk 단위는 제약 없음)는 이 레포에 실재하는 문서를 찾지 못했다(docs/ 전체 검색 결과 없음) — 정정할 소스 문서가 없어 코드 변경 없음. 향후 그런 보고서가 발견되면 이 노트를 근거로 정정할 것. |
| L2-1 | 원격 revert/checkout 이 hot-exit 미러와 디스크를 불일치시킬 수 있음 | 발견 보고서 자신도 "Wave C 고유가 아닌 기존 git 뮤테이션 공통 사안이므로 별도 후속 항목으로 기록 권장"이라고 결론지었고, `checkout_ref` 는 이미 `CheckoutBuilder::safe()` 로 디스크에 저장된 변경은 보호한다(파괴적 취약점 아님). 원격 세션은 이미 `pty_spawn` 으로 동급 작업이 가능해 별도 위협 모델 추가도 아니다. 이번 결함 수정 범위에서 제외하고 후속 과제로만 기록한다. |
| L0-2 | 커밋 상세·파일 히스토리 diff 가 계약의 "기존 diff 탭 재사용" 문언과 다르게 인라인/모달로 구현됨 | 계약 §4 "기각·보류" 표에서 이미 "신규 TabKind 신설 회피"를 명시적으로 선택했고, 실제 구현(인라인 `CommitFileDiff`, 모달 `FileHistoryPanel`)은 그 선택이 구체화된 결과다. `TabKind::Diff` 확장(rev 필드 추가)은 diff 탭의 `compareWith` 계약을 바꾸는 별도 설계 결정이라 이번 버그수정 범위를 벗어난다. 코드 변경 없음 — 계약 §3.2 문언과 실제 배치가 다르다는 점만 이 문서로 기록해 둔다. |
| L3-1 | 파일 히스토리가 "사이드 패널" 대신 Radix Dialog(모달)로 구현됨 | 위와 같은 이유(§4 "신규 TabKind 신설 회피" 선택의 연장). non-modal 도킹 패널로 바꾸려면 레이아웃 시스템에 새 도킹 슬롯이 필요해 이번 범위를 벗어난다. 코드 변경 없음. |
| L0-3 | tag_create 가 평면 인자 대신 `TagCreateOptions` 구조체를 받음 | clippy `too_many_arguments` 를 끄지 않고 기존 `CommitOptions` 선례대로 구조체화한 정당한 리팩터(검사기 우회 아님). 와이어 페이로드는 계약과 동등. 코드 변경 불필요 — 계약 §3.1 문언만 실제 시그니처에 맞게 갱신 대상으로 기록. |

## 후속 과제 (별도 작업으로)

- **L2-1**: 원격 세션의 git 뮤테이션(revert/checkout/pull/stash 등)이 열린 dirty 탭과 어긋날 수 있는 사안 — Wave C 범위 밖의 기존 구조적 이슈로, 별도 웨이브에서 "원격발 워킹트리 뮤테이션 시 열린 dirty 탭 알림/재로드" 설계로 다룬다.
- **L3-4 잔존 갭**: `LANGUAGE_ID_BY_EXTENSION`(TS) 은 여전히 Rust `LANGUAGE_ID_BY_EXTENSION`(service.rs) 의 수동 포팅이다. 이번 수정으로 TS 내부 두 사본(확장자맵 값 vs `TAIDE_LANGUAGE_IDS`) 간 드리프트는 컴파일 타임에 잡히지만, Rust ↔ TS 간 드리프트는 여전히 런타임에만 드러난다(신규 확장자 미포팅 시 plaintext 폴백). 근본 해결은 `git_show_file` 응답에 `languageId` 를 백엔드가 함께 내려주는 것 — 별도 백엔드 변경이 필요해 이번 범위 밖으로 기록.

## 계약 문서 갱신 필요 (참고용, 문서만)

`2026-08-15-wave-c-git-contract.md` §3.1·§3.2 는 아래 지점에서 실제 구현과 문언이 다르다. 이 세션에서는 계약 원본을 소급 수정하지 않고(과거 결정 기록 보존), 이 문서로 차이를 남긴다.

- §3.1 `git_tag_create` 시그니처: 계약 "`(projectId, name, target, message?, annotated)`" → 실제 "`(projectId, name, target, opts: TagCreateOptions)`" (L0-3)
- §3.2 커밋 상세/파일 히스토리 diff 배치: 계약 "기존 diff 탭 재사용" → 실제 "패널 내부 인라인(`CommitFileDiff`)" (L0-2)
- §3.2 파일 히스토리 위치: 계약 "사이드 패널" → 실제 "모달(Dialog, 우측 도킹 스타일)" (L3-1)
