# Wave G 구현 계약 — AI (Inline Edit ⌘I·AI 커밋 메시지) (2026-08-16)

> 정찰 wf_8904a89d-424(opus+high 4축: Rust AI 도메인·프론트 AI/에디터·git 커밋 UI·Inline Edit UX 리서치).
> 하중 주장 7건 메인 실물 재검증 완료(전건 확정). **갭 분석 §5·§8 의 "git_diff 재사용·난이도 하" 전제는
> 오류로 판명** — git 도메인에 통합 diff 커맨드가 없다(git_diff_file 은 파일 전문 2벌 DiffSides, 상한 없음).
> 캠페인 계약: `2026-08-14-remaining-features-pro-qa-plan.md`(완벽 우선·4렌즈·역할 상향).

## 1. 사용자 결정 (2026-08-16)

| # | 결정 | 선택 |
|---|------|------|
| ① Wave F prod | **병합** | `56712b0` 을 main fast-forward(A~E 와 동일 기준). 실기 미검증 KNOWN ISSUE 존속 |
| ② 공통 기반 | **추천 패키지 + 설정 리네임** | provider trait `instruct` 추가(complete 무변경) + 프롬프트 파일 2종 신규 + 로더 공통화 + 취소 store 공용화 + **ai_auto_tab_provider/model → ai_provider/ai_model 리네임(serde alias 마이그레이션)** |
| ③ Inline Edit | 추천 패키지 | 기본 **⌘I**(monaco 에디터 액션, ⌘K 는 Wave H 후 재검토) + ContentWidget 입력 + **모델 무변경 프리뷰**(삭제=데코·삽입=ViewZone) + 일괄 응답 + Return 제출·⌘Enter 수락·Esc 거절 + 코드펜스 수신측 스트립 |
| ④ 커밋 메시지 | 추천 패키지 | 신규 `git_diff_staged_text`(git2 native·상한·바이너리/lock 제외) + git_log 최근 20건 summary 컨텍스트 + 신규 `ai_commit_message` + 커밋 입력란 Sparkles 버튼(생성 중 취소 토글) |

## 2. 확정 사실 (메인 재검증 완료 — 근거 실물)

1. **AiProviderClient 는 list_models·complete 2메서드뿐**이고 complete 가 AiInlineCompleteRequest+
   AiPromptTemplate 에 결박된 RPITIT(providers/mod.rs:98-106) — 범용 chat/instruct 진입점 부재. 3 provider
   모두 내부에 chat 폴백 구현 보유(ollama /api/chat·codex responses SSE 누적·omlx /v1/chat/completions).
2. **git 도메인에 unified diff 커맨드가 없다** — git_diff_file 은 DiffSides{original, modified} 파일 전문
   2벌을 크기·바이너리 가드 없이 반환(git/types.rs:112-116). staged 통합 diff 는 신규 커맨드 필수.
3. **AiPromptTemplate 의 fim/chat 은 serde default 없는 필수 필드**(ai/types.rs:46-60)이고 오버라이드
   파싱 실패는 무경고 번들 폴백(prompt.rs:22) — 기존 파일에 필드를 추가하면 사용자 커스터마이즈가
   조용히 초기화된다. 신규 프롬프트는 별도 파일이어야 함.
4. **⌘K 는 monaco chord 21건의 1단계 키**(monaco-actions.ts '⌘K ' 21건) + KeymapEntry.when 은 선언만
   있고 평가기 미구현 + 캡처 훅이 preventDefault/stopPropagation — 앱 레벨 단독 ⌘K 는 chord 전멸.
   VS Code 의 Inline Chat 기본키가 ⌘I 인 이유.
5. **ai 커맨드 6종 전부 원격 dispatch 허용**(ai_set_token 포함, dispatch.rs:806) — 거부 arm 은 remote 3건뿐.
6. **레포 전체에 ViewZone·ContentWidget·OverlayWidget 사용처 0건**(grep 0) — Wave G 가 최초 도입.
   monaco 0.56 d.ts 에 InlineCompletion.isInlineEdit 는 있으나 별도 inline-edit provider API 는 없음.
7. **스트리밍**: ollama·omlx 는 stream:false 고정, codex 만 SSE(단 델타 누적 후 단일 반환) — 토큰 단위
   UX 는 provider 2종 스트림 리더 신설이 필요해 1차 제외가 합리적.
8. ai_inline_complete 취소는 AiInlineStore(requestId→oneshot) + ai_inline_cancel — requestId 기반이라
   구조 자체는 범용. 응답 후처리(코드펜스 제거)는 provider 어디에도 없음. 프롬프트 render 변수는
   {prefix}{suffix}{language}{filePath}(basename 축약) 4종.

## 3. 확정 설계

### 3.1 공통 기반 (Rust)

