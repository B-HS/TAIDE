# 배치 12 — 편집기 미니맵 (2026-10-09)

상태: 실제 원본 기준값·모델·렌더·직렬 입력·설정 저장 연결과 자동 통합 게이트를 마쳤습니다. 구현 `1a2aafe9`의 선별 커밋·일반 푸시와 로컬/원격 차이 0/0을 확인했습니다. 서브에이전트·workflow 없이 메인이 직접 수행합니다. 전체 기능 완료율·잔여 시간은 최신 전수 대응표와 실행 시간 근거가 없어 미산정입니다.

## 원본과 경계

- `code-editor.tsx:246`의 기존 `taide.toggleMinimap`은 `onMinimapToggle`을 호출합니다. 실제 파일/untitled/app-file-pane의 핸들러는 editorMinimap 설정 패치를 저장합니다. sticky scroll의 세션 토글과 수명이 다르므로 기존 설정 저장 경로에 연결합니다. 기본 enabled true이며 대형 파일에서는 표시만 끕니다.
- Monaco 0.56.0 `editorOptions.js`의 기본값은 right·proportional·mouseover slider·renderCharacters true·maxColumn 120·scale 1·autohide none입니다. TS는 enabled만 지정하므로 fill/fit·블록 렌더·사용자 축척 같은 새 옵션은 추가하지 않습니다. proportional은 sampling을 사용하지 않으며 실제 표시 줄(접기/wrap 적용)을 렌더합니다.
- 실제 `_computeMinimapLayout` 폭 식은 남은 본문 폭·반각 문자 폭·DPR·세로 스크롤바·2px 캐럿 여유·원본 미니맵 gutter를 사용합니다. DPR 2 이상은 원본 scale 2, 그 아래는 1입니다. `MinimapLayout.create`는 stable slider 높이·전체가 맞는 경우/움직이는 줄 창·부분 줄 정렬·스크롤 방향의 이전 layout 수명을 계산합니다.
- 문자 시트는 원본 `minimapPreBaked.js`의 scale 1/2를 사용합니다. 원본 `MinimapCharRenderer`의 soften·색 혼합·light 배경과 UTF-16 문자·탭/전각 처리를 확인했습니다. 현재 기본 proportional 경로에서 원본의 두 prebaked 축척만 필요합니다. 매 줄 본문 galley를 축소하는 방식은 확정 설계에서 기각됐습니다.
- 본문 클릭은 해당 표시 줄을 가운데로 reveal하며 선택을 바꾸지 않습니다. 슬라이더 drag는 최초 scroll/ratio에서 delta를 계산합니다. touch는 슬라이더 중심을 이동합니다. hover/active 슬라이더·100ms opacity·canvas 0.9 opacity와 가로 내용이 넘칠 때 6px shadow를 원본 CSS에서 확인했습니다.
- TS 테마는 minimapSlider 3색을 scrollbar.thumb/thumbHover로 지정합니다. 배경은 기본 editor 배경, selection은 editorSelectionBackground이고 나머지 공급되지 않은 minimap 장식은 실제 LSP/검색/SCM 장식 공급 범위에서 연결합니다. Inline/LineBackground/Lane 장식을 임의로 미니맵 표시로 변환하지 않습니다.
- native 기존 DisplayMap의 segment/row_start_column/hidden_lines/wrap_settings와 토큰 generation·문서 revision·뷰별 InputState·ScrollState를 사용합니다. egui 0.36.2의 실제 load_texture/TextureHandle.set/ColorImage API를 읽었습니다. 새 의존성·remote-web 기능/의존 그래프/manifest/lockfile 변경은 하지 않습니다.

## 검증과 남은 범위

원본 layout·문자 renderer를 실행한 기준값과 실제 egui 메모리 화면/입력으로 검증합니다. 변경 크레이트 전체 대상은 --no-fail-fast로 한 번 직접 실행하고 실패 영향만 재검사합니다. Cargo는 하나씩 실행합니다. 보호 Trash 3건·실제 앱 데이터/OS 입력·IME/RTL/접근성·대형 실기·soak와 ignored 성능 부채는 통과로 집계하지 않습니다.

`docs/utils/2026-10-09-native-minimap-reference.js`는 실제 Monaco 0.56.0 layout·문자 renderer를 실행해 36폭·2592 layout·808문자 표본과 원본 문자 시트 192/768바이트를 추출합니다. 원본 클래스 경계를 임시 모듈에 그대로 추출하며 원본 파일·의존성은 변경하지 않습니다. 최종 생성 exit 0, pure layout 3건 exit 0입니다. 초기 data URL 모듈 호출과 종료하지 않은 원본 타이머 시도는 성공 근거에서 제외합니다.

메모리 UI 최초 클릭은 press_origin 누락으로 scroll 0, 이를 고친 뒤 keyboard focus 해제 실패를 관찰했습니다. SDK에 해당 위젯만 opt-in하는 포커스 보존 선언을 추가했습니다. 이전 pass의 실제 hit-test·clip·레이어·enabled 상태와 기존 focusable 소유자를 확인하며 소유자가 없으면 새 포커스를 만들지 않습니다. 클릭 순서에 따라 다른 입력창의 소유자를 따라갑니다. 두 SDK 회귀가 통과했고, touch start와 raw 이벤트의 실제 허용 trigger도 같은 경계를 사용합니다. 기본 SDK/브라우저에는 이 선언을 공급하지 않습니다.

