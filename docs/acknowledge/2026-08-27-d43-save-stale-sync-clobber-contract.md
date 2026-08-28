# d-43 — 저장 직후 구식 syncedContent 되씌움(버퍼 역행·영구 dirty·2차 저장 유실) 수정 계약 (2026-08-27)

> 발견: d-42 재개 실사 ⑤(e2e 재실행)에서 spec 01 이 격리에서도 비결정 실패(5회 중 2회) —
> 파일럿 C2(dirty 점 잔존)의 잔여 절반. d-42 b(응답 역전 가드)와는 다른 축의 신규 결함.
> 재현·증거: 메인 격리 디버그 프로브(scratchpad `debug-01-dirty-dot.ts`, WS 몽키패치 계측).

## 0. 근본 원인 (메인 실증 — 추정 아님, 전송 순서 트레이스로 확정)

- 증거 트레이스(실패 런): `send file_save` → `resp(ok)` → `send file_open`(FILE.CONTENT 재조회)
  → `layout_set_dirty(false)` → **`layout_set_dirty(true)` — 재조회 응답 도착 전에 재더티**.
  직후 버퍼 라인 수 5→4 역행(타이핑한 마커 라인 소실), 백엔드 dirty=true 고정, 디스크는
  저장 내용 유지. 2차 ⌘S 를 누르면 역행한 버퍼가 디스크를 덮어 **타이핑 유실**이 실제로 발생.
- 기전: `editor-pane.tsx` 의 모델 동기 effect(`[editor, syncedContent, dirty, path]`)는
  dirty→false 전이 시 `applyExternalContent(path, syncedContent, editor)` 를 실행하는데,
  `use-editor-file-persistence.ts` 의 저장 성공 핸들러(`handleSave` onSuccess)는
  `setDirty(false)`/`setTabDirty(false)` 만 하고 **`syncedContent` 를 방금 쓴 `finalContent` 로
  갱신하지 않는다**. `syncedContent` 의 불변식은 "마지막으로 알려진 디스크 내용"인데 저장
  성공 순간 그 값은 저장 전 원본으로 구식이 된다 — 신선한 FILE.CONTENT 재조회(modifiedMs
  effect → `setSyncedContent`)가 dirty=false 와 **같은 렌더 배치에 도착하면 통과, 늦으면
  effect 가 구식 원본을 모델에 되씌운다** → `handleChange` → dirty 재설정(영구 잔존: 이후
  재조회는 `dirty` 가드에 걸려 모델을 복구하지 못함).
- 왜 지금 드러났나: 네이티브 IPC 는 sub-ms 라 재조회가 거의 항상 배치에 합류(레이스 승리),
  원격 세션은 WS 왕복 지연으로 자주 패배(격리 5회 중 2회) — e2e 하네스가 원격 경로라 적발.
  원격 제어는 1급 기능이므로 실사용 결함이다(데이터 유실 클래스).

## 1. 범위·수정 방향 (근본 수정)

- `handleSave` onSuccess 의 clean-정착 분기(`draftRef.current === finalContent`)에서
  **`setSyncedContent(finalContent)` 를 함께 수행** — 저장 성공 = 디스크 내용이 finalContent 라는
  사실을 불변식에 즉시 반영. 이로써 dirty→false effect 의 `applyExternalContent` 는 동일 내용
  no-op 이 된다(레이스 창 자체 소멸).
- 동일 클래스 정착 지점 실사: `settleAfterDiskWrite`(git 충돌 해소가 디스크에 쓴 뒤 호출)도
  dirty=false 만 하고 syncedContent 를 갱신하지 않는다 — 같은 창이 열리므로 **쓴 내용을 받아
  `setSyncedContent` 하도록 동일 수정**(시그니처에 내용 전달, 호출자
  `use-editor-git-gutter-and-conflicts.ts` 동반 수정). `handleViewDisk` 는 이미 갱신하고 있어
  대상 아님. 그 외 `setDirty(false)` 지점 전수 grep 으로 누락 확인.
- 범위 외: 원격 dispatch 의 커맨드 병행 처리 순서 보장(별건·필요성 미확증), FILE.CONTENT
  재조회 자체의 버전 가드(이 수정으로 창이 소멸하므로 불요 — 필요해지면 별건).
- 검증: 실패 재현 프로브(수정 전 5회 중 2회 실패) → 수정 후 프로브 반복 전건 통과 +
  spec 01 반복 + verify(TS 전량) + vite build. Rust 무접촉.

## 2. 실행·검토

- 구현 1 에이전트(sonnet+xhigh, TS 단일 파일+호출자) → 검토 1렌즈(opus+xhigh, 근본성·회귀 —
  **축소 근거**: 단일 훅 소형 수정·근본 원인을 메인이 전송 트레이스로 직접 실증·수정 후 프로브/
  e2e 반복 실행 재현 검증이 붙음. 계약 §0 증거로 다렌즈 수렴 대체) → major 시 적대적 →
  메인 2차(프로브·e2e 재실행). 커밋 금지(사용자 지시 — /goal "커밋하지 말고 계속").

