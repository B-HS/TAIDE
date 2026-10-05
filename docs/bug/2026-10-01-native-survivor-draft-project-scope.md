# 삭제 후 공유 초안의 프로젝트 소유권 오류

## 관찰·원인

`native/taide-native-app/src/missing_draft.rs`의 Save As는 canonical source를 resolve_owning_project로 다시 선택한 project와 draft.project가 같아야 했습니다. 넓은 프로젝트에 source-missing mirror가 있고 더 좁은 overlapping 프로젝트가 열려 있으면 기존 mirror가 유효해도 좁은 owner를 선택해 Forbidden으로 거절했습니다. 탐색기의 선택 프로젝트 삭제 후 다른 프로젝트 탭·최신 mirror를 보존하는 실제 경계와 충돌했습니다.

## 수정

초안에 담긴 project의 현재 root를 구하고 source canonical identity·root 내부·원본 부재·그 project의 정확한 matching mirror를 검사합니다. destination은 기존 프로젝트 권한·dirty/mirror 검사와 원자적 저장을 그대로 사용합니다. 다른 root, 닫힌 project, 원본 복구, mirror 교체를 허용하지 않습니다.

## 검증

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer-delete 삭제후_공유초안 --locked --offline --target-dir experiments/native-shell-spike/target`: 실제 합성 overlapping root/missing mirror 복구 1건 RED(Forbidden, 0.02초)→수정 후 GREEN(0.03초)입니다. destination 본문·원본 부재·mirror cleanup·worker 0을 확인했습니다.
- [ ] 실제 OS Save As와 여러 alias/다중 surviving 프로젝트의 전체 동등성은 explorer-delete QA의 최후 게이트입니다.
