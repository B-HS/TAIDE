# M8 Snippets Close 투명도 전환

## 대상 파일

- `native/taide-native-ui/src/snippet-editor.rs`
- `native/taide-native-ui/tests/snippet-editor.rs`
- 원본 `src/shared/ui/dialog.tsx`, `src/features/snippet/snippet-entry-editor.tsx`

## 리포트

원본 Close의 `opacity-70 transition-opacity hover:opacity-100`을150ms/cubic-bezier(0.4,0,0.2,1)로 재현했습니다. 아이콘 tint만 즉시 바꾸던 구현 대신 기존 owner별 amount 전환을 사용하고 아이콘·2px focus 링·2px offset에 같은 opacity를 적용합니다. Modal Presence의 부모 opacity를 보존하고 paint 뒤 복원합니다. Close의 포커스 표시는 원본 `focus:` 규칙이며 일반 Button의3px focus-visible 전환으로 바꾸지 않습니다.

Trash의 원본 클래스에는 transition이 없으므로 muted→error hover 색상은 같은 프레임에서 바뀌는 동작을 유지합니다. 설치된 Radix DialogClose 구현은 `data-state`를 출력하지 않으므로 해당 조건부 배경/글자 클래스만 보고 활성 색상을 추가하지 않습니다. SVG 크기·기존 입력/접근성·Tooltip 경계는 보존합니다.

기본 전환 경계는 [Tailwind transition-property](https://tailwindcss.com/docs/transition-property)의 opacity property/기본150ms 규칙과 설치된 원본 소스를 대조했습니다. 실제 Chrome 픽셀 일치나 전체 포커스 그래프의 완료 근거가 아닙니다.

## 검증

환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, cargo 실행 경로는 같은 디렉터리의 `bin/cargo`, target-dir는 `/private/tmp/taide-m8-menu-build.j6Efnw`입니다.

```sh
cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_close의 -- --nocapture
cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- Close motion 확장/기존 focus·outline·좁은 폐기 Dialog 영향의2건이 첫 실행 PASS: build1.32초/suite.14초·filtered15입니다. 신규 경계의 성공 반복은 없습니다.
- 실제 Settings/Editor UI에서70% 최초 paint→키보드 focus 링/offset70%→hover75ms 중간값→150ms100%→이탈75ms 중간값→150ms70%를 읽었습니다. 같은 연속 검사에서 Trash hover가 즉시 error 색상으로 바뀜을 확인했습니다.
- 기존 focus 검사의 Close 링 기대값을 원본 전체 요소70% opacity로 정정했습니다. outline shadow/300px 폐기 Dialog geometry도 같은 영향 검사에서 PASS입니다.
- normal Canvas Wasm check.55초 exit0/경고0입니다. bindings/screenshot은 선행 lifecycle 소스이며 이번 수정의 Chrome raster 결과로 쓰지 않습니다.

## 남은 범위

Snippets의 전체 글꼴·theme/DPI/포커스/AX·최신 raster, 실제 자동완성 UI/삽입, 전체 native shutdown은 미완료입니다. Snippets1/4(25%)·M8 363/433(83.83%)·최종0/8·ETA 산정 보류를 유지하며 전체 M8 완료 전 Git 작업은 없습니다.
