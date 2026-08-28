# ADR-0006 — Git 백엔드: git2(읽기·stage·discard) + git CLI(commit·push·pull) 하이브리드

- 상태: 승인 (2026-08-06)
- 관련: `docs/PRD.md` FR-F, `docs/features/git.md`, `docs/research/git2.md`

## 맥락

VSCode 수준 Git 기능(status/diff/blame/log/stage/commit/push)이 필요하다.
후보: git2(libgit2 바인딩), gitoxide(gix), git CLI 파싱.

## 결정

1. **주 엔진은 `git2 = 0.21`** (`features = ["ssh", "https", "vendored-libgit2"]`,
   Linux 배포 시 `vendored-openssl` 추가).
   - 읽기 전부: status, diff(gutter 포함), blame, revwalk(log/graph), refs, ahead/behind
   - 워킹트리/인덱스 조작: stage/unstage(`reset_default`), discard(`checkout_* + force()`)
2. **사용자 git 설정·hook 을 존중해야 하는 쓰기 작업은 `git` CLI 서브프로세스로 수행한다**:
   commit(pre-commit·commit-msg hook), push/pull/fetch(pre-push hook, credential helper·SSH 전체 호환),
   서명 커밋(gpg/ssh), LFS 가 얽히는 작업.
3. 접근 계층은 **`trait GitBackend`** 로 한 겹 추상화한다(`infra/repo.rs`) —
   gix 가 push·worktree blame 을 완성하면 백엔드 교체 가능하게.

## 근거

- gix 0.86 은 **push 미구현**(공식 crate-status 원문 확인), blame 의 워크트리 미지원 —
  현시점 탈락 (`docs/research/git2.md` §대안 평가).
- libgit2 는 **hook 을 실행하지 않는다.** 사용자의 실제 레포들은 commit hook(가드 훅 등)을 쓰므로
  libgit2 커밋은 조용히 hook 을 건너뛰는 실사용 사고가 된다. VSCode 자체도 git CLI 를 호출한다 —
  쓰기 경로의 CLI 위임이 참조 제품과 같은 선택.
- 읽기 경로는 git2 가 구조화된 데이터(비트플래그, hunk 콜백, blame_buffer)를 주어
  CLI 출력 파싱보다 훨씬 정확·저렴하다.

## 스레드·소유 모델 (research 확정안)

- `Repository` 는 `Send + !Sync` → `Mutex<Option<Repository>>` 로 보유, 짧은 작업만 락 사용.
- 느린 작업(blame·log·대형 diff)은 경로만 넘겨 `spawn_blocking` 안에서 `Repository::open` 새로 연다
  (open 은 저렴). blame 은 `(path, HEAD oid)` 캐시 + debounce 필수.

## 결과

- CLI 경로는 git 미설치 환경에서 실패할 수 있다 → 기동 시 `git --version` 감지, 미설치면
  commit/push UI 에 명확한 안내(읽기 기능은 git2 로 계속 동작).
- CLI 출력은 exit code + stderr 를 구조화해 에러 UI 로 전달한다(파싱 최소화).
- git2 쪽 필수 함정(`force()`, `Index::write()`, `Result` 문자열 접근자, foreach 콜백 형태)은
  `docs/research/git2.md` 함정 절을 구현 체크리스트로 사용한다.

## 구현 노트 (2026-08-28 현행화)

- `trait GitBackend` / `infra/repo.rs` 한 겹 추상화는 **만들지 않았다** — git2 호출은
  `domain/git/service.rs` 의 자유 함수들이 직접 수행한다(테스트는 실제 임시 리포로 커버).
  gix 전환 필요가 실제로 생기면 그때 경계를 세운다(조기 추상화 회피 — 2회 이상 규칙).
