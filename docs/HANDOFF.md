# HANDOFF — 2026-08-29 세션 스냅샷 (v0.1.2 첫 공개 릴리스 완주 / 잔여 = draft 공개·qa6·Phase 8 잔재)

> 최종 갱신: 2026-08-29 / HEAD = `59015ee`(main=dev·origin 동기·워킹트리에는 이 핸드오프
> 문서 갱신분만 미커밋). 이 세션 커밋 22건 — d-45·46 종결, 앱 아이콘, 릴리스 파이프라인
> 6회 실전(실패 3회 전부 근본 수정), docs 전면 정합(감사 52건+정본 4종 신설), 설치 실기
> 결함 d-47~49 종결, identifier `net.gumyo.taide` 개명, **v0.1.2 draft 완주**.
> 이 문서가 세션 인수인계 단일 진입점. 직전 스냅샷은 `git show 18bc09f:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 의 에이전트 친화 데스크톱 IDE. 상태는 Rust 소유
(ADR-0004)·view 표시 전용. macOS(arm64) 우선. 도메인 `taide.gumyo.net`·identifier `net.gumyo.taide`.

## 2. 현재 목표

- 최종: PRD FR-A~J(완료) → 전문 QA(e2e 완주 판정 완료·qa6 실기 진행 중) → Phase 8 배포.
- 현재 마일스톤: **v0.1.2 draft 릴리스 완주**(서명·공증·자립성·릴리스 노트 전부 정상 —
  설치 실기 결함 3건 종결본). **다음 한 줄 = 사용자: GitHub Releases 에서 draft v0.1.2
  Publish → qa6 실기 계속(결함은 d-50~ 계약 파이프라인).**

## 3. 완료 / 진행 중 / 미착수

### 3.1 이 세션 커밋 22건 (메시지가 1차 요약, 계약 §가 정본)

| 묶음 | 커밋 | 내용 |
|------|------|------|
| d-45·46 종결 | `9e4a822`·`f3d92ce`·`9e03469` | 피커 프리징(3층+v2)·C/C++ 테마 2종 — 실기 통과 후 3분할 |
| 앱 아이콘 | `1978e91`·`7a6696a` | Prompt Spark(시안 4종 중 B 확정) SVG 정본+전 세트 50파일. dev 도크 미반영 근본 = `generate_context!` 컴파일 타임 임베드×cargo 미추적 → build.rs `rerun-if-changed=icons` |
| 태그 4 금지 | `0c4a082`·`eccae01` | 릴리스 태그·버전에 숫자 4 절대 금지(가드+정본 — 재론 금지) |
| docs 전면 정합 | `020d523` | 감사 wf 52건 반영(32파일)+정본 4종 신설(deployment·debugging·agent-operations·README) |
| 릴리스 수리 1 | `c00a3a3`·`bebb857`·`dc18253` | collision/dead_code 근본(crate-type 제거·cfg 게이트)·base64 내성·시크릿 로컬 선검증 스크립트 2종 |
| 릴리스 수리 2·3 | `7e22fbc`·`cf4bff2`·`79352de` | 번들 경로 루트 target 정정·rust-cache 루트화·구식 락 제거 / git2 vendored-openssl·xz2 static 정적화 |
| d-47~49+병렬 CI | `7fc8a23`·`ffeb198`·`06f2f05` | GUI PATH(fix-path-env rev 핀)·에러 로그 포워딩·dev 분리(identifier·keyring 파생) / 4-job 병렬·Cargo.toml 가드·SHA256 정정 |
| d-48 근본+개명 | `5bdbf7c`·`d8fd5b6`·`f0d6782`·`5e67d85`·`59015ee` | CSP style-src 변조 비활성(훙크 분리 스테이징)·identifier `net.gumyo.taide`+v0.1.2·릴리스 노트 의무화·docs |

### 3.2 릴리스 이력 (정본: `deployment.md` §9·§8 트러블슈팅)

- v0.1.0(4런: base64→경로→dylib→완주)·v0.1.1(d-47 확인·4-job 첫 실전 10분) 전부 폐기,
  **v0.1.2 완주(런 33190360486, 9m35s)** — draft 공개만 남음. 로컬 데이터는
  `net.gumyo.taide` 로 이관 완료(2.1M), dev 디렉토리는 미생성이었음.

### 3.3 잔여

- **즉시**: ① draft v0.1.2 **Publish**(사용자) ② qa6 실기 계속(마스터:
  `quality-assurance/2026-08-11-qa6-checklist.md`, 결함은 d-50~) ③ dev 재실기 — `bun run
  tauri dev` 가 빈 환경(`net.gumyo.taide.dev`)으로 뜨는지·키체인 재입력(설치본도 AI 토큰·
  원격 비밀번호·PAT 재입력 필요 — identifier 개명으로 구 키체인 항목 잔존).
- Phase 8 잔재: 업데이터 / Windows·Linux 빌드 검증 / OSC 133·hooks 브리지(roadmap §Phase 8).
- 도메인 서빙: `taide.gumyo.net` 프록시 구성(사용자 요청 시 — `deployment.md` §7 준비물 정본).
- 이월(사용자 결정 대기 — 전부 기록 완료): d-40 §5 4건 / d-42 §5 2건(L1-03·C6) / d-43 r6 /
  d-44 F4 / d-45 F-06 / C8 후속(dev 전용) / src 4곳 "36 bundled" 감사 수치(재감사 후에만) /
  d-48 §4 후속(원격 페이지 CSP 부재 — 기록만) / 렌즈 info 기록(F9 cfg 가드·F10 fix() 동기
  지연 — 기동 지연 체감 시 재론·F11 로그 위치·F14 release 프로필 테스트 부재) /
  기존: LspInstallProgress 한국어·placeholder↔with_arg 파리티·FILE.RAW predicate·화이트리스트
  입도·fuzzy 가중치(`git show ccb1c05:docs/HANDOFF.md` §3.3).

## 4. 의사결정 요약 (상세·기각 사유는 각 정본)

- **채택**: 아이콘 B Prompt Spark / 태그·버전 숫자 4 절대 금지 / 릴리스는 "배포 후 확인"
  루프(로컬 설치 생략 가능) / d-47 fix-path-env(git rev 핀 `c4c45d5`) / d-48 계측 =
  에러 로그 포워딩(근본 개선 겸용) → CSP 근본(`dangerousDisableAssetCspModification
  ["style-src"]`) / d-49 identifier 분리+keyring identifier 파생 / CI 4-job 병렬+debug
  테스트(release 프로필 검증은 tauri build 컴파일이 담당 — deployment §4 명기) / identifier
  `net.gumyo.taide` 개명(도메인 정합·공개 전 마지막 시점) / 릴리스 노트 파일 의무화 게이트.
- **기각(재론 금지 — 근거는 각 문서)**: crates 분리·가드 테스트(`acknowledge/2026-08-28-
  no-crate-split-decision.md` — 재론 조건: 도메인 코드 2번째 소비자) / 이전 세션 기각 목록
  유지: C1 os.tmpdir·realpath / d-44 GIT.PROJECT 전폭·per-path·DIFF_PATH / d-45 useRef
  지연초기화·훅 코얼레서 / C8·L1-03·C6 보류 / nord 재등재·taide-light #6611d4·abyss 재선정·
  어드바이저리 blocking 승격 / e2e 단언 약화.
- **오판·교정(이 세션)**: 공증 검증 스크립트 1차본의 `@env:` 문법이 notarytool 미지원 →
  오탐 401(사용자 값은 정상이었음) — 직접 전달+트림으로 수정. 이 머신(macOS 26) base64 는
  관대해 CI 재현 불가 → 플랫폼 무관 형식 검사로 강화(스크립트 정본).

## 5. 사용자 방향성 & 작업 규칙

- 역할 5단·위임 Workflow 만·Rust 한 시점 한 에이전트·보고 불신(메인 실물 재검증)·계약(d-##)
  파이프라인·기각 재론 금지 — **정본 = `docs/agent-operations.md`**(이 세션에서 신설).
- 커밋: 대화 내 명시 지시가 auto-commit 합의보다 우선(feedback 2026-08-27 정본)·논리 단위
  분할(필요 시 훙크 분리 스테이징 — 5bdbf7c 선례)·한 커밋분씩 선별 스테이징·dev→main
  ff 병합→양 브랜치 푸시 관례. git add -A·stash·force push·AI 트레일러 금지.
- 릴리스: 절차·게이트 정본 = `deployment.md`(태그 4 금지·버전 3파일 동기·노트 파일 필수).
  태그 재발행은 미공개 draft 한정+사용자 승인. 실기로 확인 안 된 것 "동작한다" 단언 금지.
- 진단: 재현·계측 먼저. 실증 기법 정본 = `debugging.md` §4(WS 몽키패치·프레임 갭·레지스트리
  소스 실물·대조 환경 격리) + **이 세션 신규 선례: 원격 페이지 프로브**(임베드 번들을
  브라우저 devtools 로 — 비밀번호는 사용자가 직접 입력, 에이전트 비접촉. d-48 §4 정본).
- 코드·커뮤니케이션: HANDOFF 구 §5 그대로 유지(arrow fn·주석 금지·React Compiler 전제·
  한국어 존댓말·표·번호 객관식 한 번에·이모지 금지). 시크릿 비접촉(키체인 이관도 안 함).
- e2e: 단언 약화 금지·앱 실행은 사용자만(`bun run e2e` 는 에이전트 가능)·픽스처
  `~/Library/Caches/net.gumyo.taide.dev/e2e-fixtures`(`e2e/lib/paths.ts` JSDoc 정본)·포트
  발견은 설치본(.app) pid 제외 가드 있음.
- cargo PATH: `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"`

