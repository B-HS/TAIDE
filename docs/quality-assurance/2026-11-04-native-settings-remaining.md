# settings 남은 행 검증: 5(탭 소유권), 22(시험 알림)

> 작성일 2026-11-04 · 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`, 브랜치 `to_rust_native`
> 근거 문서 `docs/handoff/native-completion/15-remaining-settings.md` 1.1·3절
> 두 행 모두 코드 변경 없이 기존 App 검사를 병합본에서 재실행해 닫았습니다.

## 대상

| ID | 기능 | 결과 | 근거 |
| --- | --- | --- | --- |
| settings-5 | Settings 탭 소유권(보조 창 포함 프로젝트별 Owner) | complete | `실제_보조_창_settings는_자기_프로젝트_창_pane에_settings_json과_프롬프트를_열고_닫히면_메인에서_새로_mount된다` |
| settings-22 | 시험 알림 보내기와 억제 사유 toast | complete | `실제_시험_알림은_창_포커스가_없으면_가짜_플랫폼에_한번_보내고_첫전달_안내없이_성공을_알리며_플랫폼_오류를_알린다` |
| settings-23 | OS 알림 범주 7종 | partial 유지 | 검색 치환 생산자(explorer-search-33)는 2026-11-04 연결·검증 완료. 남은 것은 범주 7종 묶음 App 검사(메인 통합) |

## 검증

실행 명령(메인 작업 트리, 공통 옵션 `--locked --offline --target-dir experiments/native-shell-spike/target`):

```
cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=2 settings
```

결과: `running 48 tests`, `test result: ok. 48 passed; 0 failed` (11.28s).

- 이 필터는 모듈 이름에 `settings`가 들어가는 앱 lib 검사를 모두 실행합니다. 두 행의 검사가 여기 포함됩니다.
- settings-5 검사는 탭 메뉴 `tab.moveToNewWindow`로 Settings 탭을 보조 창으로 옮긴 뒤 그 창의 focused pane에 프롬프트·settings.json이 열리고, 프로젝트 B의 탭 수가 늘지 않으며, 창을 닫으면 탭이 메인으로 돌아와 새 owner의 `active == Section::Appearance`가 되고 보조 owner가 사라지는지 단언합니다.
- settings-22 검사는 `FakePlatform`에 `refusal`을 넣어 (1) 포커스가 없을 때 제목 `settings.notificationsSendTest`·본문 `notification.enableHint`로 정확히 1회 전달, (2) 첫 전달 info toast 없음, (3) platform 오류 문구가 error toast로 나오고 추가 전달이 없음을 단언합니다.
- 실행하지 않은 것: 실제 macOS 알림 센터 표시(허락 P2, 배치 31 실기), 병합 게이트(묶음 뒤 메인 실행).

## 남은 범위

- settings-23의 범주 7종 묶음 App 검사는 shell(agent·명령·LSP·첫 전달 안내), git(push/pull), explorer-search(검색 치환) 생산자가 모두 연결된 지금 상태에서 메인이 한 번에 추가해야 합니다.
- settings-28·30·42·48·67·78·80은 이 묶음 범위가 아닙니다.
