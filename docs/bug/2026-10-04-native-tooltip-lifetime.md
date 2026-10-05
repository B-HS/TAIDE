# native 공용 tooltip 지연·입력·소유권

## 2026-10-05 기본 키 취소·선행 Focus

소비된 Enter/Space가 clicked=false인 실제 Response의 Tooltip까지 닫았습니다. begin_frame이 raw key만 보고 취소 전에 close/Space arm을 적용한 것이 원인입니다. owner/open generation/pass-index의 close 후보와 Trigger 시점의 남은 normalized key를 연결했습니다. 등록 전 fallback도 취소된 press를 배제합니다.

후속 AX Focus2 뒤 owner1의 draw-time memory focus로 다시 여는 RED는 등록된 선행 replay focus를 보존해 수정했습니다. 고유3 성공과 영향 검사의 정확한 실패/재사용 범위·19.43초 strict 결과는 `../quality-assurance/2026-10-05-m8-tooltip-default-key-cancel.md`가 정본입니다. 기존 Escape fixture의 렌더 후 tooltip ID 카운터 오류도 렌더 전 등록으로 정정했으며 임시 진단은 제거했습니다.

disabled span의 기본 키 오닫힘도 실제 wrap_button RED 뒤 click sense/활성 child alias로 구분해 수정했습니다. disabled/enabled×Enter/Space 고유1과 기존 Theme icon 영향1 PASS·최종 strict18.05초입니다. 생산 코드 Space action·늦은 App 소비·동적 전환 graph/GUI는 미완료이며 전체 M8 완료로 주장하지 않습니다.

## 대상·근거

`native/taide-native-app/src/tooltips.rs`, `tooltips-tests.rs`, `problems.rs`, `problems-tests.rs`, `status-ide.rs`, `application.rs`, `lib.rs`입니다. 실제 제품 `src/app/providers/app-providers.tsx`의400ms와 설치된 Radix Tooltip의 skip300ms·hoverable content·Focus/Blur/Click·PointerDownOutside 계약, pinned egui0.36.2 Tooltip/Popup/Area/Response/InputState 공식 소스를 대조했습니다. wrapper의 기본0ms나 egui 기본500ms를 제품 지연으로 채택하지 않았습니다.

## 관찰·해결

egui 기본 hover tooltip은 키보드 focus에 열리지 않습니다. Problems 실제 AX Focus 검사에서 텍스트가 없는 RED를 확인한 뒤 source 지연/입력 정책을 공용 Provider로 분리했습니다. NativeApplication이 provider를 소유하고 Problems·IDE 상태바가 clone을 공유하며 viewport별 독립 상태를 사용합니다. 매 패스 한 번의 준비, pending deadline의 repaint 예약, 렌더 후 사라진 widget 회수, 제거된 viewport와 다른 Context의 동일 viewport ID 초기화를 연결했습니다.

첫 수정 뒤에도 새 Popup의 텍스트는 첫 pass에 없었습니다. egui Popup이 첫 Area를 sizing_pass로 invisible 처리하고 repaint를 예약하는 공식 소스가 원인입니다. 실제 AX focus를 먼저 확인하고 같은 합성 시각의 다음 렌더에서 텍스트를 검사하도록 fixture를 정정했습니다.400ms를 기다리거나 tooltip을 always-open으로 바꾸지 않았습니다. 새/재마운트 widget의 hit-test 역시 실제 이전 layout pass가 필요해 등록용 프레임을 추가했으며, 이 fixture 실패는 제품 지연 회귀로 기록하지 않습니다.

짝없는 pointer release를 click으로 오인한 RED는 `Response.clicked_by(Primary)`의 실제 mouse click으로 수정했습니다. tooltip content 내부 down까지 outside down으로 취급한 RED는 실제 content rect로 구분했습니다. Space의 짝없는 release RED는 widget별 armed 상태로 수정했습니다. fixture가 임의로 보낸 repeat=true는 egui InputState가 실제 keys_down으로 다시 계산하므로, 실제 press→held repeat→release 순서를 사용합니다.