클릭 뒤 Text가 최종 캐럿을 드러내지 않고 scroll 1890에 남는 실패를 재현한 뒤 raw 이벤트 순서를 따라 마지막 입력을 적용했습니다. Source alpha 127 토큰이 [104,104,104]로 그려지는 실패를 원본 [67,67,67] 기준으로 수정했습니다. GPU 최대 1024에서 144×1920 텍스처가 거절되는 실패도 재현해 원본 픽셀/좌표를 보존하는 타일 렌더로 수정했습니다. 처음 GPU 128 harness는 SDK font atlas의 최소 1024보다 작아 미니맵 재현으로 집계하지 않습니다.

수정 후 메모리 UI 12건 exit 0입니다(`/private/tmp/taide-batch12-input-and-render-after-20261009.log`, build 3.05초). 폭 예약·원본 문자/토큰·스타일 교체·캐시/선택·클릭/drag/touch·다른 입력창·클릭/Text 양방향 순서·alpha·GPU 제한을 확인했습니다. 앱 설정 저장의 queued toggle 두 번은 fresh 설정을 직렬 수신부에서 읽어 제자리로 돌아오며 `settings-controls` 2건 exit 0입니다. 고유 임시 앱 경로만 사용했습니다.

추가 DPR/Unicode/접기/wrap/옵션 수명/읽기 전용/대형/휠을 포함한 메모리 UI 16건이 전체 UI 대상에서 통과했습니다. 새 harness의 존재하지 않는 store 메서드와 folding false·잘못된 접기 byte 범위는 실제 store API/설정/범위로 수정했습니다. 원본 slider CSS 대조에서 touch active 색 누락을 추가 재현한 뒤 고쳤고 해당 1건이 다시 통과했습니다. 전체 UI 성공 결과는 재사용하고 이 실패 영향만 재검사했습니다.

## 최종 통합 게이트

빌드·테스트 Cargo는 직렬로 실행했습니다. SDK 문서 검사 중 두 패키지의 읽기 전용 Cargo fmt 확인을 병렬 실행한 한 차례는 한 Cargo 프로세스 규칙과 달랐습니다. 이후 Cargo 명령은 모두 하나씩 실행합니다. 보호 범위와 테스트 결과를 바꾸지 않았습니다.

| 대상 | 실제 결과 | 로그 |
| --- | --- | --- |
| UI 전체 기본 18대상 | exit 0, 312 통과·0 실패, build 8.51초. minimap 16·pure 3·기존 토글 명령 회귀 포함 | `/private/tmp/taide-batch12-ui-full-20261009.log` |
| UI touch active 수정 영향 | 수정 전 exit 101·1 실패, 수정 후 exit 0·1 통과. 전체 기본 검사 후 추가한 색 검증의 영향만 재검사 | `/private/tmp/taide-batch12-touch-active-before-20261009.log`·`taide-batch12-touch-active-after-20261009.log` |
| UI inspection 전용 추가 1대상 | `--features inspection --test snippet-editor --no-fail-fast` exit 0, 18 통과·0 실패, build 4.13초. 기본 18대상과 합쳐 서로 다른 330건·19대상의 성공 근거 | `/private/tmp/taide-batch12-ui-inspection-20261009.log` |
| 앱 전체 67대상 | exit 0, 612 통과·0 실패·보호 Trash 3 filtered out, build 41.77초 | `/private/tmp/taide-batch12-app-full-20261009.log` |
| vendored egui 전체 | UI manifest에서 `-p egui --no-fail-fast` 실행 exit 0, 단위 52·문서 167 통과·문서 1 ignored, build 1.72초·문서 74.56초 | `/private/tmp/taide-batch12-egui-full-20261009.log` |
| remote-web host | `cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 3.98초 | `/private/tmp/taide-batch12-browser-host-20261009.log` |
| remote-web Wasm | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --features canvas --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 2.68초 | `/private/tmp/taide-batch12-browser-wasm-20261009.log` |

변경 두 패키지 Cargo fmt·SDK 새 포커스 정책/회귀 구간 포맷·pass_state rustfmt·generator 프로젝트 Prettier·이번 소스/문서 diff 검사가 완료됐습니다. 패키지/SDK state/JS/diff 검사 exit 0이며 context.rs의 무관한 기존 구간 전체 재포맷은 하지 않았습니다. 브라우저 전체·모든 manifest/lockfile 기준 `56431f69` 대비 diff exit 0입니다. 디스크 여유 671GiB·사용률 64%, 캐시 정리는 실행하지 않았습니다. 변경 없는 editor 192·syntax 158과 기존 ignored 성능 4건은 이전 배치 근거를 재사용합니다. 후속 LSP/진단/검색/SCM 미니맵 장식·리거처·전체 전환 범위와 실제 화면/OS 입력 게이트를 보존합니다.

앱 보호 제외는 `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`, `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`, `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다`이며 성공으로 집계하지 않습니다. 앱 전체는 기존 loopback·watcher·ImageIO 테스트 경계를 위해 승인된 샌드박스 밖에서 실행했습니다.

이번 소스·회귀·원본 자산/고지·QA·완료 근거와 PROCESS 상단 절 26파일만 선별 커밋했습니다. 기존 HANDOFF·architecture·합의/운영 문서와 PROCESS 하단 변경은 포함하지 않았습니다. 전체 전환을 배치 13 괄호 일치 강조로 계속 진행합니다. 기본 미니맵의 모델·표시·입력과 기존 14옵션 소비 14/14는 전체 기능·실기 완료율이 아닙니다.
