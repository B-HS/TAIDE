# Native AppFile 닫기 확인·model 폐기의 원본 차이

## 대상 파일

`native/taide-native-app/src/{application,app-file-views,app-file-views-tests,tabs,tab_close_batch}.rs`

## 증상과 원인

AppFile 읽기 연결 중 일반 File/Untitled의 dirty 확인을 AppFile까지 확장하고 앱 Exit에도 확인 latch를 추가했습니다. 마지막 AppFile 탭을 닫으면 model을 폐기하도록 구현했습니다. 이는 기존 TS를 재현하는 범위에 없는 동작 변경이었습니다.

원본 `src/widgets/editor-area/use-request-close-tab.tsx`의 isDirtyGatedTab은 file/untitled만 포함합니다. `src/entities/layout/layout.query.ts`의 useCloseTab도 file만 releaseClosedFileTabPath로 dispose합니다. `src/features/editor/code-editor.tsx`는 editor를 dispose하지만 module-level model-registry의 model은 남깁니다. appfile은 즉시 닫고 model은 메모리에 유지하는 것이 실제 원본입니다. load_layout은 AppFile persisted dirty를 초기화하며 hot-exit mirror 복구도 없습니다.

## 재현과 해결

기존 공유 문서 검사에 dirty AppFile의 원본 즉시 닫기 기대를 추가해 `cargo test --lib native_app_file_views는`에서 RED1건을 확인했습니다. suite0.01초·컴파일7.92초, 원본 즉시 닫기 기대 assertion이 실패했습니다.

AppFile을 File/Untitled dirty kind에 넣은 확장·추가 Exit latch·마지막 model 폐기를 제거했습니다. 닫힌 owner Session/편집 view는 회수하지만 살아 있는 sibling 및 target model은 유지합니다. closed metadata의 dirty를 임의로 지우지 않습니다. 기존 File/Untitled·pinned 정책은 유지합니다.

관련 검사만1회 재실행해1 PASS(suite0.02초·컴파일6.49초)입니다. dirty AppFile의 Batch immediate Close/실제 main·aux host 즉시 닫기·dirty closed metadata·sibling 유지·마지막 view 회수/model 보존을 확인했습니다. 최종 app lib/bin/tests strict exit0(13.10초)입니다. 기존 unaffected 성공은 재사용했습니다. closed/new 탭의 model sync/view state 및 실제 GUI/Exit 동등성은 여전히 다음 게이트입니다.
