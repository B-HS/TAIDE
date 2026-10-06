# 전환 배치 6 통합 검증 (2026-10-07)

## 범위

배치 6은 플러그인·VSIX 문법(설계 3e), 편집기 장식·앵커 기반(단계 5), 접기(단계 7)입니다. 단계별 상세는 `2026-10-06-native-batch6-{plugin-grammars,decorations-anchors,folding}.md`입니다.

| 단계 | 결과 |
| --- | --- |
| 플러그인·VSIX 문법 | 리뷰 pass. 플러그인 문법을 읽어 번들 문법과 함께 레지스트리를 구성. id·scope 충돌 우선순위는 Shiki 등록 순서를 옮김. 잘못된 문법은 해당 언어만 평문 |
| 장식·앵커 기반 | 리뷰 pass. 장식 모델(줄 배경, 인라인 배경·전경, 직선·물결 밑줄, lane 표식)과 저널 기반 범위 이동, 층 렌더, Monaco 식 gutter 폭, 좌표 질의, 오버레이 배치기 |
| 접기 | 구현 작업자가 컨텍스트 한도를 넘겨 중단 → 이어받기 workflow로 완성. 리뷰 차단 1건(Monaco 포팅 고지 누락) 수정. TS 접기 명령 19개 중 13개 실행 가능 |

## 화면이 바뀌는 변경

- gutter 폭이 Monaco 식(`round(max(자릿수, 3) × 숫자 최대 폭) + 줄 장식 10 + 접기 16`)으로 바뀌어 모든 편집기의 본문·줄 번호 x가 달라집니다(글꼴 14 기준 본문 시작 41.28 → 35.00, 접기 영역 포함 시 +16). 동결된 브라우저 클라이언트도 표면을 공유해 함께 바뀝니다.

## 메인이 직접 실행한 검증

| 검사 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --no-fail-fast` (전체 대상) | 608 통과, 1 실패(인수 이전부터의 XLML 테스트) |
| `cargo test --manifest-path native/taide-native-syntax/Cargo.toml` | 89 통과, 3 ignored |
| `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | 124 통과, 1 ignored |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection --no-fail-fast` | 241 통과 |
| `cargo test --manifest-path native/taide-remote-web/Cargo.toml --features inspection --no-fail-fast` | 53 통과 |
| `cargo fmt -- --check` (app, ui, editor, syntax) | 전부 exit 0 |
| `bunx prettier --check THIRD_PARTY_LICENSES.md` | 통과 |
| 접기 단계 중단 직후 작업 트리 | editor(`--tests`)·ui·app `cargo check` 오류 0 |

## 남은 차이와 부채

- Monaco가 기본으로 처리하던 접기 단축키(⌥⌘[, ⌥⌘], ⌘K ⌘0, ⌘K ⌘J 등)가 키로 동작하지 않습니다. 명령 팔레트와 사용자 지정 키로는 실행됩니다. 편집기 문맥의 Monaco 기본 키 전반을 native 키맵에 옮기는 작업이 필요합니다.
- 접기 표식(region marker), offSide 규칙, LSP folding range, 접힘 상태 영속화는 없습니다.
- 접기 영역 계산이 문서 revision이 바뀔 때마다 표면에서 동기 실행됩니다.
- 문법 파일 내용만 바뀐 플러그인 재로드는 감지하지 못합니다(목록 비교만).
- include 사슬이 매우 깊은 플러그인 문법은 엔진의 스택을 넘칠 수 있습니다. include만으로 순환하는 문법은 싣기 전에 걸러냅니다.
- 문법이 없는 플러그인 언어는 저장 정리가 후행 공백을 지우지 않습니다(TS는 토큰 공급자가 없으면 모든 줄을 지움).
- 물결 밑줄은 Monaco 구조대로 본문보다 먼저 그립니다(작업 지시의 순서 표기와 다름).
- `EditorAppearance.horizontal_padding`은 더 이상 배치에 쓰이지 않지만 literal 소비처 때문에 필드는 남아 있습니다.
- clippy `too_many_arguments` 3건(`reveal_tokenized`, `show_presented`, `show_tokenized`)은 소비처를 `show_request`로 옮기면 없앨 수 있습니다.

## 화면 확인

- [ ] 구문 강조, gutter 폭 변화, 접기 컨트롤, 플러그인 문법 — 검증 시점에도 화면이 잠겨 있어 캡처하지 못했습니다. 캡처용 샘플 프로젝트와 세션 데이터는 스크래치패드에 준비돼 있습니다.

## 디스크

여유 721GB(사용률 61%).