## 6. 미해결 질문 / 사용자 확인 필요

1. **draft v0.1.2 Publish** (본문·산출물 검토 후 수동 공개).
2. v0.1.2 설치본 상세 실기 미보고 — 프로젝트 목록 승계·키체인 재입력 여부(사용자가 "커밋"
   지시로 진행을 확정했으나 개별 확인 항목은 미청취. 이상 있으면 d-50 으로).
3. dev 환경 재실기(§3.3 ③) — d-49 분리 이후 `bun run tauri dev` 미실행 상태.
4. qa6 계속 → Phase 8 잔재(업데이터·타 플랫폼) 착수 판단 / 이월 목록(§3.3) 처리 여부.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64)·bun 1.3.14·tao 0.35.3/wry 0.55.1·tauri 2.11.5·React 19(Compiler). cargo PATH §5 |
| 버전·식별 | **v0.1.2**(tauri.conf·Cargo.toml·package.json 3파일 동기 — CI 가드 강제)·identifier prod `net.gumyo.taide` / dev `net.gumyo.taide.dev`(오버레이 `tauri.dev.conf.json`, `scripts/tauri.ts` 래퍼가 dev 에만 주입) |
| git | HEAD=`59015ee`(main=dev·origin 동기). 태그 `v0.1.2`(v0.1.0·v0.1.1 은 폐기·삭제) |
| 기준선(2026-08-29 메인 실측) | bun **1532**/0·cargo workspace **1130**/0·typecheck/eslint/prettier/vite build 클린·커맨드 **178**+3raw·번들 테마 **38**·로케일 918×3 |
| 실행·검증 | dev=`bun run tauri dev`(**사용자만** — 분리된 빈 환경)·`bun run verify`+`bunx vite build`·로컬 릴리스 빌드 산출은 루트 `target/release/bundle/` |
| 로그 | 설치본 `~/Library/Logs/net.gumyo.taide/TAIDE.log`·dev `~/Library/Logs/net.gumyo.taide.dev/TAIDE.log`(UTC·40KB 회전). **프론트 에러 파일 포워딩 동작**(d-48 계측 — `shared/lib/error-log-forwarding*`) |
| 사용자 실기 환경 | 설치본 v0.1.2(색상·호버·LSP 정상 확인)·REMOTE password_only ON(키체인 재입력 전까지 미구성일 수 있음)·테마 monokai-dimmed |
| CI | release.yml 4-job 병렬(정본 deployment §4)·secrets 5종 유효(로컬 선검증 스크립트 2종: `scripts/verify-signing-cert.sh`·`verify-notary-credentials.sh`) |
| 장애 재시도 | `subagents/workflows/wf_*/journal.jsonl` 실사. 이 세션 wf: 82dd41b0(docs 감사)·7e4547c6(d47-49 구현)·ad9ea564(렌즈)·bf67d2be(반영)·0bb0f027(개명) |