## 0.1 근본 원인 정정 — v1 관통 경로 (메인 재계측, 같은 날)

- v1(§1 의 syncedContent 정착) 적용 후에도 프로브 재현 지속 → 페이지 내 일시 계측(모델
  setValue 스택·핸들러 로그, 검증 후 전량 원복)으로 **관통 경로 확정**: onSuccess 배치
  (syncedContent=신값·dirty=false) 커밋 직후의 리렌더에서 `editor-pane.tsx` 의 **렌더 본문
  채택 분기**(`file && !dirty && file.content !== syncedContent → setSyncedContent(file.content)`)
  가 아직 구식인 FILE.CONTENT 쿼리 데이터로 syncedContent 를 즉시 되덮음 → effect 가 그 구식
  값을 `applyExternalContent` → 클로버. 즉 **완전한 근본은 "디스크 쓰기 성공 시 FILE.CONTENT
  캐시가 구식으로 남는 것"** — 렌더 본문은 캐시를 디스크 진실로 신뢰하므로 캐시 자체를 쓴
  내용으로 동기 패치해야 한다(레이아웃 b 수정과 동일 철학). 검토 렌즈(§3)가 정적 분석으로
  동일 결론에 독립 도달(제안 수정 = setQueryData 패치 — v2 와 일치).

## 1.1 수정 v2 (캐시 패치 — 메인 적용, 인터랙티브 소수정 예외)

- `entities/file/file.query.ts` `useSaveFile.onSuccess`: invalidate 전에
  `setQueryData<OpenedFile>(FILE.CONTENT(path), {...existing, content})` — modifiedMs 는 의도적
  비패치(재조회가 정본 mtime 공급, content 패치만으로 채택 분기가 동일-내용 no-op 화).
- `entities/git/git.query.ts` `useResolveGitConflict.onSuccess`: 동일 패치(쓴 content 실보유).
- `entities/app-file/app-file.query.ts` `useWriteAppFile.onSuccess`: `APP_FILE.CONTENT(target)`
  에 쓴 문자열 패치 + invalidate(기존 암시 반환(await 되던 invalidate)을 블록으로 바꾸며 그
  대기 의미가 사라진 것을 캐시 동기 패치가 대체). settings 타깃은 백엔드 sanitize 로 디스크
  본문이 다를 수 있어 **잠정치** — 후속 재조회가 실제 영속 본문으로 재동기(v1 의
  app-file-pane JSDoc 에 단서 명기).
- v1(3지점 syncedContent 정착)은 유지 — 캐시 패치와 같은 커밋 배치를 이루는 이중 안전벨트.
- 메인 직접 적용 사유: 실행 재현 루프(프로브 반복)와 일시 계측이 얽힌 인터랙티브 디버깅 —
  d-39 픽스처 선례의 예외 조건 부합. 원문 계측 트레이스는 세션 스크래치(`full-round.log`).

## 3. 구현·검토·검증 기록

- **구현**: v1 = wf_4696e380 fixer(sonnet+xhigh) — 계약 2지점+전수 실사로 app-file-pane 동일
  클래스 1지점 추가 발견·수정, settleAudit 8건 전수 판정 기록. v2 = 메인(§1.1).
- **검토 1렌즈**(opus+xhigh, wf_5254b252 — v1 시점 대상): **major 1**(d43-r1 — v1 관통·캐시
  패치 필요 = 메인 v2 와 독립 수렴, 적대적 생략 근거)·minor 3·info 2. 판정: r2·r3(app-file
  JSDoc 근거 오류·settings 잠정치) = JSDoc 재작성으로 수용 / r4(마크다운 프리뷰 회귀 창) =
  **v2 로 자동 소멸**(채택 분기가 저장 직후 발화하지 않게 됨) / r5(일시 계측 잔존) = 원복
  완료 / r6(ide-sync-provider 의 외부 저장 경로가 로컬 dirty 를 못 내려 'changed on disk'
  오탐 가능 — 선재 별건) = **이월 기록**(아래).
- **검증**: typecheck·eslint(0 error)·prettier·bun test 1499/0 전부 그린(메인 재실행).
  경험 재현 — 수정 전 프로브 실패 재현(디스크 기록+dot 잔존+버퍼 역행), v2 후 프로브 2/2
  클린(dot 해제) + spec 01 격리 다수 통과. spec 01 의 잔여 플레이크 1건은 별건(하네스 type()
  입력 손상 — S1 계열 재발)으로 판명되어 클립보드 붙여넣기로 스펙 수정(d-39 몫).
- **이월**: r6(외부 `ide_save_requested` 경로의 로컬 dirty/syncedContent 비동기화 — 선재,
  실기 확인 필요). 프로브의 "2차 ⌘S 후 dot 재점등" 관측 1건 — formatOnSave off 진단 환경
  한정 관측이라 실조건 재현 미확정, 기록만.