같은 프레임 AX Focus(error)→Enter→Focus(warning)에서 Enter가 최종 focus의 warning에 오배정되는 RED도 확인했습니다. focus 사건이 있으면 직전 focus에서 출발해 raw 사건 순서로 focus/Enter/Space/AX Click을 해석하고, 나중의 focus 진입이 앞선 click 폐기를 대체하도록 수정했습니다. 기존 Problems 활성화·HostBridge 액션 순서는 바꾸지 않았습니다.

mousedown 뒤 다른 trigger로 focus를 옮겼다가 돌아오면 아직 누른 상태인데도 다시 열리는 RED([true,false]/기대[false,false])를 확인했습니다. 현재 프레임의 down 사건만 보던 원인이며, widget별 pointer-down ref를 실제 up 또는 InputState의 해제까지 유지하도록 수정했습니다. 누른 동안 focus 왕복은 열리지 않고 해제 뒤 새 keyboard focus는 다시 열리는 actual Response 검사1 PASS입니다.

hoverable content는 원본5px padded exit/target rect convex hull을 사용하고, trigger↔content 이동은 유지하며 영역 밖 이동·외부 down은 닫습니다. 파생된 geometry의 WorkOS MIT 고지를 `native/taide-native-app/LICENSE-RADIX-TOOLTIP`에 보존했습니다.

같은 pass의 `begin_frame` 호출은 준비된 snapshot을 먼저 확인해 raw Event/Text 전체를 소비자마다 복사하지 않습니다. 사건 처리 정책을 바꾸지 않고 cache guard만 clone 앞에 두었으며 마지막 관련 lib clippy `-D warnings` exit0(5.25초)입니다. RSS/성능 전체 gate를 통과한 것으로 세지 않습니다.

## 공용 renderer·접근성·disabled 회수 후속

원본 공용 TooltipContent의 색상/글꼴/Frame을 Provider::show로 이동해 Problems·IDE status·글꼴 icon/button8곳·system usage button이 같은 owner와 renderer를 사용합니다. 실제 AppSurfaces가 provider를 명시 전달하며 Context.data에 강한 Provider를 넣는 순환 소유권을 만들지 않습니다. 이 범위에 `status-editor.rs`, `system-usage-view.rs`, `system-usage-view-tests.rs`가 포함됩니다.

Role::Tooltip이 없는 actual AX RED 뒤 실제 area ID에 이름/bounds를 연결하고 trigger described_by를 열린 pass에만 추가했습니다. 중간 구현은 Area::end의 move response를 tooltip으로 오인해 role2개가 되었으며 area ID 하나로 정정했습니다. IDE span은 children 생성 전 unique UI ID의 GenericContainer를 사용해 자식/설명이 실제 같은 노드에 연결됩니다. Tooltip의 Frame container→Label.value 구조는 실제 노드 출력과 공식 source로 확인했습니다. 직접 Label.label 자식을 가정한 fixture 실패는 제품 tree 결함으로 세지 않습니다.

새 status 통합 검사에서 disabled 뒤 열린 tooltip이 남는 RED를 재현했습니다. egui Response.contains_pointer는 disabled에서도 true일 수 있으며 기본 for_enabled를 공용 renderer로 교체한 뒤 이 gate가 빠졌습니다. 공용 is_open에서 disabled widget의 열린 상태·deadline·입력/cache를 회수합니다. dark/light 각각9곳 actual Response의 role/설명·TOP·테마·disabled 즉시 회수·pending 취소·re-enable 무재생성 검사1 PASS입니다. 첫 paint의 Area fade-in alpha21은 별도 fixture 안정 시각으로 분리했으며 원본 motion parity로 주장하지 않습니다.

고유7검사 PASS·최종 app lib/tests strict24.17초입니다. 실제 명령/실패/fixture 정정/성공 재사용은 native-problems QA의 공용 renderer 후속 절이 정본입니다. prior 기본 수명10검사는 반복하지 않습니다.

## 탐색기4버튼·controlled validation 후속

