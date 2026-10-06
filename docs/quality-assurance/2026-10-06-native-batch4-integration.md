# 전환 배치 4 통합 검증 (2026-10-06)

## 범위

배치 4는 편집기 표시 계층입니다(설계 문서 `docs/research/2026-10-06-native-editor-display-layer-design.md` 7절의 단계 1·2·4). 단계별 상세는 `2026-10-06-native-batch4-{display-skeleton,editor-fonts,word-wrap}.md`입니다. 골격과 글꼴 단계는 리뷰 판정 pass, word wrap 단계는 차단 항목 2건을 수정했습니다.

| 단계 | 내용 |
| --- | --- |
| 표시 계층 골격 | 화면과 입력 결과를 바꾸지 않고 편집기 내부를 줄 매핑(`DisplayMap`), 세로 배치(`VerticalLayout`), 줄 텍스트(`RowText`), 좌표, 페인트 층으로 분리. 리팩터링 전에 고정한 특성화 테스트 4건과 기존 13건이 기대값 수정 없이 통과 |
| 글꼴 체인 | `editorFontFamily` 적용, TS와 같은 폴백 순서(사용자 글꼴 → 시스템 monospace → SFMono → Menlo → Apple SD Gothic Neo), 굵은 face 패밀리 등록, 글리프의 줄 상자 가운데 정렬, 줄 높이를 Monaco와 같은 `round(1.5 × 글꼴 크기)`로 보정 |
| word wrap | Monaco 줄바꿈 계산 이식, 표시 줄 매핑과 증분 갱신, 탭 정지 표시, 표시 줄 기준 이동·Home·End·Page, wrap 상태의 줄 번호·선택·클릭·IME |

리뷰가 word wrap에서 잡은 차단 항목: wrap 상태에서 현재 줄 강조가 캐럿의 표시 줄 하나에만 그려지던 것(Monaco는 문서 줄 전체), wrap 설정·폭·글꼴이 바뀔 때 첫 보이는 줄 위치를 유지하지 않던 것. 둘 다 재현 테스트와 함께 수정됐습니다.

## 메인이 직접 실행한 검증

| 검사 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | 전 대상 통과(실패 0) |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --test editor_surface --test workbench` | 116 / 33 / 12 통과 |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` | 352 통과, 0 실패 |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --no-fail-fast` (전체 66개 대상) | 588 통과, 1 실패(아래 XLML) |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection` (전체) | 15개 대상 통과 |
| `cargo test --manifest-path native/taide-remote-web/Cargo.toml --features inspection` (전체) | 18개 대상 통과 |
| `cargo fmt -- --check` (app, ui, editor) | exit 0 |

wrap 계산 시간은 구현자 측정값입니다(release 빌드, 합성 문서): 5만 줄 전체 6.5ms, 한 글자 편집 뒤 갱신 0.12ms, 30만 줄 전체 31ms.

## 배치 3에서 놓친 회귀와 수정

- 증상: `tests/terminal-host.rs`의 `headless_input_budget은_포화에서_선택을_유지하고_취소된_대기를_전송하지_않는다`가 항상 실패했습니다.
- 원인: 배치 3이 터미널 `view.error`의 화면 도색을 TS 기준대로 없애고 경고 로그로 바꿨는데, 이 테스트는 화면에 그려진 "budget exceeded" 문구를 검사했습니다.
- 놓친 이유: 배치 3 통합 검증에서 메인이 lib 테스트와 일부 대상만 실행하고 `terminal-host` 전체를 실행하지 않았습니다.
- 수정: 테스트가 현재 보고 경로(경고 로그, 같은 스레드의 기록만)를 검사하도록 바꿨습니다. 묶음 52건 통과.
- 이후 규칙: 배치 통합 검증에서는 lib 테스트뿐 아니라 변경한 크레이트의 전체 테스트 대상을 `--no-fail-fast`로 1회 실행합니다.

## 남은 실패와 부채

| 항목 | 상태 |
| --- | --- |
| `tests/preview-spreadsheet-xlml.rs`의 `xlml은_할당전_grid_합산_인덱스와_xml_보안_경계를_거절한다` | `<Row ss:Index="0">` 입력이 거절되지 않아 실패. 관련 소스와 테스트는 체크포인트 커밋(`4005731`) 이후 바뀌지 않았으므로 인수 이전부터의 실패입니다. 미수정 |
| `tests/explorer-delete.rs`의 `실제_탐색기_삭제는_…` | 전체 실행 중 1회 실패(12.36초), 단독 재실행과 두 번째 전체 실행에서는 통과. 부하에 민감한 간헐 실패 |
| `show_presented` 인자 8개로 clippy `too_many_arguments` 경고 1건 증가 | 설계 서명 유지 여부 결정 필요 |
| `EditorDisplayOptions` 14개 필드 중 word wrap 외에는 표면이 아직 읽지 않음 | 각 옵션 구현 단계에서 연결 |
| 30만 줄 문서의 wrap 전체 계산 31ms, 프레임 분할 미구현 | 대형 파일을 wrap 상태로 처음 열 때와 창 폭을 끄는 동안 프레임 지연 가능 |
| 탭 모드에서 설정의 탭 크기 대신 4가 탭 표시와 wrap 계산에 쓰임 | 기존 한계. `EditorAppearance` 서명 변경 필요 |
| 동결된 브라우저 클라이언트 화면 변화 | 표면을 공유하므로 글리프 세로 정렬, 줄 높이 반올림, 열 중간 탭 폭이 함께 바뀜 |

## 화면 확인

- [ ] 한글 폴백 글꼴과 기준선, 편집기 글꼴 변경의 즉시 적용, wrap 화면의 나눔 위치·이어지는 줄 들여쓰기·줄 번호·현재 줄 강조, wrap 전환 시 맨 위 줄 유지 — 프로젝트와 파일을 열어야 볼 수 있어 캡처하지 못했습니다. 합성 입력은 쓰지 않았습니다.

## 디스크

전체 테스트 대상 빌드 뒤 여유 공간이 70GB에서 22GB로 줄었습니다(사용률 99%). `experiments/native-shell-spike/target`과 루트 `target/`이 대부분을 차지합니다. `experiments/native-shell-spike/target` 안에는 사용자 실기용 `TAIDE M8 Egui Spike.app` 번들이 있어 메인이 임의로 정리하지 않았습니다.

## 중단 상태

사용자 지시(2026-10-06 저녁)로 배치 4까지만 진행하고 멈춥니다. 다음은 배치 5(구문 강조, 설계 단계 3a~3e)이며 엔진·의존성·문법 자산 결정은 `docs/acknowledge/2026-10-06-native-transition-decisions.md`에 기록돼 있습니다.
