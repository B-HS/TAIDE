# ADR-0003 — UI 컴포넌트: shadcn/ui 기본 + Radix Primitives fallback

- 상태: 승인 (사용자 확정, 2026-08-06)
- 관련: `docs/research/tailwind-shadcn.md`, `docs/theme-system.md`

## 맥락

IDE 셸 UI(사이드바, 탭, 트리, 컨텍스트 메뉴, 모달, 툴팁, 드롭다운 등)에 쓸 컴포넌트 전략.
후보: shadcn/ui, Radix Primitives 직접, Base UI, React Aria Components.

## 결정

1. **shadcn/ui 를 기본**으로 한다. 컴포넌트를 코드로 소유(vendored)하고 TAIDE 테마 시스템에 맞게 어레인지한다.
2. shadcn 에 없는 컴포넌트는 **Radix Primitives 로 직접 제작**한다(fallback).
3. 누락 방지를 위해 **shadcn 레지스트리의 전체 컴포넌트 목록을 확보**해 두고
   (`docs/research/tailwind-shadcn.md`), 새 UI 필요 시 이 목록 → Radix 순으로 조사 후 제작한다.

## 근거

- shadcn 은 Radix 기반 + Tailwind v4 공식 지원. 코드를 직접 소유하므로 IDE 밀도 높은 UI 로
  자유롭게 개조 가능하고, 외부 스타일 의존이 없다.
- fallback 을 Radix 로 고정하면 shadcn 과 같은 기반이라 접근성·동작·스타일 방식이 일관된다.

## 기각한 대안

- **Base UI**: 신생이라 컴포넌트 수·생태계가 아직 작다.
- **React Aria Components**: 접근성은 최강이나 DOM 구조·API 가 독특해 고밀도 IDE UI 커스텀 비용이 크다.

## 결과

- shadcn 컴포넌트는 `src/shared/ui/`(FSD shared)로 들여오고, 수정 이력은 TAIDE 가 소유한다
  (업스트림 업데이트 자동 추종 안 함).
- cn(clsx + tailwind-merge)·CVA 패턴은 프론트 컨벤션(frontend.md §8.1) 그대로 사용.
