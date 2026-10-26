# 빌드 캐시로 디스크가 가득 참

> 최종 갱신일: 2026-10-09 · 대응 커밋: `bf5175ca`

## 무엇을 지적받았나

2026-10-07, 배치 4 통합 검증 뒤 메인이 "디스크 여유 22GB(99%)"를 보고하자 사용자가 "아니 2테라 모델인데 용량이 다 달았다고? 재개하기 전에 빌드 캐시 이런 거 싹 다 제거하고 시작해"라고 했습니다.

## 왜 틀렸나

- 저장소 안의 빌드 산출물이 약 741GB였습니다: 루트 `target/debug` 457GB(8월부터 누적), `experiments/native-shell-spike/target/debug` 261GB(native 크레이트 공용 target, 테스트 대상 66개), spike 3곳 약 18GB.
- 메인은 세션 초반에 219GB·465GB를 확인하고도 감사 요약에 한 줄로만 적고 정리를 제안하지 않았습니다. 그 상태에서 전체 테스트 대상 빌드를 실행해 70GB를 22GB로 줄였습니다.
- native 크레이트가 개별 workspace인데 target은 하나를 공유하고, dev 프로필에 debuginfo 축소가 없어 테스트 바이너리마다 크기가 큽니다.

## 어떻게 고치나

- `cargo clean --profile dev`(루트), `cargo clean --manifest-path native/taide-native-app/Cargo.toml --target-dir experiments/native-shell-spike/target --profile dev`, spike별 `cargo clean --manifest-path …`. `--profile dev`는 `debug` 디렉터리만 지우므로 실기용 `.app` 번들과 릴리스 번들이 남습니다.
- `rm -rf`와 `cargo clean`은 자동 모드에서 거부됩니다. 사용자가 `Bash(cargo clean *)` 허용 규칙을 추가했습니다.
- 정리 뒤 751GB(59%)가 됐습니다.

## 언제 적용하나

- 전체 테스트 대상을 빌드하기 전과 배치가 끝날 때 `df -h`로 여유를 확인해 보고합니다. 여유가 100GB 아래로 내려가면 빌드를 늘리기 전에 정리를 제안합니다.
- 단계별 작업자는 필요한 테스트 대상만 빌드하고, 전체 대상 빌드는 메인이 배치 끝에 1회만 합니다.
- 근본 조치(native workspace 통합, 전용 target, dev 프로필의 debuginfo 축소, 통합 테스트 바이너리 통합)는 구조 정리 배치의 범위입니다.
