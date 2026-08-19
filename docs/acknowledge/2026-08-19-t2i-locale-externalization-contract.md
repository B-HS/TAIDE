# T2-I — 로케일 데이터 외부화 계약 (2026-08-19)

> 정본: 감사 `2026-08-18-architecture-audit.md` §6.3 T2-I("`locale/service.rs` 3,760줄 리터럴을
> `resources/locales/*.json` + `include_str!` 로 — theme 도메인이 이미 쓰는 형태", R5#8·R5#12)
> + §5 T2-F(미참조 로케일 키 39개 — R5#12 겹침).
> 사용자 상시 지시("전부 추천대로 계속 진행해봐") 하 T1 트랙 완결 후 T2 착수 — T2 중 T2-I 를
> 첫 배치로 선정(위험 최저: 검증된 theme 패턴 재사용 / 효과 최대: 최대 비대 파일 4,081줄 해소
> — T2-B 의 한 축 자연 해소 / T2-F 미참조 키 정리 동반). 직전 산출물 T1-H 는 main 병합 완료
> (main=e554c9f).
> 착수 전 메인 실물 확인: locale/service.rs 4,081줄 — `en_messages`(:905)·`ko_messages`(:1852)·
> `ja_messages`(:2796) 리터럴 페어 3벌(~2,850줄)이 몸통, `MESSAGE_NAMESPACES`(:12~893)가
> required key 계약, 로딩·해석 로직(:3761~)은 소규모. theme 선례 실재(theme/service.rs:13-17
> `include_str!("../../../resources/themes/*.json")`)·`src-tauri/resources/` 디렉토리 실재.

## 1. 범위

### 1.1 메시지 카탈로그 외부화

- `en_messages`/`ko_messages`/`ja_messages` 리터럴을 `src-tauri/resources/locales/{en,ko,ja}.json`
  으로 이동, `include_str!` + 1회 파싱(OnceLock 계열)으로 로드. theme 선례와 동일 형태.
- **동작 무변경이 절대 조건**: `builtin_en/ko/ja` 의 반환값(키·값 전체)이 이동 전과 완전 동일
  — 3언어 전수 파리티를 기계 검증(이동 전 스냅샷 대비 테스트 또는 구현 중 자가 대조 스크립트,
  방식은 구현 판단·근거 기록). JSON 이스케이프·유니코드 왕복 정확성 포함.
- `MESSAGE_NAMESPACES`(required key 계약)는 **코드에 유지** — en 카탈로그에서 유도하면
  en⊆required 검증이 순환(자기 참조)이 되므로 계약과 데이터를 분리 유지. 기존 required·
  en⊆required 파리티 테스트 전량 유지(외부화 후에도 기계 강제 지속).
- JSON 파싱 실패는 빌드가 못 잡으므로 테스트로 고정(3팩 파싱 성공·키 집합·값 스팟).

### 1.2 미참조 키 정리 (R5#12·T2-F 겹침)

- 감사 "미참조 로케일 키 39개"는 압축분 — 구현이 **실측**(프론트 t()/키 사용 전수 grep 대비
  카탈로그 키 목록)으로 재확정 후, 미참조 확정분만 3언어+MESSAGE_NAMESPACES 에서 동반 제거.
  동적 키 조합(템플릿 문자열로 키를 만드는 패턴)이 있으면 보수적으로 유지·보고.

### 1.3 범위 외

- locale 로딩·해석 로직 리팩토링(외부화에 필요한 최소 변경만). 사용자 커스텀 로케일 경로
  (`save_locale`/`load_locale`) 의미 무변경. 커맨드 표면·이벤트·원격 정책·와이어 무변경.
- T2-E/T2-J(AppError taxonomy — 별도 캠페인)·T2 나머지 묶음.

## 2. 실행 구조

- **Rust 단독 1 에이전트(sonnet+xhigh)**: §1.1~§1.2 + 전수 파리티 자가 검증 + cargo 3종·
  bindings 무변경 확인. git stash 금지.
- **Phase E 검토(별도 Workflow, 배치 특성 재구성 4렌즈 opus+xhigh)**: 이동충실성(3언어 키·값
  전수 대조 — 유실·오염·이스케이프 왜곡 0) / 계약(required·en⊆required·4곳 동기 테스트 의미
  보존·표면 무변경) / 정확성(1회 파싱 로딩 경로·파싱 실패 시 동작·OnceLock 경합) / 설계(JSON
  스키마 형태·미참조 키 판정의 보수성) → 적대적(opus+high, major 이상) → confirmed 수정 →
  메인 2차 → 커밋(dev).

## 3. 완료 조건

- `bun run verify` + vite build 그린. bindings 무변경. locale/service.rs 대폭 축소(리터럴
  ~2,850줄 소거). 3언어 전수 파리티 검증 기록. 미참조 키 제거 목록·보수 유지 목록 기록.
- 실기 이월(qa6): 3언어 전환 UI 표시 정상·커스텀 로케일 로드 무회귀.

---

## 4. 구현 완료 기록 (2026-08-20, Phase E 검토 전)

> 구현 wf_0a11c367-f88(Rust 단독 sonnet+xhigh) 완료. 메인 2차: 스팟 실물 일치(1,242줄·3 JSON
> 778키 등치·OnceLock+include_str!+panic 처리) + verify 메인 직접 재실행.

- **외부화**: `resources/locales/{en,ko,ja}.json` 평면 키-값(카탈로그가 원래 flat key —
  i18next keySeparator:false 정합). service.rs 는 함수 내부 OnceLock 1회 파싱, `builtin_*`
  반환 의미 동일(clone). **4,081 → 1,242줄**(−2,888).
- **파싱 실패 처리 — theme 선례와 의도적 차이(검토 대상 자기 표기)**: theme 은 `.ok()` 조용
  스킵(Option 반환)이나 locale 의 builtin 시그니처는 infallible — 빈 맵 폴백은 required
  계약을 조용히 깨므로 명확한 panic 채택·테스트 고정.
- **전수 파리티 기계 증명**: 임시 스냅샷 테스트로 실행 결과 기반 3팩 직렬화 — 이동 전후
  diff 0(810키×3언어 바이트 동일, 전각·백슬래시 왕복 포함) → 제거 후 "before−32키" 기대본과
  dict 등치 PASS. 임시 파일 삭제 완료.
- **미참조 키 실측(감사 39 → 실측 32 제거)**: 810→778키. 동적 키 조합 네임스페이스 4개
  (keymap·problems·agent·themeEditor) 보수 전체 유지. Rust 측 직접 참조(remote/login_page.rs
  의 remote.login* 5키) 확인·유지. 제거 32키 목록은 구현 보고 원문(§보고) 기준.
- **MESSAGE_NAMESPACES 코드 유지** — required·en⊆required 파리티 기존 테스트 전량 무수정
  그린. 신규 테스트 2(OnceLock 캐시 ptr_eq·언어별 대표 값+interpolation+전각 스팟).
- **검증**: verify exit 0(cargo 1052+17+6+3·bun 1375)·vite build exit 0·bindings 무변경.
