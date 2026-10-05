# Native 알림 목록 focus 수명과 AX 이름

## 대상 파일

`native/taide-native-app/src/toast.rs`

## 리포트

일반 Tab 후보에서 제외한 알림 목록을 Alt+T로 직접 focus했을 때 egui의 다음 pass 수명 검사에서 focus가 사라졌습니다. 별도로 Label metadata의 role만 ListItem으로 바꾸면 문구가 value에 남고 AX 이름은 None이었습니다.

## 원인·해결

hover-only widget은 interactive/focusable 경로의 used ID 등록이 없어서 dead-man 검사에서 제거됩니다. 공개 `Context::check_for_id_clash`가 문서화한 non-interactive state ID 등록을 사용하고 공개 interaction/request_focus로 목록 focus를 유지합니다. private `create_widget`를 사용하거나 검사기를 끄지 않습니다. 원본의 tabindex=-1과 최신 card/close 순서를 보존합니다.

AX Label은 문구를 value에 넣습니다. ListItem의 명시적 label/description을 연결하고 기존 value를 제거했습니다. polite/non-atomic named List의 하위 전체 entry와 버튼 관계를 보존합니다.

## 검증·잔여

focus None RED→목록 수명 수정 후 raw fixture FAIL→최종 focus/input/2-pass1 PASS(0.01초), AX 이름 None RED→metadata 수정/정리 최종1 PASS(0.02초)입니다. 원본 renderer 영향5 PASS(0.02초)와 authored strict exit0(12.47초)를 기록했습니다. 실제 VoiceOver/전체 modal/다중 창·원본 animation 완료는 아니며 `docs/quality-assurance/2026-10-03-m8-native-toast-focus.md`가 정본입니다. 별개의 keybindings Tab clipping RED를 해결했다고 주장하지 않습니다.
