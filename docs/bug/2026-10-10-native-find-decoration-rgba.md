# 본문 찾기 반투명색 보존

본문 찾기 장식에 Color32의 premultiplied 바이트를 전달하면 장식 렌더러가 이를 unmultiplied RGBA로 해석해 색을 다시 어둡게 만들었습니다. `native/taide-native-ui/src/editor-find.rs`의 장식 생성 경계에서 Color32를 직접 받고 `to_srgba_unmultiplied`로 변환하도록 수정했습니다. 앱 공급과 UI 검사 호출도 같은 계약을 사용합니다.

`native/taide-native-ui/tests/editor-find.rs`의 새 회귀는 수정 전 0통과/1실패(exit 101)였으며 실제 `[103,66,11,126]`와 기대 `[208,134,22,126]`의 차이를 확인했습니다. 수정 후 찾기 대상 16건과 UI inspection 전체 358건, 앱 전체 628건이 통과했습니다. 보호 Trash 검사 3건은 제외했습니다.

로그는 `/private/tmp/taide-batch16-find-body-before.log`, `/private/tmp/taide-batch16-find-body-after.log`, `/private/tmp/taide-batch16-ui-full.log`, `/private/tmp/taide-batch16-app-full-after.log`입니다. native-host 경계에서만 변경했으며 browser 소스·manifest·lockfile은 변경하지 않았습니다. 동결 host/Wasm 컴파일도 exit 0입니다.
