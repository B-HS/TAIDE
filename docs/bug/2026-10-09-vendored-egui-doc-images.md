# Vendored egui 문서 검사 이미지 누락

2026-10-09 배치 9에서 egui 전체 대상을 직접 실행했습니다. 단위 50건은 통과했지만 문서 10건이 `include_bytes!` 대상 이미지 누락으로 컴파일되지 않았습니다. 최초 로그는 `/private/tmp/taide-batch9-egui-full-20261009.log`(exit 101)이며 기존 문서 예시의 경로를 확인했습니다.

누락 경로는 `native/taide-native-app/vendor/egui-input/assets/ferris.png`와 형제 경로 `native/taide-native-app/vendor/eframe/data/icon.png`입니다. 새 SDK 입력 API를 사용하는 예시는 아니며 원본 SDK 배포 레이아웃의 이미지 누락입니다.

설치된 egui/eframe 0.36.2의 `.cargo_vcs_info.json`에서 같은 원본 커밋 `49682f8baa058bf49e011035cfbd6e825f88a5ef`를 확인했습니다. 원본 Ferris를 내려받고 icon은 설치된 동일 버전에서 복사한 뒤 원본 blob SHA-1을 각각 대조했습니다. 각 fixture README에 출처·크기·해시를 기록하고 MIT 원문을 보존했습니다. 가짜 이미지를 만들거나 예시/검사기를 끄지 않았습니다.

재검사 명령은 아래 공통 명령 끝에 해당 필터를 붙였습니다.

`cargo test --manifest-path native/taide-native-ui/Cargo.toml -p egui --doc --locked --offline --target-dir experiments/native-shell-spike/target`

| 필터 | 결과 | 로그 |
| --- | --- | --- |
| `-- image` | exit 0, 7 통과·기존 1 ignored·160 filtered, 4.32초 | `/private/tmp/taide-batch9-egui-doc-image-after-20261009.log` |
| `-- atomics::atom` | exit 0, 6 통과·162 filtered, 3.34초. 기존 실패 3건 포함 | `/private/tmp/taide-batch9-egui-doc-atom-after-20261009.log` |

최초 성공을 재사용한 서로 다른 최종 결과는 단위 50·문서 167 통과, 문서 1 ignored입니다. 전체 대상을 두 번 실행한 결과로 표현하지 않습니다. fixture 폴더에는 새 Cargo 패키지/manifest가 없으며 앱 런타임·기존 eframe patch·의존 그래프·lockfile·remote-web 소스를 바꾸지 않았습니다.
