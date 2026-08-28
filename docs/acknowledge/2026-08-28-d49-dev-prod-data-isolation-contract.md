# d-49 — dev·설치본 데이터 디렉토리 분리 계약 (2026-08-28)

> 발견: v0.1.0 설치 실기(사용자) — "dev 설정이 설치본에도 적용". 원인 = 동일 identifier
> (`dev.taide.app`)로 app_data_dir·로그·keyring·window-state 전부 공유(tauri 표준 동작).

## 0. 결정 (사용자 확정: 추천 A)

- **dev 전용 identifier `dev.taide.app.dev` 로 분리.** 설치본(prod)은 기존
  `dev.taide.app` 유지 — 현 데이터가 사용자 실사용 데이터로 승계된다. dev 는 분리 후 첫
  실행에서 신규(빈) 환경으로 시작한다(필요 시 사용자가 수동 복사 — 안내만, 자동 이관 없음).

## 1. 수정 방향

- `src-tauri/tauri.dev.conf.json` 오버레이 신설(`{ "identifier": "dev.taide.app.dev" }`).
- **사용자 습관(`bun run tauri dev`) 보존**: package.json 의 `tauri` 스크립트를 소형 래퍼
  (`scripts/tauri.ts` 등)로 바꿔 **`dev` 서브커맨드일 때만** `--config src-tauri/tauri.dev.conf.json`
  을 주입한다. `tauri build` 등 다른 서브커맨드는 무가공 통과(prod identifier 불변).
- 파생 정리(dev 를 바라보는 참조): `e2e/lib/paths.ts` 픽스처 캐시 루트
  `~/Library/Caches/dev.taide.app/e2e-fixtures` → `dev.taide.app.dev`(e2e 는 dev 인스턴스
  대상 — JSDoc 사유 갱신 포함). 레포 전수 grep 으로 dev 참조/prod 참조를 구분해 dev 몫만 갱신.
  문서(debugging.md 로그 경로 등)는 dev/prod 경로가 갈라졌음을 명기.
- 범위 외: 자동 데이터 이관·prod identifier 변경·Windows/Linux 경로 검증(미배포).
- 검증: typecheck/eslint/prettier/bun test + e2e 경로 상수 테스트(있으면). 실기: 사용자
  `bun run tauri dev` 재시작 → 신규 환경 확인 + 설치본과 상호 불간섭.

## 2. 실행·검토

- 구현 fixer(sonnet+xhigh, TS/설정) → 통합 1렌즈 → 메인 2차 → 사용자 실기.

## 3. 기록 (구현·검토·검증)

- **구현**(wf_7e4547c6 TS fixer): `tauri.dev.conf.json`(identifier 단일 키)·기존
  `scripts/tauri.ts` 래퍼에 dev 분기 추가(빌드 서브커맨드의 사이드카·번들 config 선행 로직
  무손상 — 렌즈가 git show 대조로 확증)·e2e 상수 3종(`E2E_FIXTURE_PROJECTS_DIR`·
  `REMOTE_LOG_PATH`·`MACOS_APP_SETTINGS_PATH`) 갱신·살아있는 문서 4건 갱신.
- **렌즈**(wf_ad9ea564): major 1 — F2 keyring `SECRET_SERVICE="dev.taide.app"` 하드코딩으로
  keyring 미분리(계약 §0 위반, dev 에서 토큰 삭제 시 설치본 자격증명 소실 위험) →
  identifier 파생 주입으로 수정(prod 는 identifier 불변이라 기존 키체인 무손실 —
  wf_bf67d2be). minor 2 — F6 HANDOFF dev 경로 2곳 낡음(메인 반영)·F8 포트 발견이 dev·설치본
  동시 실행을 미구분 → .app 경로 후보 제외(wf_bf67d2be). F4(버전 0.1.1 드리프트 —
  Cargo.toml 승격+CI 가드 확장)도 이 라운드에 동승.
- 실기: 사용자 `bun run tauri dev` 재시작 → dev 가 빈 환경(신규 identifier)으로 시작하는지 +
  설치본과 상호 불간섭 확인 대기. dev keyring 도 분리되므로 dev 에서 AI 토큰·원격 비밀번호는
  재입력 필요(설치본 것은 그대로).