실제 explorer toolbar는 기본 on_hover_text를 사용해 source BOTTOM·공용 provider focus·AX가 빠져 있었습니다. 실제4버튼의 Tooltip role0 RED 뒤 AppSurfaces의 공용 owner와 현재 Appearance를 연결했습니다. theme 초기 구성과 두 갱신 경로를 함께 연결했고 actual dark/light4버튼·AX 설명·BOTTOM·Escape1 PASS와 기존 toolbar 액션 영향1 PASS입니다.

create/rename 오류 입력의 always_open은 기본 theme·Role::Tooltip 누락·aria-invalid 누락과 취소 프레임에 남은 draft 오류를 표시할 수 있었습니다. 실제 Tooltip role 없음 RED 뒤 paint/AX를 Appearance::show_controlled로 분리해 일반 Provider renderer와 재사용했습니다. Explorer가 사건 처리 후 현재 input ID/owner의 오류 metadata만 전달하므로 오류 해제/취소 같은 패스의 stale popup을 제거합니다. 실제 dark/light×create/rename·invalid/설명·이탈 유지·오류 해제·Escape1 PASS입니다.

원본 Radix의 controlled prop 변경은 Provider onOpen/onClose를 자동 호출하지 않습니다. validation을 일반 is_open/deadline 상태로 강제 등록하지 않으며 전체 close-attempt/provider graph는 미완료로 유지합니다. 정확한 실패·fixture 정정·명령·성공 재사용은 native-problems QA의 탐색기 후속 절이 정본입니다.

## IconButton span의 직접 자식·disabled 입력 후속

theme reset/bold/italic·Settings duplicate/edit의 source는 HTML title이 아닌 RadixTooltip span입니다. 실제 role 누락 RED와 중첩 크기 UI의 직접 자식 RED 뒤 wrap_button metadata·enabled child focus alias·disabled span focus/hover를 연결했습니다. Button 자체 min_size24로 불필요한 접근성 컨테이너를 제거했고 span에만 설명, 직접 Button에 이름/disabled/클릭을 남겼습니다. actual dark/light×3icon×disabled2 신규1 PASS·hex blur/reset/Settings create 영향 각1 PASS·strict16.59초입니다. 이전 미완료 문구는 당시 상태이며 이 후속이 해당 소비자의 정본입니다. 명령과 실제 실패는 native-problems QA의 IconButton 후속 절을 참조합니다. preview/전체 GUI/혼합 입력은 미완료입니다.

## PDF·HWP direct Button App owner 후속

PDF4/HWP2는 원본 Button 직접 BOTTOM trigger인데 native builtin hover에서 App owner/AX가 빠졌습니다. 실제 Tooltip role 누락 RED 뒤 live Response를 내부 Output으로 전달하고 동일 App Provider/현재 theme로 렌더했습니다. disabled button은 wrapper처럼 열지 않고 화면 제거 뒤 finish_frame으로 회수합니다. external source는 tooltip/title 없는 일반 Button이므로 native-only hover 설명을 제거했습니다. public show API/실제 control ID·page/zoom/cache는 유지합니다. 신규dark/light×6×disabled2 actual AX/24px/방향/font/theme/Escape/hover/unmount1 PASS·PDF/HWP 기존 caller 영향 각1 PASS·strict16.71초입니다. fixture color 키 실패/명령은 native-problems QA PDF·HWP 후속이 정본입니다. 이전 preview 미연결 문구는 당시 상태이며 전체 arrow/motion/collision/GUI는 미완료입니다.

## 원본 DOM 치수·배치·arrow painter 후속

`tooltip-placement.rs`, `tooltip-placement-tests.rs`, `problems-tests.rs`와 격리 source reference를 추가했습니다. 실제 source DOM의 line16/body30/gap10·10×10 회전 rounded arrow·center shift/opposite flip을 공용 renderer에 연결합니다. 이전 line18은 추론값이었으며 실측 JSON/screenshot으로 정정했습니다. 원본과 pinned dependency 소스를 확인했고 Radix/Floating UI의 MIT 고지를 보존했습니다.

