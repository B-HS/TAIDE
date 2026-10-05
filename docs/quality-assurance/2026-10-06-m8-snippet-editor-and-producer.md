# M8 Snippets 실제 UI·typed 요청 생성

후속 정본은 `2026-10-06-m8-snippet-consumers.md`입니다. 실제 Host/Workbench/Toast·mutation admission과4개 신규 검사/최종 빌드를 연결했습니다. 아래의 소비자 미연결 문장은 생산자 경계 당시 이력이며 현재 남은 범위는 후속 정본을 따릅니다.

## 대상 파일·계약

- `native/taide-native-ui/src/snippet-editor.rs`, `snippet-edit.rs`, `settings-view.rs`, `settings-controls.rs`, `settings-code-view.rs`, `icons.rs`, `lib.rs`, UI manifest/검사와 native Snippets 아이콘 리소스입니다.
- 원본 Settings 섹션 순서/버튼, `widgets/snippet-editor`, `features/snippet`, Dialog/AlertDialog/OptionPicker·설치된 Lucide/Radix 및 egui SDK를 확인했습니다. backend snippet_actions는 재사용하며 새 라이브러리/버전은 없습니다.

## 구현 결과

1. Editor 다음/Terminal 이전 Snippets 목차와 Manage32/Folder24 버튼, 전체 takeover header·256px sidebar·구조화 name/prefix/body4행/description/global scope·행 추가/삭제·새 파일/파일 삭제/폐기 Dialog입니다. JSON textarea 대체가 아닙니다. stable 행/필드 ID·라벨 연결/pressed·disabled·합성 IME Escape guard를 연결했습니다.
2. 기존 Picker와 실제 Lucide Trash2/Plus/FolderOpen SVG를 재사용합니다. 기존 rasterizer/font/cache는 바꾸지 않았고 설치된 Lucide/Feather LICENSE를 동반합니다. NewFile 모델 값은 유지하되 child Picker/transform은 닫힘 때 회수합니다.
3. typed List/Save/Delete 요청은 owner/Weak editor lifetime/opaque operation identity를 갖습니다. matched pending 응답만 UI에 적용하고 실패 초안 보존·명시 retry·저장 후 목록 요청·성공/실패 Notice를 출력합니다. native execute는 기존 supervised worker/snippet_actions를 호출하지만 실제 HostCommand/AppSurfaces/BrowserWorkbench가 아직 요청을 소비하지 않습니다. 이 단계의 응답은 합성 worker fixture이며 실제 파일 I/O 증거가 아닙니다.
4. 공용 Popup은 닫힘 Presence 중 외부 포커스가 생기면 trigger 복귀를 취소합니다. 완료 시각을 지난 slow frame에서도 motion을 회수하기 전에 외부 포커스를 검사합니다. 원본 Radix nonmodal outside-interaction/close-autofocus 계약을 확인했습니다.

## 검증·실패 구분

- UI 첫 검사1 PASS: 실제 Settings 목차/Manage→List→선택/AX/본문 편집→Save 거절→명시 retry/별도 identity→Saved/List→늦은 응답 무시→미완성 거절→Trash 확인→Back/unmount. 수정 뒤 전체2 실행 build1.73초이며 이 검사 자체는 통과했습니다. 이후 실패 경계만 필터링하고 이 성공은 반복하지 않았습니다.
- UI 두 번째 검사 PASS: 실제 NewFile Picker→global 입력 자동 focus→합성 IME Escape 보호→부모 disabled 확인→create 요청/닫힘. 첫 정상 프레임 guard 수정 후1.54초/.12초, 이후 distinct slow250ms profile RED를 재현하고 `motion.is_some()` guard 수정 후1.09초/.12초·filtered1로 통과했습니다. 정상/slow는 다른 입력이며 동일 성공을 반복하지 않았습니다.
- 기존 Popup Tab/search/option/close Presence/trigger 복귀 영향1 PASS(3.65초/.14초·filtered7)입니다. 첫 guard 수정 시점이며 최종 guard는 기존 owned-focus 경로를 변경하지 않아 재사용합니다.
- native builtin palette 정확한 경로1 PASS(4.07초/.06초·filtered317)입니다. 직전 unqualified exact 명령은0 tests이므로 통과 증거로 세지 않습니다. 기존 Wry17 및 libtest linker eh_frame 크기 경고는 별도입니다.
- native lib/bins/tests check10.49초·최종 normal Canvas Wasm check1.13초 exit0입니다. 실제 Chrome/bindings 갱신 증거가 아니고 현재 bindings는 선행 키바인딩 pointer-hit 소스입니다.

동일 readiness 가정3회 실패 후 재시도하지 않고 SDK를 읽었습니다. Context 종료 후 read_response가 이전 this_pass geometry를 선택해 최신 trace y88와 오래된 interact y2993가 달랐습니다. 검사 전용 current Response.interact_rect/enabled capture로 관찰 경계를 수정했으며 생산 click/scroll을 우회하지 않았습니다. 초기 production WidgetInfo 문자열 이동·fixture Ui.disable/IME active_range_chars·Style.scroll_animation API compiler 실패는 실제 SDK 기준으로 수정했고 suppressions는 없습니다.

## 남은 범위

- 원본 Dialog200ms fade/scale Presence·전체 스타일/반응형 footer·focus 복귀/Tab 그래프/전체 AX/theme/DPI는 미완료이며 UI 단계 전체를 체크하지 않습니다.
- 실제 native/browser 소비자·전역 목록 cache/stale/GC/invalidation·모든 mutation 추적·Toast success/error·owner/remount·Closed/no replay/close drain·실제 Chrome 연속 검증은 미완료입니다. UI만으로 목록/저장 기능이 제품에서 동작한다고 주장하지 않습니다.
- 전체 M8/나머지 Settings/App/assets/최종 gates는 미완료입니다. 실제 CJK/VoiceOver는 사용자 실기·최후 순위입니다.

Snippets1/4·전체363/433(83.83%)·provider2/4·최종0/8·ETA 보류·main 직접·goal active·전체완료 전 Git 없음입니다. OS 설정/보호 앱/실제 프로젝트/Keychain/원격 저장소는 변경하지 않았습니다.