- **provider trait**: `instruct(client, model, system, user) -> AppResult<Option<String>>` 메서드 추가 —
  기존 complete 무변경(auto-tab 회귀 0). 3 provider 의 기존 내부 chat 구현(complete_chat 류)을 승격해
  구현(codex 는 instructions+input, ollama 는 /api/chat stream:false, omlx 는 /v1/chat/completions).
  마스킹(mask_provider_error)·타임아웃(공용 HTTP 60s) 기존 유지.
- **프롬프트**: `resources/prompts/inline-edit-default.json`·`commit-message-default.json` 신규(각각
  전용 타입 — chat{system, user} 형). prompt.rs 로더를 id 파라미터화 공통화(3파일 = 2회 이상 룰 충족).
  사용자 오버라이드는 `{app_data}/prompts/<id>.json` 동일 규약(전체 대체·파손 시 번들 폴백).
  inline-edit 치환 변수: {selection}{instruction}{language}{filePath}(+선택 주변 컨텍스트
  {prefix}{suffix} — 상한 상수). commit-message 치환 변수: {diff}{recentCommits}.
- **취소 store 공용화**: AiInlineStore → `AiRequestStore` 일반화(requestId 키 구조 그대로),
  `ai_inline_cancel` → **`ai_request_cancel` 리네임**(배선 3곳·프론트 호출처 갱신 — 결정 ② 의 이름
  정확성 기조). 신규 2커맨드도 동일 store 로 취소.
- **설정 리네임**: `ai_auto_tab_provider`→`ai_provider`·`ai_auto_tab_model`→`ai_model` —
  `#[serde(alias = "ai_auto_tab_provider")]` 류 alias 를 Settings·SettingsPatch 양쪽에 부여(구 settings.json·
  구 gist 페이로드 하위호환). sync 리터럴·emptySettingsPatch·settings-view·인라인 완성 게이트 등 소비처
  전량 갱신. `ai_auto_tab_enabled` 는 auto-tab 전용 토글이므로 유지.
- **신규 커맨드 3종**(배선 3곳+파리티, 원격 dispatch 는 기존 ai/git 조회 정책과 동일 — 허용):
  `ai_inline_edit(request{selection, instruction, language, filePath, prefix, suffix, model?, provider?, requestId})`
  → Option<String>(제안 코드) / `ai_commit_message(request{diffText, recentCommits, requestId, ...})` →
  Option<String> / `git_diff_staged_text(projectId)` → §3.3. 앞 2종은 프롬프트 렌더를 Rust 가 수행
  (auto-tab 선례 — 오버라이드 가능성 유지).

### 3.2 Inline Edit (프론트)

- **진입**: monaco 에디터 액션(editor.addAction — code-editor.tsx 기존 addAction 선례) id
  `taide.aiInlineEdit`, 기본 키 ⌘I, precondition editorTextFocus. 팔레트 진입 동반(커맨드 등록).
  ⌘K 채택은 Wave H(chord 엔진) 이후 재검토(§4).
- **입력 위젯**: 선택 영역 상단 ContentWidget(레포 최초 — 검토 집중 항목). 인스트럭션 입력·Return
  제출·Esc 닫기·생성 중 스피너+취소. 선택 없으면 현재 줄을 선택으로 승격.
- **프리뷰(모델 무변경 — 핵심 불변식)**: 응답 수신 후 원본 선택 영역에 삭제 데코(Wave C 충돌 데코
  계열 스타일), 제안 코드는 선택 아래 **ViewZone** 에 렌더(monaco.editor.colorize 로 구문색 — 실패 시
  플레인 텍스트 폴백). **모델은 수락 전까지 절대 불변**(dirty·LSP didChange·hot-exit 미러·git gutter
  부작용 사슬 원천 차단). 수락(⌘Enter/버튼) 시 **1회 executeEdits + pushStackElement 경계**(⌘Z 1회
  원복). 거절(Esc/버튼)은 데코·zone·위젯 폐기만. 진행 중 문서 편집 발생 시 프리뷰 무효화(stale 방지).