actual renderer height32/기대30 RED 뒤 네 방향 geometry/AX1 PASS입니다. 이후 첫 sizing에서는 본문 invisible인데 새 Context layer_painter의 기본 opacity1로 arrow만 그려지는 RED를 재현했습니다. arrow를 Area callback 내부 Frame 뒤에 그려 ui.painter의 visibility/opacity를 그대로 상속하게 수정했고 원본 DOM8/표시-state/Provider arrow hit의 신규3 PASS를 확인했습니다. Provider는 실제 회전 rounded 면적과 viewport clip을 쓰므로 빈 bounding corner down은 outside로 닫습니다. 이 검사는 뒤에 실제 Button을 겹치지 않았으므로 egui 전체 hit blocking/modal parity는 아직 미완료입니다.

추가로 안정 상태의 라벨 폭 변경 후 Area의 실제 크기만 바뀌고 repaint_delay=Duration::MAX인 RED였습니다. Frame 크기와 이전 Area 크기가 다르면 repaint를 예약하며 다음 프레임에서 새 중앙 배치를 사용하도록 수정했고 신규1 PASS입니다. 같은 프레임의 즉시 relayout을 보장한 것은 아닙니다.

Problems style/preview actual caller 영향 각1 PASS이며 unchanged provider 수명/액션/cache 성공은 재사용합니다. source reference의 body ready 누락 hidden screenshot은 PASS로 세지 않고 fixture readiness/visible predicate를 수정한 뒤 실제 source screenshot을 저장했습니다. 105초 성공 빌드는 브라우저 권한 실패 뒤 반복하지 않았고 동일 build·격리 profile 실행만 승격했습니다. 정확한 명령/시간/실패/최종 strict는 native-problems QA의 원본 DOM 후속 절이 정본입니다. CSS motion/closed Presence·전체 collision/viewport/GUI와 M8은 미완료입니다.

## CSS motion·closed Presence와 이동한 본문 hit-test 후속

source의150ms/ease·scale95/slide8·closed Presence를 실제 CSSAnimation32시점과 finish 후 DOM 제거로 확인했습니다. Widget별 Motion을 사용해 closed Role은 기한까지 보존하고 trigger 설명은 즉시 제거하며 Escape는 닫힘 애니메이션 동안에도 tooltip이 소유합니다. Explorer validation처럼 원본에서 Content 자체를 조건부 제거하는 경우는 즉시 unmount를 유지합니다. 기존 fixture의 즉시 paint/closed Role 제거·18px line·scale 중 정수 radius 비교는 source 실측으로 정정했고 수정 실패11개와 공용 easing Toast 영향1개만12 PASS입니다. source 측정/신규 motion2 PASS는 재실행하지 않았습니다.

새 실제 뒤쪽 Button 재현은 이동한 본문 안의 layer_id_at이 Background인 RED였습니다. 그림만 transform하고 Area 입력 rect는 원래 위치인 것이 원인이었습니다. pinned egui의 입력/그래픽 공용 set_transform_layer로 연결하고 clip은 inverse transform으로 viewport에 유지했습니다. 이어 layer 판정은 맞아도 hover-only Area 경계에서 Button release 클릭이 통과하는 별개 RED였습니다. 포커스 없는 CLICK sense로 tooltip 본문이 클릭을 받도록 바꾼 뒤 실제 Button press/release 비실행1 PASS(compile5.21초/suite0.02초)입니다. arrow 전체/rounded corner/모든 mixed/controlled owner/GUI까지 통과한 것은 아닙니다.

graphics_mut 중 Context.content_rect 재진입은 실제 debug deadlock panic이었으며 guard 진입 전 clip을 읽도록 수정했습니다. 임의 OS process를 종료하지 않았습니다. 정확한 명령/관찰값은 native-problems QA의 motion 절이 정본입니다. motion 변경 전 strict는 새 소스 검증으로 재사용하지 않습니다.

## 자식 Label AX 좌표와 남은 rounded input 경계

layer transform은 egui 그림/입력만 바꾸고 Response의 AX bounds를 자동 변환하지 않았습니다. 실제 closed75ms Label 좌표 비교에서 AX `[166,67]-[258.4375,83]`와 그림 약 `[167.8,68.3]-[256.5,83.7]`의 RED를 확인했습니다. 실제 Label Response를 Frame 반환값으로 받고 부모와 같은 transform을 bounds에 적용해 확장 검사1 PASS·actual modal 영향1 PASS·최종 native strict16.63초입니다. 다른 성공 검사는 반복하지 않았습니다.

