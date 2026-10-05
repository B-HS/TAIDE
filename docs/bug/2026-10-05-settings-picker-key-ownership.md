# Settings Picker의 조합 키와 첫 포커스 프레임

## 대상·관찰

`native/taide-native-ui/src/settings-code-view.rs`, `native/taide-native-app/vendor/egui-input/src/memory/mod.rs`, `native/taide-remote-web/tests/settings-resources.rs`, `tools/m8-remote-rust-file-probe.ts`입니다.

합성 IME Preedit 후 Escape를 보내면 검색 Popup이 닫혔습니다. 원본 Popover와 cmdk는 조합 중 키를 닫기·선택으로 처리하지 않습니다. egui의 기본 Escape 처리는 pass 시작에서 포커스를 해제하므로, 검색 입력의 현재 포커스를 기준으로 조합 상태를 검사하기 전에 상태가 사라졌습니다.

별도 Chrome 연속 검사에서는 자동 포커스를 확인한 직후 ArrowLeft를 누르면 최종 포커스가 `settings.terminal`로 이동했습니다. `set_focus_lock_filter`는 현재 포커스와 이전 프레임 포커스가 모두 같을 때만 동작합니다. 최초 `request_focus`와 같은 pass의 필터 설정은 적용되지 않아 다음 키가 기본 목차 탐색으로 처리될 수 있었습니다.

## 수정·한계

검색 Picker가 Preedit·Commit을 소유하고 조합 중 Enter/Escape/탐색 키를 설정 선택으로 소비하지 않습니다. Popup은 명시 open 상태로 렌더링하고 닫기 요청에서 조합 Escape와 바깥 클릭을 구분합니다. 검색 입력의 Escape·수평/수직 화살표 필터를 유지합니다.

기존 vendored egui에 `Memory::request_focus_with_filter`를 추가해 포커스 요청과 첫 키 필터를 원자적으로 설치합니다. 기존 `request_focus`·`set_focus_lock_filter`의 동작은 변경하지 않았습니다. Picker의 검색·trigger 복귀에서만 새 API를 사용합니다. SDK의 `missing_docs` 계약에 따라 공개 API의 영어 Rustdoc 한 줄을 추가했으며 내부 설명 주석·검사 억제는 추가하지 않았습니다. 기존 vendor의 입력/버튼/IME 패치는 보존했습니다.

최종 포커스 전에 캡처한 Trace.has_focus를 검사한 별도 portable 실패는 제품 실패가 아니었습니다. 최종 Context.memory 포커스 ID로 수정하고 첫 프레임 ArrowLeft 보존도 검사합니다. 셸 그림 검사에서 같은 문자열을 가진 single-line 입력란을 선택한 실패 역시 fixture 오류이며, 실제 목록 행 내부 TextShape로 구분했습니다.

실제 OS CJK 입력기·VoiceOver 검증은 사용자 수행 마지막 순서를 유지합니다. 합성 egui IME 검사·AccessKit 트리는 그 실기 성공의 대체 근거가 아닙니다. 검증 명령과 결과는 `docs/quality-assurance/2026-10-05-m8-settings-picker-and-shell-parity.md`에서 관리합니다.

Chrome의 첫 키 수정 확인은 검색/일반 닫기/바깥 클릭까지 통과했고 Drop 뒤의 null snapshot 호출로만 실패했습니다. 제품 probe를 해제한 뒤 상태를 다시 호출하지 않도록 시험 코드를 수정한 최종 연속1은 seq11/write0·Drop/socket0·quiet1.1초 요청 불변·오류0으로 통과했습니다. 공개 API와 최종 native 소비 check5.72초도 통과했습니다.
