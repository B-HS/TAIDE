# d-48 — 설치본 에디터 색상 전무 진단·수정 계약 (2026-08-28)

> 발견: v0.1.0 설치 실기(사용자). dev 정상·prod 전무. 사용자는 LSP(d-47) 연관을 추정했으나
> 코드상 무관 — shiki 토크나이제이션은 LSP 를 소비하지 않는다(별개 결함).

## 0. 진단 현황 (메인 — 원인 미확정·후보 압축)

- shiki 초기화는 단일 경로: plugins 쿼리 로드 → `assemblePluginGrammarRegistrations`(플러그인
  문법 조립) → `initShiki` (`theme-provider.tsx`). 이 체인 어디서 실패해도 **`console.error`
  뿐** — 프론트 에러가 파일 로그로 포워딩되지 않아 prod 에선 침묵 사망(설치본 로그 0 에러와
  정합). 릴리스 빌드는 devtools 부재라 콘솔 확인 불가.
- 배제된 가설: WASM/CSP(shiki 는 `createJavaScriptRegexEngine` — WASM 미사용)·문법 청크 누락
  (dist/assets 에 rust/typescript/tsx 등 실존)·LSP 연관(무관).
- 잔여 후보: ① plugins 쿼리/문법 조립 체인의 prod 전용 실패 ② `@shikijs/langs` 동적 import 의
  임베드 자산 서빙 실패 ③ 그 외 — **런타임 콘솔 증거로 확정한다**.

## 1. 이번 라운드 범위 (사용자 확정: 계측 = 근본 개선 겸용)

- **프론트 에러 → 파일 로그 포워딩 신설**(진단 수단이자 prod 가시성의 근본 개선):
  - JS: `@tauri-apps/plugin-log` 추가(최신), `shared/lib` 에 포워딩 부트스트랩 —
    `console.error`/`console.warn` 후킹(원본 동작 보존·재귀 방지·직렬화 안전) +
    `window 'error'`/`'unhandledrejection'` 청취 → plugin-log error/warn. 앱 진입점 1회 설치.
  - Rust: capabilities 에 log 권한 추가(플러그인은 기등록).
- 이후: 메인이 로컬 `bun run tauri build` 산출 → 사용자 재설치·재현 → 로그로 원인 확정 →
  근본 수정은 이 계약 §4 로 이어서.
- 검증: typecheck/eslint/prettier/bun test/vite build + cargo(capabilities 접촉 시).

## 2. 실행·검토

- 구현 fixer(sonnet+xhigh, TS — Rust측은 d-47 에이전트에 동승) → 통합 1렌즈 → 메인 2차 →
  로컬 build → 사용자 실기.

## 3. 기록 (구현·검토·검증)

- **구현**(wf_7e4547c6 TS fixer): `shared/lib/error-log-forwarding.ts`(+테스트)·main.tsx 설치·
  plugin-log 2.9.0 고정. `log:default` 권한은 기존재(Rust 무수정). 재귀 방지 = 동기 플래그 +
  plugin-log 호출 `.catch(()=>{})` 흡수.
- **렌즈**(wf_ad9ea564): minor 2 — F3 설치가 `@app/app` 모듈 평가 뒤라 모듈 평가 시점 오류
  미포착 → 부수효과 모듈로 main.tsx 1행 승격 / F7 재귀 가드·원본 보존·거부 격리 미테스트 →
  mock.module 실테스트 3건 추가. info 2 기록: F11(포워딩 로그 위치가 포워더 지향 — 대상
  호출부가 태그·스택을 본문에 실어 실피해 제한)·F12(자명 반환 타입 제거 — 반영). 반영 =
  wf_bf67d2be. verifiedOk 요지: 원본 console 선호출·직렬화 폴백 전 분기·FSD 무위반·
  unhandledrejection 루프 원천 차단.
- 잔여: 로컬 `tauri build` → 사용자 재설치·재현 → 로그로 원인 확정(§4).

## 4. 원인 확정·근본 수정 (2026-08-29, 메인 원격 프로브 — d-45 선례 방식)

- **증상 확장(v0.1.1 실기)**: 색상 전무에 더해 LSP 호버 텍스트가 코드 위에 무배치로 겹침
  (위젯 스타일 파괴). d-47(LSP 감지)은 해소 확인.
- **계측 경로**: 설치본 원격 서버가 임베드 번들을 원바이트 서빙(`serving.rs::serve_embedded`
  — `asset_resolver`)함을 이용, Chrome(devtools)으로 동일 번들을 로드(로그인은 사용자 직접
  입력 — 비밀번호 비접촉).
- **확정 원인**: 브라우저에서는 동일 번들이 **정상 착색**(`.mtk22` 적색 실측·문법 청크 전부
  로드) → 번들 무죄. 결정 증거: ① 원격 서빙 HTML 에는 CSP 부재(브라우저 무제약) ② `.mtk`
  토큰 테마 151KB 는 **논스 없는 런타임 주입 `<style>`** ③ 다른 style 요소에 tauri 빌드 시
  논스 실재 — 즉 설치 앱에서는 tauri 커스텀 프로토콜이 CSP 헤더에 style-src 논스를 추가하고,
  **CSP 규격상 논스/해시 존재 시 `'unsafe-inline'` 무효화** → 모나코가 런타임 주입하는 토큰
  테마·위젯 스타일이 앱 웹뷰에서만 전면 차단. dev 는 vite dev 서버 직로드(CSP 미주입)라
  정상 — dev/prod 괴리와 두 증상(무착색·위젯 겹침)을 단일 원인으로 설명.
- **수정**: `tauri.conf.json` `app.security.dangerousDisableAssetCspModification: ["style-src"]`
  — style-src 한정으로 tauri 의 논스 주입을 끄고 명시 `'unsafe-inline'` 을 살림(원 CSP 의도
  복원). script-src 논스 강화는 유지 — 표면 확장 없음. 메인 직접 적용(진단 직결 1라인 설정 —
  소수정 예외 사유 기록).
- **후속 기록**: 원격 접속 페이지는 CSP 헤더가 아예 없음(serve_static 미부착) — 별도 강화
  후보로 기록만(원격은 비밀번호 게이트 뒤·로컬 바인드).
- 검증: 로컬 tauri build → 사용자 재설치 실기(색상·호버) → 통과 시 v0.1.2 릴리스.
- **실기 확인(2026-08-29, 사용자): 색상·호버 정상 — d-48 종결.** d-47·d-48·d-49 설치본
  결함 3건 전부 해소 확정.