기존 빌드를 재사용한 source hit16위치 측정에서 화살표 중심/tip은 Tooltip 자식이고 빈 arrow corner와 body rounded corner는 Tooltip 자식이 아니었습니다. 현재 Provider의 body Rect와 egui Area의 사각 WidgetRect는 둥근 모서리를 그대로 차단하므로 전체 hit parity는 미완료입니다. 회전 arrow의 membership 성공 또는 본문 이동 영역 뒤 Button 비실행 성공으로 전체 arrow/rounded input 완료를 대체하지 않습니다. 정확한 source JSON/절차는 native-problems QA의 자식 AX/source hit 절이 정본이며 다음 구현 경계입니다.

## 정확한 rounded body/arrow 입력 영역 후속

앞 절의 사각 input 미완료는 수정 전 기록입니다. source JSON16위치의 actual Button 재현에서 arrow tip layer가 Tooltip이 아닌 RED를 확인했습니다. pinned egui의 사각 WidgetRect·렌더 전 hit-test가 원인이며 native workspace에 같은0.36.2 source patch를 연결했습니다. viewport/pass 소유 exact predicate가 hit-test/Context layer lookup을 함께 제한하고 실제 Tooltip body/arrow transform·clip과 CLICK proxy를 공유합니다. 빈 arrow/body rounded corner는 뒤 Button 클릭을 통과시키고 arrow center/tip은 차단합니다. 실제16위치1 PASS·두 viewport 같은ID/이동/Arc 회수1 PASS·기존 modal/Problems/motion/AX 영향4 PASS·engine/app strict22.70초 exit0입니다. vendor111파일 원본 동일/MIT·native lock만 path source 전환·yanked lock 재생성 실패와 fixture 정정은 native-problems QA의 정확한 입력 영역 절이 정본입니다. vendored source는 authored Rust 비율에 포함하지 않습니다.

controlled validation·disabled/Context의 sticky transform은 아직 별도 수명 정리가 필요합니다. 기본 입력 영역 회수 성공을 전체 owner graph/GUI 완료로 세지 않습니다.

## controlled·disabled·Context의 sticky transform 회수

controlled 오류 제거와 일반 disabled 상태 각각 실제 transform 잔존 RED였습니다. controlled는 owner 없는 Appearance 렌더 경로, disabled는 기존 Widget을 덮어쓴 뒤 content 종료 기록 상실이 원인입니다. Provider의 viewport/pass 소유 실제 LayerId 목록에 일반/controlled 렌더를 등록하고 화면 제거·disabled·Context 교체·viewport 제거에서 lock 밖 identity 복원을 적용했습니다. controlled는 일반 open/skip graph 밖에 유지해 Explorer Escape를 소비하지 않습니다. 신규 owner2건·기존 motion 영향3건·actual Explorer1건·서로 다른ID의 실제 child viewport 제거1건이 통과했고 앱 strict17.62초입니다. 넓은 motion 필터에 포함된 변경 없는 source32 수식1건의 불필요한 재실행도 QA에 구분했습니다. 같은ID viewport transform·controlled close-attempt/full graph·GUI는 아직 미완료입니다.

## 동일 widget ID의 두 viewport transform 충돌

viewport-owned input region만으로는 global `Memory.to_global`의 LayerId 충돌을 없애지 못했습니다. Root/child의 같은 widget ID에서 actual child 제거 뒤 child layer가 Root transform을 계속 조회하는 RED였습니다. 보조 viewport의 native Tooltip ID를 trigger+viewport로 namespace하고 track/render/clear에 동일 함수를 적용했습니다. 동일ID 신규1 PASS 뒤 서로 다른 위치의 두 변환 독립성을 강화해1 PASS·서로 다른ID 제거 영향1 PASS·최종 앱 strict16.98초입니다. Root ID는 바꾸지 않았고 정확한 실제 명령/실패/증거는 native-problems QA의 동일ID viewport 절이 정본입니다. 전체 controlled close-attempt·mixed/modal/DPI/GUI는 아직 미완료입니다.

