# native 모델 폐기와 raw 진단 정리 미연결

## 대상·관찰

`native/taide-native-editor/src/store.rs`와 `native/taide-native-app/src/{application,lsp,lsp-diagnostics-tests}.rs`입니다. 원본 `src/entities/editor/model-registry.ts`의 model dispose는 각 session의 `src/shared/lib/lsp/adapters/diagnostics.ts` listener에서 raw URI를 제거합니다. native raw 후속 구현은 didClose/unbind 보존을 맞췄지만 실제 모델 제거·retarget을 raw 정리로 전달하지 않았습니다. 코드 대조로 확인했고 실행 전 RED를 주장하지 않습니다.

## 해결

실제 Store 변경 지점에서 opt-in 폐기 journal을 남깁니다. detach와 실패는 기록하지 않고 release/discard·retarget 이전 URI/displaced target·Untitled 저장 합류만 기록합니다. 프레임 안의 transient model도 기록하므로 snapshot diff의 누락을 피합니다. App은 visible Sync 이전에 이를 전송하고 큐 입장 실패 시 확인 처리하지 않습니다.

worker는 모든 session의 normalized raw URI를 제거하고 URI에 맞는 문서 binding만 닫습니다. 다른 문서/owner는 보존합니다. 현재 File-only/UTF-8 LSP 입장 경계를 유지하며 모델 폐기 뒤 새 raw 알림 자체를 막는 tombstone은 추가하지 않았습니다.

## 검증·남은 범위

core4 PASS(compile1.51초)·bridge 큐/실제 child2 PASS(compile13.94초/suite0.39초)·core strict1.27초·app strict17.23초입니다. 이후 기본 inactive/grace5초 후속도 구현했습니다. 직접 명령·최신 경계·기존 성공 재사용은 `docs/quality-assurance/2026-10-04-m8-native-problems.md`가 정본입니다. 누적 raw/pending quota·다중 provider/retarget·전체 provider/실제 App GUI/M8은 미완료입니다. manifest/lock·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.