- **응답 처리**: 코드펜스 스트립(``` 감지 관대 파서 — 중첩 펜스는 최외곽만), 빈 응답/취소는 토스트.
  요청은 ai_inline_edit 단발 + ai_request_cancel. 스트리밍은 보류(§4).
- **키 계약**: Return 제출 / ⌘Enter 수락 / Esc 거절·닫기. hunk 부분 수락 없음(선택 영역=1 hunk).
- i18n locale 4곳(위젯 라벨·플레이스홀더·수락/거절·실패 토스트·팔레트 라벨).

### 3.3 AI 커밋 메시지

- **Rust `git_diff_staged_text(projectId)`**: git2 native — head_tree↔index `diff_tree_to_index` +
  `Diff::print(DiffFormat::Patch)`(Wave C git2 native 계보 일관). 바이너리 델타 스킵(파일명만 표기)·
  lock 파일 제외(상수 목록: bun.lock·Cargo.lock·package-lock.json·pnpm-lock.yaml·yarn.lock)·전체 바이트
  상한 상수(초과 시 절삭 + 절삭 사실 문자열 명시). 반환 { diffText, truncated, skippedFiles }.
  spawn_blocking 조회(무거운 조회 관행).
- **프론트(git-panel)**: 커밋 입력란 우상단 Sparkles IconButton(icon-button 선례) — 클릭 시
  git_diff_staged_text + 프론트가 이미 보유한 git log 캐시에서 최근 20건 summary 조립 →
  ai_commit_message → 결과를 커밋 입력란에 채움(기존 입력 있으면 대체 전 확인 없이 대체하지 않고
  — 비어있을 때만 즉시, 있으면 확인 토스트 대신 그냥 덮어쓰기 여부는 **비어있지 않으면 덮어쓰기**
  가 VS Code 관행 — 덮어쓰기 채택, undo 는 입력란 텍스트라 위험 낮음). 생성 중 Loader2 + 재클릭=취소
  (ai_request_cancel). staged 0건이면 버튼 비활성. 실패 toast.
- **프롬프트**(commit-message-default.json): diff + 최근 커밋 스타일 모방 지시("참고만, 복사 금지") +
  단일 메시지 출력 강제 + 코드펜스/따옴표 스트립 수신측 처리. Conventional Commits 하드코딩은
  폴백 지시로만(히스토리 추론 우선).
- i18n locale 4곳(버튼 툴팁·생성 중·실패·staged 없음).

### 3.4 실행 구조

- **Phase S(Rust 단독, sonnet+xhigh)**: §3.1 전체 + git_diff_staged_text + locale + 배선 3곳
  (ai_request_cancel 리네임 포함)+파리티 + bindings 재생성 + cargo fmt/clippy/test + 설정 alias
  하위호환 테스트(구 키 역직렬화).
- **Phase B 프론트 병렬 2(sonnet+xhigh, 파일 소유 분리)**: B1 = Inline Edit(위젯·프리뷰·액션·키 —
  features/editor·widgets/editor-pane·shared/lib inline-edit 모듈·관련 테스트) / B2 = 커밋 메시지 UI +
  설정 리네임 프론트 반영(widgets/git-panel·widgets/settings-view·entities 설정/ai ipc·인라인 완성
  게이트 필드명 갱신). bindings 소비 필드명은 S exports 기준.
- **Phase D 통합(단독)**: 팔레트 커맨드·keybinding 카탈로그 노출 확인·문서(features/ai.md 신설 또는
  갱신·features/git.md·ipc-contract·data-model·qa6 Wave G 실기 항목·갭 분석 §5·§8 종결) + 전체
  verify + vite build.
- **Phase E 검토**: 4렌즈(opus+xhigh — 보안 렌즈: 원격 dispatch 노출·diff 유출 표면·프롬프트 주입) →
  적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차 → 커밋(dev).

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| ⌘K 기본키 채택 | 보류 — monaco chord 21건 충돌·when 평가기 부재. Wave H(chord 엔진) 후 재검토 |
| Channel 델타 스트리밍 | 보류 — provider 2종 스트림 리더 신설 필요(codex 만 네이티브). 1차 일괄 |
| diff 탭 프리뷰·임베드 diff editor | 기각 — 제자리 편집 체감 상실 / zone 내 에디터 인스턴스 비용 |
| 모델 선반영(스트리밍 직접 편집) | 기각 — dirty·LSP·미러·undo 부작용 사슬. 모델 무변경 프리뷰 채택 |
| run_git CLI 로 staged diff | 기각 — git2 native 계보(Wave C hunk stage 결정) 일관 |
| 프론트 파일별 git_diff_file 조립 | 기각 — 파일 전문 2벌 토큰·IPC 폭발 |
| AiInlineCompleteRequest 에 diff 우겨넣기 | 기각 — HACK(의미 어긋난 필드 재사용·FIM 오발) |
| 별도 AiInstructClient trait | 기각 — 기존 trait 메서드 추가(complete 무변경)로 충분 |
| auto-tab 템플릿 파일에 섹션 추가 | 기각 — 오버라이드 무경고 파손(§2-3). 별도 파일 채택 |
| 최근 커밋 full body 컨텍스트 커맨드 | 기각(1차) — summary 20건으로 시작, 품질 미달 시 후속 |
| hunk 단위 부분 수락 | 보류 — 선택 영역=1 hunk. 다중 위치 편집 확장 시 재설계 |
| ai_auto_tab_enabled 리네임 | 유지 — auto-tab 전용 토글이라 이름 정확 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 3커맨드+cancel 리네임 반영).
- 4렌즈+적대적 검증+메인 2차 통과. 초점: **프리뷰 모델 무변경 불변식**(수락 전 dirty·미러·LSP 무변화
  실증)·ViewZone/ContentWidget 최초 도입 안정성(스크롤·접기·워드랩)·수락 undo 단일성·설정 리네임
  하위호환(구 settings.json·구 gist 페이로드 alias 역직렬화 테스트)·diff 상한·바이너리/lock 제외
  실효성·ai 신규 커맨드 원격 노출 타당성·프롬프트 주입(diff 내용이 지시를 탈취하는 시나리오)·
  코드펜스 스트립 경계·프리뷰 중 문서 편집 무효화.
- 문서: features/ai.md·features/git.md·ipc-contract(3커맨드+리네임)·data-model(설정 리네임)·qa6
  Wave G 실기 항목·갭 분석 §5(AI 커밋)·§8(Inline Edit) 종결 표기.

---

## 6. Phase E 검토 결함 수정 반영 (2026-08-16)

> 검토 wf_1f2bcdcf-0bb — 4렌즈 발견 52건(major 11·minor 41) → 적대적 검증에서 major **전건
> confirmed**(중복 제거 시 5개 근본 클러스터) → 수정 50 fixed·1 rejected·1 deferred.
> 메인 2차: 핵심 수정 6건 실물 재검증 + 반환 타입 컨벤션 1건 직접 수정 + `bun run verify` 전체
> exit 0(프론트 972·Rust 817+6+17) + vite build exit 0.

- **[major] instruct 가 auto-tab 용 256 토큰 출력 캡을 상속 — Inline Edit 결과 무경고 절단**:
  instruct 전용 예산 분리(OLLAMA_INSTRUCT_NUM_PREDICT=4096 등, complete 경로 256 유지) +
  ollama `done_reason`/omlx `finish_reason` == "length" 를 에러로 전환(잘린 코드가 정상 응답처럼
  수락되는 경로 차단).
- **[major] 팔레트 'AI: 선택 영역 편집' 무동작**: monaco 의 `precondition` 이 run() 자체를
  게이트(팔레트 포커스에서 editorTextFocus=false)함을 소스로 확인 — `keybindingContext` 로 이전해
  ⌘I 키 스코프는 유지하고 팔레트 실행은 허용.
- **[major] 코드펜스 스트립이 후행 설명문 응답에서 실패 — 펜스·산문이 소스에 삽입**: 전문 앵커
  매칭을 첫/마지막 마커 스캔 방식 관대 파서로 재작성(선행/후행 산문·단일 라인·CRLF), ai-commit-message
  의 중복 구현 제거(공용 재사용). 회귀 테스트 7건 동반.
- **[major] truncated·skippedFiles 가 모델에 미전달(계약 §3.3 문언 이탈)**: diffText 본문에 제외
  파일·절삭 안내 문자열 직접 삽입(오버라이드 무관 보장) + **시크릿류 파일(.env·id_rsa·*.pem 등)
  diff 제외 추가**(is_secret_like_path — 보안 렌즈 파생).
- **[major] 제출 직후 입력 hidden 으로 포커스가 body 이탈 — ⌘Enter/Esc 키 계약 불능**: 상태 전환
  시 가시 컨트롤(취소/수락 버튼) 재포커스로 키 계약 복구.
- **minor 41 수정 요지**: AiRequestStore 누수·커밋 메시지 취소/재요청 레이스·ViewZone 높이 불일치·
  utf8 절삭 경계 패닉 위험·프롬프트 체인 치환 재주입(단일 패스 렌더러 교체 — diff/selection 이
  후속 플레이스홀더를 탈취하는 주입 차단)·요청 크기 상한·세션 종료 시 미취소·ipc-contract 수치
  정정·중첩 삼항 등.
- **기각 1**: features 레이어의 entities IPC 직접 호출 — code-editor.tsx 기존 선례와 동일(FSD 합법
  방향), Wave G 신규 이탈 아님.
- **보류 1(사용자 재확인 여지)**: 계약 §3.3 명문대로 staged 0건이면 Sparkles 버튼 비활성 —
  unstaged 폴백 제안은 계약 상충으로 미채택, features/ai.md §9 에 비대칭 사실 기록.
- 메인 2차 부기: `stripCodeFence` 의 자명한 반환 타입 명시 1건 메인 직접 제거(컨벤션).
  `dirty_미러` 테스트 병렬 실행 플레이키 1회 재관측 — Wave G 무관(file/service.rs 미변경)·단독
  5회 연속 통과, 기존 산발 플레이키로 기록.