## controlled 요청과 같은 pass의 전역 닫기/AX

원본 Explorer는 오류가 없어도 controlled false Trigger를 유지합니다. native는 오류 Content가 있을 때만 Provider를 호출해 빈 input의 focus/hover 열기 요청과 전역 닫기 효과가 빠졌습니다. actual prop/setOpen 요청·is_delayed/skip deadline을 분리하고 오류 유무와 무관한 실제 input metadata를 연결했습니다. 초기 controlled 오류의 닫기 시도는 초기 지연을 해제하지 않으며 반복 닫기 요청은 actual 오류를 유지한 채 timer만 갱신합니다. 외부 prop 변경과 input Enter는 onChange/버튼 click으로 처리하지 않습니다.

전역 닫기 상태 연결 뒤 같은 pass normal Trigger의 described_by 잔존은 실제 RED였습니다. 현재 pass의 Response를 pending에 모아 모든 요청 뒤 finish_frame에서 최신 상태로 한 번 렌더하도록 수정했습니다. pending은 finish/새 pass/Context 교체에서 회수합니다. 신규 controlled3건·기본 Provider 영향8건·actual paint/hit/AX/modal/Explorer 영향8건 PASS·최종 앱 strict19.13초입니다. 초기 fixture 인자 누락 E0061과 성공 focus1건의 불필요한 필터 재포함은 QA에 실제 실행대로 구분했습니다. 전체 mixed events·top-modal Escape/조상 scroll·source callback/GUI 실측은 아직 미완료입니다.

## source의 닫힘 Content listener와 빠른 focus 재진입

source uncontrolled Tooltip observer를 격리 Chrome에서 한 번 측정했습니다. CSS Presence를 pause한 상태의 first→second→first focus에서 마지막 first가 열림 요청 직후 다시 닫힙니다. 기존 mounted Content의 document `tooltip.open` listener가 own reopen도 받는 것이 원인입니다. 후속350ms/400ms hover 기록은 animation pause 조건이라 일반150ms 경과 근거로 쓰지 않습니다. controlled 오류 input은 blur 직후 전체 제거되므로 input을 계속 유지하는 혼합 focus 가정도 채택하지 않습니다.

actual native는 third focus에 Some(first)가 남는 RED(7.56초/0.02초)였습니다. normal request_open에 own closing Motion의 present listener를 반영해1 PASS(6.14초/0.02초)이며 같은 pass AX 설명도 닫히고 Content 종료 뒤 새 focus는 정상 열립니다. closed Role/AX와 controlled focus의 영향2건 PASS·최종 앱 strict17.24초입니다. 원자료/정확한 명령·fixture 한계는 native-problems QA의 source focus callback 절에 저장했습니다. 전체 사건 순서·controlled top-modal Escape/조상 scroll·unpaused 전체 source/GUI는 미완료입니다.

## 실제 조상 scroll과 프로그램 offset 변경 누락

actual 중첩 ScrollArea를 움직여도 normal Tooltip open/AX 설명과 controlled 오류의 닫기 요청 timer가 남았습니다. Provider가 물리 조상·실제 scroll 변경을 추적하지 않은 것이 원인입니다. stable WidgetRect parent 또는 AX 재배치 그래프로 대체하지 않고, pinned native egui의 unique UI 부모 관계와 viewport/pass별 final offset을 연결했습니다. sibling/descendant/무변화/clamp는 닫지 않고 actual 조상만 final render 전에 닫습니다.

첫 수정은 begin의 persisted State만 비교해 프레임 사이 공개 State.store 변경을 놓쳤습니다. offset12에서 open 잔존 RED를 확인한 뒤 이전 pass의 실제 최종 offset으로 비교 경계를 수정했습니다. actual wheel·AX 비활성·AX 재배치·두 viewport 같은 Trigger·재배치/제거/재등장도 별도 경계로 확인했습니다. 고유 신규3·공용 영향6 PASS, engine/app strict20.61초 exit0이며 정확한 실패/fixture 정정/성공 재사용은 native-problems QA의 실제 조상 scroll 절이 정본입니다. source DOM scroll 실측·전체 graph/GUI parity 완료로 세지 않습니다.