## 8. 다음 세션 TODO (우선순위)

1. draft v0.1.2 공개 여부 확인(§6-1) + 설치본·dev 실기 잔여 확인(§6-2·3).
2. qa6 실기 확증 계속 — 발견 결함은 계약 신설(d-50~) 후 구현 wf→렌즈→메인 2차 파이프라인.
3. Phase 8 잔재 판단: 업데이터(tauri-plugin-updater — secrets 재론 필요)·Windows/Linux 검증·
   OSC 133. / (선택) `taide.gumyo.net` 프록시 구성(deployment §7).
4. (선택) 이월 결정(§3.3) / PROCESS.md 아카이브(300줄 초과 상태 — `docs/history/` 이동 후보).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/README.md` | 문서 지도 — 정본/시점기록 분류·진입점 |
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 큐 체크리스트(d-45~49·릴리스 6런 전 과정 등재) |
| `docs/deployment.md` | 배포 정본 — 절차·4-job CI·태그 4 금지·도메인 준비물(§7)·트러블슈팅(§8)·이력(§9) |
| `docs/debugging.md` | 진단 정본 — 로그(신 identifier)·HMR 함정·실증 계측 기법·결함 클래스 |
| `docs/agent-operations.md` | AI 세션 운용 정본 — 역할 5단·계약 파이프라인·커밋 규칙 |
| `docs/release-notes/v0.1.2.md` | 릴리스 본문 정본(표본 — 노트 파일은 릴리스 게이트) |
| `docs/acknowledge/2026-08-28-d45~d49-*-contract.md` | 결함 계약 5건(각 §3 구현·§4 후속이 정본) |
| `docs/acknowledge/2026-08-29-bundle-identifier-rename.md` | identifier 개명 결정·이관 절차 |
| `docs/acknowledge/2026-08-28-no-crate-split-decision.md` | 크레이트 분리 기각(재론 조건 포함) |
| `docs/acknowledge/2026-08-28-release-tag-no-digit-4.md` | 태그 숫자 4 금지 절대 규칙 |
| `docs/acknowledge/2026-08-28-app-icon-prompt-spark.md` | 아이콘 정본(icon.svg)·재생성 파이프라인 |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | 실기 QA 마스터(진행 중) |
| `docs/quality-assurance/2026-08-18-e2e-harness.md` | e2e 사용법(13스펙·신 경로 반영) |
| `docs/feedback/2026-08-27-commit-despite-no-commit-directive.md` | 커밋 지시 우선순위 교정 정본 |
| `scripts/verify-signing-cert.sh`·`verify-notary-credentials.sh` | 서명·공증 시크릿 로컬 선검증 |
| `docs/architecture.md`·`data-model.md`·`ipc-contract.md`·`theme-system.md`·`tech-stack.md`·`roadmap.md` | 구조·데이터·IPC·테마·버전·순서 정본(2026-08-28 전면 정합 완료) |

## 10. 복기 신뢰도

- **높음**: 커밋 22건 git 고정·계약 5건과 릴리스 이력이 실시간 동기·기준 수치 전부 이 세션
  메인 실측·워킹트리 클린 상태에서 스냅샷 작성.
- **중간**: v0.1.2 설치 실기의 개별 항목(프로젝트 목록 승계·키체인 재입력)은 사용자 총괄
  확인("커밋" 지시)만 있고 항목별 청취 없음(§6-2) / dev 분리 환경은 실기 미실행(§6-3).
- **낮음**: 없음.
