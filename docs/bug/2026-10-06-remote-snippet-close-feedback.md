# Snippets 종료 실패에서 누락된 Toast

## 대상·재현

`native/taide-remote-web/src/browser-editor.rs`, 공용 `snippet-editor.rs`·`settings-view.rs`입니다. actual Chrome에서 Snippets를 생성하고 구조화 입력 후 저장 응답을 hold합니다. Close가 Pending인 동안 InvalidArgument로 저장을 거절하면 Failed(Snippet)·dirty 초안은 정상 보존되지만 109 frame 이후에도 Toast 목록이 비어 있었습니다.

## 원인·수정

Canvas는 Open에서만 Settings를 그립니다. 응답이 만든 Notice는 편집기 내부에 남고 다음 show에서만 배출되므로 Pending/Failed 화면에서는 show_feedback이 실행돼도 받을 알림이 없었습니다. Views의 `take_snippet_notices`가 retained 편집기의 알림을 한 번 회수하고 BrowserEditor가 실제 응답 처리 후 기존 Feedback::Snippet 큐로 전달하도록 연결했습니다. 다음 일반 show와 중복되지 않으며 native의 기존 렌더 경로는 유지합니다.

## 검증·범위

공용 실제 Settings UI→save 요청→거절 응답→렌더 없이 알림 배출→두 번째 배출 0→disabled 렌더의 중복 0 검사 1 PASS(build 2.40초/suite .11초, filtered 9)입니다. 수정한 actual Chrome 연속 검사도 Failed에서 parseError Toast·Cancel·retry·최종 Ready/socket0까지 PASS이며 `docs/quality-assurance/2026-10-06-m8-snippets-browser-lifecycle.md`가 정본입니다. 실제 native shutdown 전체를 이 검사로 대체하지 않습니다.

하네스의 선행 두 오류는 별개입니다. 없는 `settings.snippets` 키를 actual `settings.snippetsSectionTitle`로 정정했고, Snippets SaveFailed 원본은 raw 오류 description이 아니라 번역된 `snippetEditor.parseError` title이므로 기대값도 정정했습니다. title을 기다려도 비어 있던 관찰이 위 생산 오류의 재현 근거입니다.