## Escape 선처리와 focus 후적용·취소된 input metadata

원본 focus→Escape는 닫힘, Escape→focus는 새 열림입니다. native begin_frame이 Escape를 전체 선처리하고 소비자별 focus를 나중에 적용해 focus→Escape 및 초기 focus→Escape에서 open/AX가 다시 남았고 초기 경우에는 하위 consumer까지 Escape가 전달됐습니다. actual source3순서와 native UI2순서의 RED를 확인한 뒤 등록된 Trigger의 raw AX/key를 전역 사건 순서로 적용하고 실제 소비한 Escape index만 제거했습니다. Provider timer/closed Presence와 같은 pass 설명까지 신규1 PASS·공용 영향15 PASS입니다.

actual Explorer 오류 input이 focus를 가진 상태에서 일반 hover Tooltip도 열린 경우 Escape는 input에서 stopPropagation·취소돼야 합니다. Provider의 Escape 비소비는 유지됐지만 취소된 draft의 Output.input이 같은 pass에 남는 별개 RED였습니다. 취소 분기에서 metadata를 회수하고 repaint를 요청해 신규1 PASS·기존 create/rename validation와 IconButton alias 영향2 PASS·rename/blur/합성 IME integration1 PASS·최종 strict17.05초입니다. 같은 pass에 이미 그려진 input 픽셀 제거 또는 전체 controlled top-modal ordering의 완료 근거는 아닙니다.

source key 측정의 same-context Clock 실패는 context별 격리로 수정했습니다. Playwright Clock의 BrowserContext 소유는 실제 설치된 구현에서 확인했습니다. source event 기존 자료는 유지하되 사례별 독립32ms라는 주장에는 사용하지 않습니다. 정확한 명령/실패/시간/성공 재사용은 native-problems QA의 ordered AX/key 절이 정본입니다. 등록 전/동적 Trigger·전체 pointer/touch/viewport·top-modal/조상 scroll·전체 GUI는 남아 있습니다.

## document capture Escape와 늦게 마운트한 Modal (2026-10-05)

원본 draft의 React stopPropagation은 앞선 document capture를 막지 않습니다. 실제 원본 draft/shared Dialog를 쓰는 source3사례에서 valid/invalid input Escape는 normal close 후 draft cancel이며 later Dialog는 normal hover를 유지한 채 먼저 닫힙니다. 앞 절의 normal hover 보존 기대는 잘못된 추론으로 정정합니다. input 취소 metadata 수정 자체는 유지합니다.

actual Explorer normal 설명 잔존과 later egui Modal의 Escape 미전달 두 RED를 재현했습니다. Provider의 Tooltip capture 닫기와 input target 취소를 분리하고 native egui의 viewport/pass dismissal 마운트 순서를 연결했습니다. paint Order/AX 순서를 마운트 순서로 대체하지 않으며 closed Tooltip Presence는 기존 위치를 유지합니다. source3사례1회·native 수정2/registry 수명1/영향10 PASS·engine/app strict20.72초입니다. source build1회·selector/child viewport fixture 오류와 vendor108 동일/8변경/MIT·검증 명령은 native-problems QA의 document capture 절이 정본입니다. 전체 dynamic/current-pass·Popup/menu/Modal Presence·mixed/GUI parity는 미완료입니다.

## 같은 pass의 pointer click 뒤 AX focus가 다시 닫히는 오류 (2026-10-05)

등록된 actual Trigger에서 pointerdown/up 뒤 AX Focus를 전달해도 Tooltip open/설명이 None인 RED였습니다. raw 단계는 down-ref만 갱신하고 소비자별 any-down/aggregate click이 나중에 실행돼 사건 순서를 뒤집었습니다. pointer close/실제 primary click ID를 같은 raw replay에 연결하고 이미 적용한 click을 반복하지 않도록 수정했습니다. 신규 Trigger는 pointer_replayed 대상이 아니므로 기존 fallback을 유지합니다.

