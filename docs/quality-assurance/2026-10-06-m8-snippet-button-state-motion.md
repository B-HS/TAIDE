# M8 Snippets Button 포커스·비활성 전환

## 대상 파일

- `native/taide-native-ui/src/button-color-motion.rs`
- `native/taide-native-ui/src/snippet-editor.rs`
- `native/taide-native-ui/tests/snippet-editor.rs`

## 리포트

원본 Button의150ms/cubic-bezier(0.4,0,0.2,1)를 focus-visible ring의 spread/색상·outline border·disabled opacity에 연결했습니다. 기존 owner namespace/Drop 회수·중간 반전 단축을 색상과 부동소수점 amount가 함께 사용합니다. 링은0→3px와 transparent→ring 색상, outline border는 app.border→app.focusBorder를 각각 보간합니다. 일반 ring50%와 destructive20% target·modality 정책은 유지합니다. CSS 전환의 경계이므로 최초 프레임부터 완료 색상/두께가 나타나는 것으로 검사하지 않습니다.

실제 버튼 추가 경계에서 SDK의 즉시 disabled alpha를 일시적으로1로 두고, 기존 부모 opacity를 보존하면서 자신의 보간 opacity를 적용합니다. 배경·텍스트·아이콘·outline shadow·링이 같은 opacity를 사용하고 추가가 끝나면 부모 opacity/disabled alpha를 원래 값으로 복원합니다. Modal Presence opacity와 inert 배경의 disabled alpha1을 보존합니다. 버튼 ID를 추가 scope로 바꾸지 않습니다. 입력·접근성 enabled는 실제 ui.add_enabled에 맡기므로 저장 중에는 첫 프레임부터 입력이 차단됩니다. 현재 enabled=false를 hover target 계산에도 반영합니다.

outline shadow-xs의 offset0/1·blur2·black5%는 정적 target을 유지하며 disabled opacity만 함께 전환합니다. shadow를 다른 모양으로 바꾸거나 vendor SDK를 수정하지 않았습니다.

후속 구현 대조에서 기본 border와 보간 border를 둘 다 그리는 경계 및 inert 부모의 alpha1이 실제 isDeleting disabled50%를 덮는 경계를 수정했습니다. Frame은1px transparent stroke로 border-box 공간을 유지하고 border 색상은 한 번만 그립니다. 자신의 enabled=false를 부모 inert alpha보다 우선 처리합니다. 원본 오류가 아니라 이번 전환 구현 중 발견한 경계로 [원인 문서](../bug/2026-10-06-snippet-double-border-and-inert-opacity.md)에 기록했습니다.

공식 근거는 [CSS Transitions1](https://www.w3.org/TR/css-transitions-1/#starting)의 property별 전환/반전, [Tailwind ring](https://tailwindcss.com/docs/box-shadow)의 box-shadow 모델입니다. 설치된 egui Ui.add_enabled/disable/set_opacity, Painter.set 및 shape_transform.adjust_colors 구현을 확인했습니다.

## 검증

공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, Cargo `/Users/hyunseokbyun/development/rust/cargo/bin/cargo`, target-dir `/private/tmp/taide-m8-menu-build.j6Efnw`입니다.

1. `cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir ... --test snippet-editor snippet_button은_포커스 -- --nocapture`: 최초1 PASS, build1.53초/suite0.12초·filtered15. 기존 포커스 검사의 기대 시점을150ms 계약으로 갱신하고 실제75ms 중간 ring 두께/알파 및 border 색상을 추가했습니다. pointer autofocus의 ring 없음→Tab destructive/Shift-Tab outline→이동 후 keyboard ring 유지→창 focus 회수/복원→disabled 회수를 같은 actual UI 세션에서 확인합니다.
2. `cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir ... --test snippet-editor snippet_save는 -- --nocapture`: 최종1 PASS, build0.63초/suite0.11초·filtered15. 실제 Save의 Enter 활성화→typed 저장 요청→첫 disabled 프레임 ID 유지/상호작용 없음→75ms 배경·glyph 색상→150ms50%→클릭 차단→실제 Saved reply 수락→enabled 즉시 복구/150ms 색상 복구를 확인했습니다.
3. 생산 source 변경 후 `cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir ...`:0.58초 exit0/경고0. 이후에는 fixture 변경만 있어 같은 Wasm 검사를 반복하지 않습니다.

후속 두 생산 경계의 단일 검사 `cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir ... --test snippet-editor snippet_outline은 -- --nocapture`는 최초1 PASS, build1.27초/suite0.12초·filtered16입니다. #31324480 border가 실제 한 번만 그려지는 것과 실제 Delete 요청 뒤 Dialog 닫힘 Presence가 남은 동안의75/150ms border/shadow opacity를 확인했습니다. 해당 source 변경 후 normal Canvas Wasm은0.55초 exit0/경고0입니다. 앞의 focus/Save 성공은 동일하게 재실행하지 않았습니다.

비활성 검사 최초 실패2회는 fixture가 Context.run_ui 종료 후 read_response로 enabled를 읽어 이전 패스를 관찰한 것입니다. 첫 실패 뒤 ID·Response를 대조했고, SDK end_pass가 prev_pass/this_pass를 swap한 뒤 read_response가 this_pass를 먼저 읽는 실제 원인을 확인했습니다. 다음 실행 전 fixture를 현재 렌더에서 수집한 실제 Response의 snippet_interactions로 수정했습니다. 생산 입력/disabled 코드를 우회하거나 테스트를 삭제하지 않았습니다. 같은 성공은 반복하지 않았습니다.

## 남은 범위

이번 실제 경계는 일반 Button의 색상·ring/border·자신의 disabled opacity입니다. 다른 control과 Close/IconButton의 전환·전체 원본 viewport/theme/DPI/글꼴·전역 focus graph·최신 Chrome raster는 전체 UI 완료로 세지 않습니다. 임의 modern/wide-gamut 색상 보간이나 모든 OS focus 정책을 완료로 주장하지 않습니다. 실제 자동완성 UI/삽입·전체 native shutdown·나머지 Settings/App/assets·최종 gate가 남습니다.

Snippets1/4(25%)·M8 363/433(83.83%)·최종0/8 유지, 전체 ETA 산정 보류, main 직접·goal active·전체완료 전 Git 없음입니다. live 검사 handle은 없습니다. Wasm bindings/screenshot은 선행 lifecycle 소스입니다.
