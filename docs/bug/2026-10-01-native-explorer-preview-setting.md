# Native 탐색기의 원본 preview 설정 불일치

## 대상·증상

- 대상: `native/taide-native-app/src/explorer.rs`, `tests/explorer.rs`입니다.
- 단일 클릭이 native Open preview=true를 반환했으나 실제 TS explorer container는 onOpenPreview/onOpenPinned 모두 preview=false를 전달합니다. source callback 이름과 실제 제품 설정을 혼동한 오류입니다.

## 원인·수정

- `!response.double_clicked()`로 단일 클릭을 preview로 추정했습니다. 실제 `src/widgets/explorer/explorer-container.tsx`의 `openRowFileTab`과 두 callback의 false 전달을 확인했습니다.
- 단일 클릭·focused Space/Cmd+Down을 false로 통일하고 directory는 열지 않도록 했습니다. TS를 바꾸거나 기존 runtime의 preview 기능·검색 결과 preview 정책은 변경하지 않았습니다.

## 실제 근거

- 신규 `탐색기_열기는_원본_container의_클릭_space_cmd_down_설정을_따른다`에서 pointer 클릭의 true/false 차이로 1건 실패, 0.02초입니다.
- 수정 뒤 해당 1건만 0.02초에 통과했습니다. 원본에 근거한 기대값은 유지했으며 전체 기존 검사를 반복하지 않았습니다.
- 전체 화면/기능 동등성·실제 OS 입력기·M8 완료는 이 좁은 수정의 판정에 포함되지 않습니다.