첫 수정 뒤 외부 클릭 사례만 실패해 egui Response 생성의 aggregate 외부 focus 해제가 더 늦은 AX Focus까지 지우는 것을 확인했습니다. 실제 pointer press/click release index 뒤 Focus 요청을 보존하도록 native-only Context를 수정했습니다. 순서8조합 신규1·엔진 Presses/Clicks/Never×3위치 신규1·입력/Modal/Explorer/hit 영향10 PASS·engine/app strict20.08초입니다. compile qualification/private import 및 조회 경로 오류는 native-problems QA의 pointer/AX focus 절에 제품 RED와 분리했습니다. 원본 DOM 재계측·여러 release/동적 current-pass/touch pointerType/Tab/IME/전체 GUI 완료 근거는 아닙니다.

## 실제 검증·잔여

Settings 위치9도 source TOP Tooltip 대신 기본 hover만 사용해 actual AX Focus의 role0 RED였습니다. 실제 interface section Response metadata를 앞선 Settings→App 공용 renderer에 추가해 dark/light9개·AX/이름/설명·theme/font/TOP·Escape 제거1 PASS입니다. 실제 클릭/Change::Position·geometry/숫자 blur 처리 구현은 불변이며 기존 native-settings-surface QA 성공을 재사용합니다.

테마 Picker·ANSI16은 기본 on_hover_text에서 실제 Tooltip AX·원본 BOTTOM/TOP·App owner가 빠져 있었습니다. actual role 없음 RED 뒤 현재 Response/label/align metadata와 Picker→Editor→Settings→App 전달을 연결했습니다. Theme editor가 닫힌 pass에는 metadata를 넘기지 않습니다. actual dark/light17곳의 role/설명/theme/font/방향1 PASS와 기존 Picker hex blur/reset 영향1 PASS입니다. 인접 표본의 black/기대red 진단은 원본 global transit grace 억제와 같은 것으로 확인했고, 각 표본 renderer 검사는 grace 바깥에서 진입하도록 fixture를 정정했습니다. 전체 browser 이벤트 parity로 주장하지 않습니다.

source IconButton은 HTML title이 아닌 span 래퍼 RadixTooltip입니다. token reset·bold/italic·theme duplicate/edit와 disabled span focus는 미완료이며 다음에 실제 래퍼를 연결합니다. 정확한 source/실패/검증은 native-problems QA 테마 후속 절이 정본입니다.

단축키 Editor의 actual reset/unbind도 기본 hover tooltip이 source BOTTOM·공용 owner focus·AX를 보존하지 못했습니다. 실제 modal의 role0 RED 뒤 내부 Output metadata와 App renderer를 연결했습니다. 기존 shell 뒤 finish_frame은 늦게 그리는 modal 위젯을 회수할 수 있어 modal 렌더 뒤로 이동했고, actual dark/light×2버튼·AX·첫 Escape tooltip만 닫기/다음 Escape modal닫기1 PASS·strict16.93초입니다. 자세한 명령은 native-problems QA의 단축키 후속 절이 정본입니다.

고유10검사 PASS·app lib/tests strict16.22초/bin/mock18.20초를 재사용하며 마지막 관련 lib clippy `-D warnings` exit0(5.25초)입니다. 정확한 명령·실패·fixture 정정·성공 재사용은 [native-problems QA](../quality-assurance/2026-10-04-m8-native-problems.md)의 공용 tooltip 후속 절이 정본입니다.

이 범위는 공용 provider 기본 수명과 Problems/IDE/글꼴/사용량 status 호출부입니다. 명시한 실제 content AX role/trigger described_by는 위 후속에서 확인했습니다. 모든 native tooltip consumer, tooltip content의 전체 hit-test/modal 경계, arrow·animation/text-balance·Radix collision shift, 모든 pointer/touch/keyboard 혼합 순서·PointerGone·조상 scroll, 실제 NativeApplication GUI/DOM 픽셀과 M8 전체는 미완료입니다. OS/IME/VoiceOver·보호 bundle·제품 TS·manifest/lock·Git은 변경하지 않았습니다.
