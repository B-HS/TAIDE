# M8 native toast 위치 설정 수명

## 대상 파일

- `native/taide-native-app/src/{toast.rs,application.rs}`
- 원본 `src/shared/constants/toast.ts`, `src/widgets/app-toaster/app-toaster.tsx`, `src/shared/styles/global.css`, `node_modules/sonner/dist/index.js` (Sonner2.0.7)

## 리포트

native는 위치 변경 때 기존 geometry만 옮겼으므로 원본 keyed 목록의 remount 수명과 달랐습니다. 원본의 `middle`은 Sonner의 `top`으로 매핑되고 CSS만 바뀝니다. 같은 horizontal의 Top/Middle은 timer/motion/widget identity를 유지하며 horizontal 또는 Top/Bottom key 변경은 local 상태를 새로 만듭니다. 예약된 부모 제거 callback은 취소하지 않습니다. 실제 browser/OS/full App 동등성 완료가 아닌 source 기반 결정적 구현입니다.

## 상세

1. Sonner의 `possiblePositions.map`은 `ol key=position`을 만들고 child Toast는 local remainingTime4000ms·removed/mounted/swiping과 motion을 보유합니다. lifetime effect cleanup은 timeout을 취소하지만 deleteToast의200ms removeToast callback에는 unmount cleanup이 없습니다. [React 공식 key/state 계약](https://react.dev/learn/preserving-and-resetting-state#option-2-resetting-state-with-a-key)과 설치 원본을 대조했습니다. 실제 브라우저에서 timer·paint 순서를 측정한 결과는 아닙니다.
2. 마지막 parsed Position의 canonical parent key는 `(notBottom, horizontal)`입니다. 추가 suffix와 invalid fallback도 기존 parse 계약을 유지합니다. 최초 설정은 기존 push 시간을 유지합니다. 같은 Top/Middle은 geometry hit cache만 무효화하고 timer/motion/capture/identity를 유지합니다. 다른 key는4000ms timer·local dismissed/motion/swipe state를 새로 만들되 entry ID/title/description·부모 expanded/interacting과 hidden pause를 보존합니다.
3. `dismissed_at`은 local 제거 그림이고 `scheduled_removal_at`은 부모 eviction 예약입니다. timeout/close/swipe의 공통 dismiss가 최초 예약을 보존합니다. remount는 local state만 지우고 예약 제거는 보존하며 재-dismiss에도 최초200ms deadline을 연장하지 않습니다. tick retention/repaint와 show repaint는 예약 시간을 읽습니다.
4. widget generation은 checked 증가하며 list/card/close/content ID에 포함합니다. 한 Area ID는 고정해 위치 변경마다 새 Area state를 쌓지 않습니다. 이전 widget의 AX click은 새 widget에 적용되지 않습니다. 이전 toast가 실제 focus를 소유할 때만 surrender하고 enabled/modal gate 아래 previous focus를 반환합니다. 이미 다른 widget이 focus를 소유했다면 바꾸지 않습니다. App background tick 전에 현재 Settings 위치를 관찰하고 show에도 같은 관찰을 적용해 late 설정 및 직접 renderer 호출을 연결합니다.
5. installed egui0.36.2의 public `Memory::surrender_focus`·`request_focus` 계약을 읽었습니다. private Area state/reset-all-areas·vendor patch·새 dependency를 사용하지 않습니다. 제품 TS·root/native lock/MSRV·보호 bundle·OS 설정과 사용자 데이터는 변경하지 않았습니다.

## 실제 검사

동일 native app manifest·locked/offline·공유 target·serial Cargo, synthetic text와 headless egui만 사용했습니다.

- [x] 신규 `--lib native_toast_position_key`: baseline1 RED (suite0.02초/compile2.48초, exit101), 위치 변경 뒤 remaining2s/expected4s입니다.
- [x] 초기 identity 기계 치환에서 Mount 내부 constructor까지 치환해 E0609 두 건이 발생했습니다. 실제 constructor를 복원했고 검사를 끄거나 타입/필드를 우회하지 않았습니다.
- [x] 변경 영향 `--lib toast::`:21 PASS/1 FAIL (suite0.03초/compile5.76초). 신규 position3건은 모두 PASS입니다. timer reset·same Top/Middle/suffix·invalid fallback·부모 pause·예약 제거 최초 시각·재-dismiss·count collapse·capture 취소·현재 focus 반환·disabled/외부 focus 보호·새 AX tree/이전 click 거절·middle geometry를 덮습니다. 나머지 기존 renderer/timer/swipe/AX/motion 검사도 변경된 identity/removal 영향 근거입니다. 순수 motion/swipe의 이 batch 성공은 반복하지 않습니다.
- [x] 실패한 기존 focus 검사에는 이전 단계의 focus-shadow200ms 전환 전에 full ring을 요구하는 오래된 즉시 그림 기대가 남아 있었습니다. timeline을 단조 증가로 유지하며 focus 시작 뒤200ms idle frame에서 최종 ring을 확인하도록 fixture만 수정했습니다. `--lib native_toast_focus`:1 PASS (suite0.02초/compile2.49초). 제품 변경 없이 나머지21 PASS는 재사용합니다. 신규 test 식별자의 AX 대문자는 소문자로 정리했습니다.
- [x] 최종 `cargo clippy ... --lib --bins --tests -- -D warnings`:exit0(12.25초). authored 두 파일 exact rustfmt --check·tracked git diff --check exit0, 두 untracked 파일 no-index whitespace 출력은 비어 있습니다(exit1은 /dev/null과 내용 차이). inherited Wry17 warning은 authored strict와 구분합니다. 마지막 제품 변경 뒤 성공이며 fixture만 수정한21 PASS를 다시 실행하지 않았습니다.

## 남은 gate

- [ ] 실제 browser의 remount/mounted paint·동일frame 예약 callback/새 position 순서·stationary hover/실제 DOM selection/touch·모든modal/aux/WebView/full App을 비교합니다. 합성 renderer는 실제 browser/OS event 순서를 대신하지 않습니다.
- [ ] 실제 OS/VoiceOver focus-return·AX announcement·pointer capture/hidden/wake와 whole-pixel/성능을 확인합니다. App tick 배선은 컴파일 근거이며 모든 Settings mutation UI 구현/실기 완료가 아닙니다.
- [ ] 원본 toast global observer/update/dismiss/action/promise API의 전체 계약과 전체 view/Monaco/palette/Settings·N1~N8은 별도 gate입니다. 현재 실제 warning/Settings-failed queue 경로만 연결했습니다.
- [ ] 별개 keybindings Tab RED와 PTY remount 결정은 미해결입니다. 전체 M8은0/8이며 cutover/TS 제거·성능/보안/서명/공증/rollback과 전체 완료 뒤 commit/push 조건을 유지합니다.

process/verify/save-docs 스킬은 기존 toast 체크리스트의 세분화·실패한 검사만 재실행·실제 결과와 미완료 분류에 적용했습니다.
