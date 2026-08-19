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
  :153-171 의 remote.login* **7키** — `loginTitle`·`loginPasswordLabel`·`loginSubmit`·
  `loginLocked`·`loginFailed`·`loginLinkExpired`·`loginInsecureNotice`) 확인·유지
  (Phase E 정정: 초판 "5키"는 과소 기재 — T2I-R1/C-02).
- **제거 32키 전량(3언어+MESSAGE_NAMESPACES 동반 제거 — Phase E 에서 git 실측 재확인)**:
  `app.settings`·`app.welcome`·`common.delete`·`common.empty`·`common.open`·`common.rename`·
  `common.reset`·`editor.reloadFromDisk`·`editor.unsavedChanges`·`explorer.renameTitle`·
  `git.commitDetails`·`git.pull`·`git.push`·`git.tags`·`ide.clientConnected`·
  `ide.clientDisconnected`·`ide.diffTabTitle`·`ide.lockfileWriteFailed`·`ide.portUnavailable`·
  `ide.startFailed`·`settings.aiCompletionFailed`·`settings.aiTokenCleared`·
  `settings.aiTokenLabel`·`settings.cliDevBuildUnsupported`·`settings.cliNotOurSymlink`·
  `settings.keymapCustomized`·`settings.themeCredits`·`settings.themeImportButton`·
  `settings.themeImportDialogTitle`·`settings.themeImportFailure`·`tab.moveToSplit`·
  `terminal.newTerminal`. 이 중 `settings.themeCredits` 는 `docs/backlog.md` "테마 크레딧 UI"
  보류 항목이 예약했던 키 — 제거 유지하되 해당 문서에 재추가 필요를 명시(F4).
- **보수 유지 77키(리터럴 미참조) 구성 분해(Phase E 실측 — T2I-C-03)**: 동적 도달 70키
  (keymap.monaco.\*·problems.severity.\*·agent.status.\*·themeEditor.ns.\* 4패턴) + 의도적 예약
  1키(`agent.hooksFileInvalid` — `docs/features/agent-integration.md:241` "소비 UI 는 후속
  웨이브") + **미판정 6키(T2-F 잔여)**: `agent.badgeAriaLabel`·`themeEditor.previewAnsiTitle`·
  `themeEditor.previewChromeTitle`·`themeEditor.previewGitStatusSample`·
  `themeEditor.previewSidebarTitle`·`themeEditor.previewSyntaxTitle`.
- **감사 R5#12 의 역방향 검사**(코드 참조 키 ⊆ 카탈로그 — 미참조·누락 키 CI 검출) 도입 여부는
  **미결** — 다음 locale/T2-F 배치에서 결정한다.
- **MESSAGE_NAMESPACES 코드 유지** — required·en⊆required 파리티 기존 테스트 전량 무수정
  그린. 신규 테스트 2(OnceLock 캐시 ptr_eq·언어별 대표 값+interpolation+전각 스팟).
- **검증**: verify exit 0(cargo 1052+17+6+3·bun 1375)·vite build exit 0·bindings 무변경.

## 5. Phase E 검토 반영 (2026-08-20)

> 4렌즈+적대적 검증: confirmed 2(F1·D1 — 둘 다 minor 강등)·refuted 0. 나머지는 기록·문서
> minor. 이 절의 수정으로 §4 의 초판 기재 오류(5키·목록 미기재)는 위에서 직접 정정했다.

- **F1/D1 — panic 최초 접촉을 부팅 경로로**: `warm_builtin_catalogs()` 신설(locale/service.rs),
  lib.rs setup(창 생성 전)에서 en/ko/ja 3 카탈로그 eager 강제 초기화. 지연(lazy) 상태에서는
  파싱 panic 이 async 커맨드의 spawn 태스크만 죽여 invoke 프라미스가 미정착 → reveal 게이트에
  걸려 "보이지 않는 창"이 되므로, 부팅 시 loud crash 로 끌어올렸다. panic 채택 근거(번들
  데이터 파싱 실패는 프로그래머 오류 — 빈 폴백은 required 계약을 조용히 깨므로 theme `.ok()`
  스킵·lsp manifest 빈 목록 폴백과 의도적으로 다른 선택)는 `parse_builtin_messages`·
  `warm_builtin_catalogs` rustdoc(영어)으로 코드에 명시(D1).
- **T2I-R3 — panic 선택 기계 고정**: `#[should_panic(expected = "is not valid JSON")]` 테스트
  (`손상된_내장_카탈로그는_패닉한다`) 추가 — 리팩토링이 조용히 폴백으로 되돌리면 실패한다.
- **F2 — 중복 키 가드**: serde 는 JSON 중복 키를 last-wins 로 조용히 통과시키고 키 집합 기반
  파리티 테스트는 이를 못 잡으므로, 원문 텍스트 기반 테스트
  (`내장_카탈로그_원문에_중복_키가_없다`) 추가. 방식: 원문에서 `    "` 접두 라인 수(top-level
  엔트리 — JSON 문자열은 개행을 못 담아 1키 1라인이 구조적으로 보장되고, prettier 4스페이스
  형식은 verify 의 `prettier --check` 가 상시 강제)를 파싱된 맵 len 과 대조. 중복이면 라인 수
  > len 으로 실패, 형식이 바뀌면 라인 수 부족으로 시끄럽게 실패(오탐이 안전 방향).
- **F4 — settings.themeCredits**: 제거 유지(미래 예약 키를 카탈로그에 남기지 않음 — dead code
  원칙). `docs/backlog.md` 테마 크레딧 UI 항목에 "T2-I 에서 제거됨·구현 시 재추가 필요" 명시.
- **F3/D2 — 문서 정합**: HANDOFF.md:106 4곳 동기 서술을 실위치(MESSAGE_NAMESPACES+3 JSON)로
  갱신, vsix-theme-import.md 의 위치·키 서술을 실소비 6키 기준으로 정정,
  ipc-contract.md locale 절에 내장 카탈로그 정본 경로·동기 지점 3줄 추가.
- **이월(이번 배치 수정 금지 판정)**: F5 경량 summaries 헬퍼(별도 배치 무방 — 검토 자체 판정)·
  D4 BUILTIN_LOCALE_SOURCES 테이블화(다음 locale 작업 시).
