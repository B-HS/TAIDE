# Tailwind CSS v4 + shadcn/ui 리서치 (TAIDE)

조사 기준일: 2026-08-06. 아래 버전은 npm registry `latest` 태그와 공식 블로그/문서에서 직접 확인한 값이다.

## 버전 확정 (2026-08 기준)

| 패키지 | 최신 버전 | 확인 방법 |
|---|---|---|
| `tailwindcss` | **4.3.3** | registry.npmjs.org/tailwindcss/latest |
| `@tailwindcss/vite` | **4.3.3** | registry.npmjs.org (tailwindcss 와 버전 동기) |
| `shadcn` (CLI) | **4.16.1** | registry.npmjs.org/shadcn/latest |
| `radix-ui` (통합 패키지) | **1.6.7** | registry.npmjs.org/radix-ui/latest |
| `tw-animate-css` | **1.4.0** | registry.npmjs.org |
| `class-variance-authority` | **0.7.1** | registry.npmjs.org |
| `clsx` | **2.1.1** | registry.npmjs.org |
| `tailwind-merge` | **3.6.0** | registry.npmjs.org |
| `lucide-react` | **1.28.0** | registry.npmjs.org |

### Tailwind v4 버전별 주요 변경

- **v4.0** (2025-01): 엔진 재작성(Oxide), CSS-first 설정. `tailwind.config.js` 대신 `@import "tailwindcss"` + `@theme`. 자동 콘텐츠 감지, `@source` / `@utility` / `@custom-variant` 도입. 색상은 oklch 기본.
- **v4.1** (2025-04): `text-shadow-2xs` ~ `text-shadow-lg`(+색상/불투명도 모디파이어), `mask-*` 유틸(선형/방사형/원뿔/이미지 마스크), 컬러 `drop-shadow-<color>`, `pointer-*` / `any-pointer-*`, `overflow-wrap` 유틸, 구형 브라우저 호환 개선.
- **v4.2**: 신규 중성 팔레트 `mauve` / `olive` / `mist` / `taupe`(50~950). 논리 속성 유틸 대폭 확장(`mbs-*` `mbe-*` `pbs-*` `pbe-*` `border-bs` `border-be`, `block-*` `inline-*` `min-block-*` `max-inline-*`, `inset-s-*` `inset-e-*` `inset-bs-*` `inset-be-*`). `font-features-[...]`. `@tailwindcss/webpack` 로더(PostCSS 대비 약 2.17배 빠름).
- **v4.3** (2026-05-08 블로그 공개): **스크롤바 유틸**(`scrollbar-auto|thin|none`, `scrollbar-thumb-*`, `scrollbar-track-*`, `scrollbar-gutter-auto|stable|both`), `@container-size`(블록축 컨테이너 쿼리 단위 `cqb`/`cqh`), `zoom-*`, `tab-*`, `@variant` 의 스택/컴파운드 지원(`@variant hover:focus`, `@variant hover, focus`), functional utility 기본값 `--default(...)`, `@tailwindcss/vite`·postcss 의 플러그인/import 처리 개선, 동일 이름 `@utility` 다중 정의 허용.

> 사용자가 언급한 "4.3 출시"는 사실이며, 패치 최신은 4.3.3 이다. v4.4 이상은 2026-08-06 기준 **미확인(존재하지 않음)**.

---

## 핵심 API·사용법

### 1. Vite 플러그인 설정

```bash
npm install tailwindcss@4.3.3 @tailwindcss/vite@4.3.3
```

```ts
// vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'

export default defineConfig({
    plugins: [react(), tailwindcss()],
    resolve: { alias: { '@': path.resolve(__dirname, './src') } },
})
```

```css
/* src/styles/global.css — 엔트리에서 import */
@import 'tailwindcss';
```

- PostCSS 설정 파일 · `tailwind.config.js` 불필요. Vite 플러그인이 변환 파이프라인에 직접 붙는다.
- Tauri 처럼 `index.html` 밖(예: Rust 템플릿, 별도 패키지)에 클래스가 있으면 `@source` 로 명시한다.

### 2. 디렉티브 · 함수 전체

```css
@import 'tailwindcss';

/* 콘텐츠 소스 추가/제외/세이프리스트 */
@source '../../packages/ui/src';
@source not '../legacy';
@source inline('bg-red-500 underline');

/* 디자인 토큰 → 유틸리티 자동 생성 */
@theme {
    --color-editor-bg: oklch(0.18 0.01 260);
    --font-mono: 'JetBrains Mono', ui-monospace, monospace;
    --breakpoint-3xl: 120rem;
    --radius-panel: 10px;
    --animate-blink: blink 1s step-end infinite;
}

/* 다른 변수를 참조할 때는 inline (해석된 값이 유틸에 들어감) */
@theme inline {
    --color-background: var(--background);
}

/* 사용 여부와 무관하게 모든 변수 출력 (런타임 JS 에서 읽을 때 유용) */
@theme static {
    --color-accent-1: var(--color-sky-500);
}

/* 네임스페이스 통째로 리셋 */
@theme {
    --color-*: initial;
    --color-white: #fff;
}

/* 커스텀 유틸 (variant 와 조합 가능) */
@utility scrollbar-hidden {
    scrollbar-width: none;
}
@utility tab-* {
    tab-size: --value(integer, --default(4));
}

/* 커스텀 variant */
@custom-variant dark (&:where(.dark, .dark *));
@custom-variant panel-active (&:where([data-state='active'] *));

/* CSS 안에서 variant 적용 (v4.3 부터 스택·컴파운드 지원) */
.button {
    background: var(--color-sky-500);
    @variant hover:focus {
        background: var(--color-sky-700);
    }
    @variant hover, focus {
        outline: 2px solid;
    }
}

/* 함수 */
.x {
    color: --alpha(var(--color-lime-300) / 50%); /* → color-mix(in oklab, ...) */
    margin: --spacing(4); /* → calc(var(--spacing) * 4) */
}

/* 컴포넌트 <style> 블록에서 @apply 쓸 때 (CSS 중복 출력 방지) */
@reference 'tailwindcss';
```

`@theme` 네임스페이스 → 생성 유틸: `--color-*`(bg/text/border), `--font-*`, `--text-*`(font-size), `--font-weight-*`, `--tracking-*`, `--leading-*`, `--breakpoint-*`(반응형 variant), `--spacing-*`, `--radius-*`, `--shadow-*`, `--animate-*`.

레거시 호환: `@config "../tailwind.config.js"`, `@plugin "@tailwindcss/typography"`.

### 3. v4.1~4.3 신규 유틸 실사용

```html
<!-- 텍스트 그림자 (4.1) -->
<h1 class="text-shadow-lg text-shadow-sky-300/50">Title</h1>

<!-- 마스크 (4.1) -->
<div class="mask-b-from-50% mask-radial-at-center"></div>

<!-- 스크롤바 (4.3) — 에디터/파일트리에 유용 -->
<div class="scrollbar-thin scrollbar-thumb-neutral-600 scrollbar-track-transparent scrollbar-gutter-stable overflow-auto"></div>

<!-- 탭 폭 (4.3) — 코드 뷰어 -->
<pre class="tab-4">...</pre>
<pre class="tab-(--editor-tab-size)">...</pre>

<!-- 줌 (4.3) — 에디터 확대/축소 -->
<div class="zoom-(--zoom-level)">...</div>

<!-- 블록축 컨테이너 쿼리 (4.3) -->
<div class="@container-size/panel"><div class="h-[50cqb]"></div></div>
```

### 4. 런타임 테마 전환 (다크/라이트 + 사용자 커스텀 색상)

핵심 원칙: **`@theme inline` 에 시맨틱 토큰만 매핑하고, 실제 값은 일반 CSS 변수(`:root` / `.dark` / `[data-theme=...]`)로 두어 런타임에 덮어쓴다.** `@theme` 안의 값을 직접 바꾸려 하면 빌드 타임 값이라 런타임 전환이 안 된다.

```css
/* src/styles/global.css */
@import 'tailwindcss';
@import 'tw-animate-css';

@custom-variant dark (&:where(.dark, .dark *));

:root {
    --radius: 0.625rem;
    --background: oklch(1 0 0);
    --foreground: oklch(0.145 0 0);
    --primary: oklch(0.205 0 0);
    --primary-foreground: oklch(0.985 0 0);
    --border: oklch(0.922 0 0);
}

.dark {
    --background: oklch(0.145 0 0);
    --foreground: oklch(0.985 0 0);
    --primary: oklch(0.922 0 0);
    --primary-foreground: oklch(0.205 0 0);
    --border: oklch(1 0 0 / 10%);
}

/* 사용자 커스텀 테마: 같은 변수만 재정의하면 전 컴포넌트에 즉시 반영 */
[data-theme='taide-ocean'] {
    --primary: oklch(0.62 0.14 235);
    --primary-foreground: oklch(0.99 0 0);
}

@theme inline {
    --color-background: var(--background);
    --color-foreground: var(--foreground);
    --color-primary: var(--primary);
    --color-primary-foreground: var(--primary-foreground);
    --color-border: var(--border);
    --radius-sm: calc(var(--radius) - 4px);
    --radius-md: calc(var(--radius) - 2px);
    --radius-lg: var(--radius);
    --radius-xl: calc(var(--radius) + 4px);
}

@layer base {
    * {
        @apply border-border outline-ring/50;
    }
    body {
        @apply bg-background text-foreground;
    }
}
```

런타임 전환 코드:

```ts
// shared/lib/theme.ts
const THEME_STORAGE_KEY = 'taide-theme'

export const applyColorScheme = (scheme: 'light' | 'dark' | 'system') => {
    const isDark = scheme === 'dark' || (scheme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
    document.documentElement.classList.toggle('dark', isDark)
    if (scheme === 'system') localStorage.removeItem(THEME_STORAGE_KEY)
    else localStorage.setItem(THEME_STORAGE_KEY, scheme)
}

export const applyCustomPalette = (tokens: Record<string, string>) => {
    for (const [name, value] of Object.entries(tokens)) document.documentElement.style.setProperty(`--${name}`, value)
}
```

FOUC 방지를 위해 번들 로드 전에 `index.html` 인라인 스크립트로 `classList.toggle('dark', ...)` 를 먼저 실행한다.

JS 에서 현재 토큰 값 읽기:

```ts
const value = getComputedStyle(document.documentElement).getPropertyValue('--color-primary')
```

### 5. shadcn/ui 설치 · components.json

shadcn CLI 는 Tailwind v4 를 **정식 지원**한다(전 컴포넌트가 v4 + React 19 기준으로 갱신됨: `forwardRef` 제거, `data-slot` 속성 기반 스타일링, HSL → OKLCH 전환, `size-*` 유틸 사용).

```bash
npx shadcn@latest init          # 대화형 (Vite / React Router / Next / Astro 템플릿 지원)
npx shadcn@latest add button dialog command
npx shadcn@latest add --all
```

`components.json` (Vite + React + TS + Tailwind v4 기준):

```json
{
    "$schema": "https://ui.shadcn.com/schema.json",
    "style": "new-york",
    "rsc": false,
    "tsx": true,
    "tailwind": {
        "config": "",
        "css": "src/styles/global.css",
        "baseColor": "neutral",
        "cssVariables": true,
        "prefix": ""
    },
    "iconLibrary": "lucide",
    "aliases": {
        "components": "@/components",
        "utils": "@/shared/lib/utils",
        "ui": "@/shared/ui",
        "lib": "@/shared/lib",
        "hooks": "@/shared/hooks"
    },
    "registries": {
        "@acme": {
            "url": "https://acme.com/r/{name}.json",
            "headers": { "Authorization": "Bearer ${ACME_TOKEN}" }
        }
    }
}
```

키 요약: `style`(`new-york` 만 유효, `default` 는 deprecated) · `tailwind.config`(**v4 에서는 빈 문자열**) · `tailwind.css`(전역 CSS 경로) · `baseColor`(neutral/gray/zinc/stone/slate, 초기화 후 변경 불가) · `cssVariables`(true 권장, false 면 유틸 인라인) · `rsc`(Tauri/Vite 는 false) · `tsx` · `iconLibrary`(lucide/radix/tabler) · `aliases`(tsconfig paths 또는 package.json#imports 와 일치해야 함) · `registries`(네임스페이스 `@name` 으로 서드파티/사내 레지스트리).

`init` 시 전역 CSS 에 `@import "shadcn/tailwind.css"` 가 추가된다(공용 variant `data-open:` `data-closed:` 등과 accordion 애니메이션 제공). 의존성으로 `class-variance-authority`, `lucide-react`, **`radix-ui`(통합 패키지)** 가, devDependency 로 `tw-animate-css`, `shadcn` 이 설치된다.

CLI 명령: `init`/`create`, `add`, `apply`(프리셋/테마/폰트 적용), `view`, `search`/`list`, `preset`(decode/resolve/url/open), `build`(registry.json → 배포용 JSON), `docs`, `migrate`(아이콘 라이브러리 교체, RTL 활성화, Radix import 업그레이드), `eject`(shadcn Tailwind 유틸을 프로젝트에 인라인 — **되돌릴 수 없음**), `info`.

### 6. shadcn/ui 레지스트리 컴포넌트 전체 목록

`https://ui.shadcn.com/r/index.json` 기준 `registry:ui` 항목 **63개** (알파벳순).

| # | 이름 | 용도(1줄) |
|---:|---|---|
| 1 | accordion | 접었다 펴는 수직 섹션 목록 |
| 2 | alert | 인라인 상태·경고 메시지 배너 |
| 3 | alert-dialog | 사용자 확인을 강제하는 모달(파괴적 액션용) |
| 4 | aspect-ratio | 자식 요소를 고정 종횡비로 유지 |
| 5 | attachment | 메시지에 첨부된 파일(PDF·이미지 등) 렌더 |
| 6 | avatar | 사용자 프로필 이미지 + 폴백 이니셜 |
| 7 | badge | 상태·카테고리 라벨 칩 |
| 8 | breadcrumb | 계층 경로 내비게이션 |
| 9 | bubble | 대화형 메시지 말풍선(변형·정렬·그룹·리액션·접기) |
| 10 | button | 기본 버튼(variant/size CVA 기반) |
| 11 | button-group | 버튼을 붙여 하나의 그룹으로 배치 |
| 12 | calendar | 날짜 그리드(react-day-picker 기반) |
| 13 | card | 헤더/콘텐츠/푸터 구조의 표면 컨테이너 |
| 14 | carousel | 슬라이드 캐러셀(embla 기반) |
| 15 | chart | Recharts 래퍼 + 테마 연동 차트 컨테이너 |
| 16 | checkbox | 체크박스 입력 |
| 17 | collapsible | 단일 영역 열기/닫기 |
| 18 | combobox | 검색 가능한 선택 입력(Popover + Command 조합) |
| 19 | command | 커맨드 팔레트/명령 메뉴(cmdk 기반) |
| 20 | context-menu | 우클릭 컨텍스트 메뉴 |
| 21 | dialog | 일반 모달 다이얼로그 |
| 22 | direction | 앱 텍스트 방향(ltr/rtl) 지정 Provider |
| 23 | drawer | 모바일 친화 바텀/사이드 시트(vaul 기반) |
| 24 | dropdown-menu | 트리거 기반 드롭다운 메뉴 |
| 25 | empty | 데이터 없음 상태 플레이스홀더 |
| 26 | field | 라벨·설명·에러를 묶는 폼 필드 레이아웃 |
| 27 | form | react-hook-form + zod 연동 폼 래퍼 |
| 28 | hover-card | 호버 시 뜨는 미리보기 카드 |
| 29 | input | 단일 행 텍스트 입력 |
| 30 | input-group | 입력에 접두/접미 아이콘·버튼을 붙인 그룹 |
| 31 | input-otp | OTP/인증코드 분할 입력 |
| 32 | item | 아이콘·타이틀·설명·액션으로 구성된 범용 리스트 아이템 |
| 33 | kbd | 키보드 단축키 표기 |
| 34 | label | 폼 접근성 라벨 |
| 35 | marker | 대화 스레드의 인라인 마커(상태 업데이트·시스템 노트·구분선) |
| 36 | menubar | 데스크톱 앱 스타일 상단 메뉴바 |
| 37 | message | 대화 내 개별 메시지(아바타·헤더·푸터·정렬) |
| 38 | message-scroller | 메시지 목록용 스크롤 컨테이너(자동 하단 고정) |
| 39 | native-select | 네이티브 `<select>` 를 스타일링한 버전 |
| 40 | navigation-menu | 드롭다운 패널이 있는 사이트 내비게이션 |
| 41 | pagination | 페이지 이동 컨트롤 |
| 42 | popover | 트리거 기준 부유 패널 |
| 43 | progress | 진행률 바 |
| 44 | questionnaire | 단일선택·다중선택·자유입력·건너뛰기 지원 다단계 설문 |
| 45 | radio-group | 라디오 버튼 그룹 |
| 46 | resizable | 드래그로 크기 조절되는 패널 분할(react-resizable-panels) |
| 47 | scroll-area | 커스텀 스크롤바 영역 |
| 48 | select | 커스텀 드롭다운 선택 |
| 49 | separator | 수평/수직 구분선 |
| 50 | sheet | 화면 가장자리에서 나오는 패널 |
| 51 | sidebar | 접힘·모바일 대응 전체 사이드바 시스템 |
| 52 | skeleton | 로딩 자리표시 셰이프 |
| 53 | slider | 값 범위 슬라이더 |
| 54 | sonner | sonner 기반 토스트 알림(권장 토스트) |
| 55 | spinner | 로딩 스피너 |
| 56 | switch | on/off 토글 스위치 |
| 57 | table | 정적 테이블 마크업 세트 |
| 58 | tabs | 탭 전환 UI |
| 59 | textarea | 여러 줄 텍스트 입력 |
| 60 | toast | 토스트 알림(구형, 신규는 sonner 권장) |
| 61 | toggle | 눌림 상태를 갖는 토글 버튼 |
| 62 | toggle-group | 토글 버튼 그룹(단일/다중 선택) |
| 63 | tooltip | 호버/포커스 툴팁 |

문서 사이드바에만 있고 레지스트리 UI 항목이 아닌 **조합 가이드**(코드는 위 컴포넌트 조합으로 작성): `data-table`(TanStack Table + table), `date-picker`(popover + calendar), `typography`(타이포 스타일 가이드).

`accordion` ~ `tooltip` 외에 `registry:lib`/`registry:hook` 로 `utils`, `use-mobile` 등이 함께 설치된다.

### 7. CVA + cn 유틸 최신 패턴

```ts
// src/shared/lib/utils.ts
import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export const cn = (...inputs: ClassValue[]) => twMerge(clsx(inputs))
```

```tsx
// src/shared/ui/button.tsx — shadcn v4 스타일(forwardRef 없음, data-slot 사용)
import { Slot } from 'radix-ui'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/shared/lib/utils'

const buttonVariants = cva(
    "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
    {
        variants: {
            variant: {
                default: 'bg-primary text-primary-foreground shadow-xs hover:bg-primary/90',
                destructive: 'bg-destructive text-white shadow-xs hover:bg-destructive/90',
                outline: 'border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground',
                secondary: 'bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80',
                ghost: 'hover:bg-accent hover:text-accent-foreground',
                link: 'text-primary underline-offset-4 hover:underline',
            },
            size: {
                default: 'h-9 px-4 py-2 has-[>svg]:px-3',
                sm: 'h-8 rounded-md gap-1.5 px-3',
                lg: 'h-10 rounded-md px-6',
                icon: 'size-9',
            },
        },
        defaultVariants: { variant: 'default', size: 'default' },
    },
)

type ButtonProps = React.ComponentProps<'button'> & VariantProps<typeof buttonVariants> & { asChild?: boolean }

export const Button = ({ className, variant, size, asChild = false, ...props }: ButtonProps) => {
    const Comp = asChild ? Slot.Root : 'button'
    return <Comp data-slot='button' className={cn(buttonVariants({ variant, size, className }))} {...props} />
}

export { buttonVariants }
```

- 타입은 `VariantProps<typeof xVariants>` 로 **유도**한다(수기 union 금지).
- `tailwind-merge` 3.x 는 Tailwind v4 토큰 체계에 맞춰져 있다. v4 프로젝트에서 2.x 를 쓰면 병합 규칙이 어긋난다.
- 커스텀 유틸(`@utility`)을 만들었고 그것이 충돌 그룹을 가진다면 `extendTailwindMerge` 로 등록해야 정상 병합된다.

### 8. tw-animate-css (1.4.0)

`tailwindcss-animate` 는 deprecated, 후속이 `tw-animate-css` 다. shadcn 신규 프로젝트에 기본 설치된다.

```bash
npm i -D tw-animate-css
```

```css
@import 'tailwindcss';
@import 'tw-animate-css';
```

Radix `data-state` 기반 enter/exit 애니메이션 유틸을 제공한다.

```html
<div class="animate-in fade-in-0 zoom-in-95 slide-in-from-bottom-2 duration-200
            data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95">
    ...
</div>
```

주 유틸군: `animate-in` / `animate-out`, `fade-in-*` / `fade-out-*`, `zoom-in-*` / `zoom-out-*`, `slide-in-from-{top,bottom,left,right}-*`, `spin-in-*`, `duration-*`, `delay-*`, `ease-*`, `fill-mode-*`.

### 9. Radix Primitives (radix-ui 1.6.7)

shadcn 최신은 개별 `@radix-ui/react-*` 대신 **통합 `radix-ui` 패키지**를 쓴다.

```tsx
import { Dialog, DropdownMenu, Tooltip } from 'radix-ui'

<Dialog.Root>
    <Dialog.Trigger asChild>...</Dialog.Trigger>
    <Dialog.Portal>
        <Dialog.Overlay className='fixed inset-0 bg-black/50' />
        <Dialog.Content className='fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2'>...</Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>
```

기존 프로젝트는 `npx shadcn@latest migrate radix` 로 import 를 일괄 전환할 수 있다.

**컴포넌트 (문서 기준 30개)**: Accordion, Alert Dialog, Aspect Ratio, Avatar, Checkbox, Collapsible, Context Menu, Dialog, Dropdown Menu, Form, Hover Card, Label, Menubar, Navigation Menu, One-Time Password Field, Password Toggle Field, Popover, Progress, Radio Group, Scroll Area, Select, Separator, Slider, Switch, Tabs, Toast, Toggle, Toggle Group, Toolbar, Tooltip.

**유틸리티**: Accessible Icon, Direction Provider, Portal, Slot, Visually Hidden.

**내부/저수준 패키지**(레포 `packages/react` 에 존재, 직접 만들 때 참고): `announce`, `arrow`, `collection`, `compose-refs`, `context`, `dismissable-layer`, `focus-guards`, `focus-scope`, `id`, `menu`, `popper`, `presence`, `primitive`, `roving-focus`, 그리고 훅 `use-callback-ref`, `use-controllable-state`, `use-effect-event`, `use-escape-keydown`, `use-is-hydrated`, `use-layout-effect`, `use-previous`, `use-rect`, `use-size`.

> shadcn 에 없는 컴포넌트를 직접 만들 때: 위 저수준 패키지(`dismissable-layer` + `focus-scope` + `presence` + `portal`)를 조합하면 접근성·포커스 트랩·애니메이션 종료 처리를 재구현하지 않아도 된다.

---

## TAIDE 적용 가이드

1. **설치**
   ```bash
   npm i tailwindcss@4.3.3 @tailwindcss/vite@4.3.3
   npm i -D tw-animate-css shadcn
   npx shadcn@latest init   # tailwind.config 은 빈 문자열, rsc:false, baseColor:neutral
   ```
2. **FSD 배치**: `components.json` 의 `aliases.ui` 를 `@/shared/ui` 로 지정해 shadcn 컴포넌트가 `shared` 레이어에 떨어지게 한다. `utils` 는 `@/shared/lib/utils`. 생성된 컴포넌트는 순수 UI 이므로 FSD 상 `shared/ui` 가 맞다(features 아님).
3. **IDE 특화 컴포넌트 우선순위**: `resizable`(에디터/사이드바/터미널 3분할), `sidebar`(파일 트리 셸), `command`(커맨드 팔레트 — Ctrl+Shift+P), `context-menu`(파일 우클릭), `tabs`(에디터 탭), `scroll-area`, `tooltip`, `kbd`(단축키 표기), `sonner`(알림), `dialog`/`alert-dialog`(저장 확인), `empty`(빈 에디터 상태), `spinner`, `separator`, `menubar`(Tauri 커스텀 타이틀바 메뉴).
4. **테마 시스템**: 위 §4 구조 그대로. `.dark` 클래스 토글 + `[data-theme=...]` 로 사용자 커스텀 팔레트. 테마 설정은 Tauri store 에 저장하고, 앱 부팅 시 `index.html` 인라인 스크립트로 먼저 적용해 흰 화면 깜빡임을 막는다.
5. **에디터 색상 연동**: CodeMirror/Monaco 토큰 색을 `getComputedStyle` 로 읽으려면 해당 변수를 `@theme static` 또는 `:root` 에 두어 항상 출력되게 한다. `@theme`(비-static)은 사용된 유틸만 변수를 내보내므로 JS 에서 못 읽을 수 있다.
6. **v4.3 유틸 활용**: 파일 트리·에디터 스크롤은 `scrollbar-thin scrollbar-gutter-stable`, 코드 미리보기는 `tab-(--editor-tab-size)`, 에디터 줌은 `zoom-(--zoom-level)`.
7. **`@source`**: Tauri 는 `src-tauri` 에 프런트 코드가 없으므로 기본 자동 감지로 충분하다. 다만 `dist`·`target` 은 자동 제외되며, 별도 UI 패키지를 만들면 `@source '../packages/ui/src'` 를 추가한다.

---

## 함정·주의

- **`@theme` 안의 값은 빌드 타임 고정**이다. 런타임 테마 전환은 반드시 "일반 CSS 변수 → `@theme inline` 매핑" 2단 구조로 해야 한다. `@theme` 에 직접 색을 넣고 JS 로 덮어쓰면 유틸이 갱신되지 않는다.
- **`@theme inline` 없이 `var()` 를 참조하면** 캐스케이드 위치에 따라 값 해석이 어긋난다(다크 모드가 부분만 적용되는 전형적 증상).
- **`tailwind.config.js` 는 v4 에서 자동 로드되지 않는다.** 필요하면 `@config` 로 명시해야 하고, `components.json` 의 `tailwind.config` 는 빈 문자열이어야 한다.
- **`tailwindcss-animate` 를 v4 에 그대로 쓰지 말 것.** deprecated 이며 `tw-animate-css` 로 교체한다. 둘을 동시에 넣으면 클래스가 충돌한다.
- **`tailwind-merge` 버전 고정**: v4 에는 3.x. 2.x 를 쓰면 `size-*`, 새 색상 팔레트 등에서 병합이 틀어진다.
- **`cn()` 충돌 그룹**: 같은 그룹 클래스를 동시에 넘기면 뒤엣것만 남는다(예: `text-sm` + `text-muted-foreground` 는 v3 시절 문제였고 3.x 에서 개선됨 — 그래도 커스텀 유틸은 `extendTailwindMerge` 등록 필요).
- **shadcn 컴포넌트는 소스가 프로젝트에 복사된다.** 업그레이드는 자동이 아니므로 `add` 재실행 시 로컬 수정이 덮어써진다. 커스터마이즈한 파일은 커밋 diff 로 관리한다.
- **`style: "default"` 는 deprecated**, `new-york` 만 사용한다. `baseColor` 는 init 이후 변경 불가.
- **`shadcn eject` 는 되돌릴 수 없다.** 실행 전 커밋 필수.
- **Radix import 경로**: 신규는 `radix-ui` 통합 패키지. 구 `@radix-ui/react-*` 와 혼용하면 컨텍스트가 분리돼 `asChild`·포커스 관리가 깨질 수 있다.
- **`zoom-*`(CSS `zoom`)은 레이아웃에 영향**을 준다. transform scale 과 동작이 다르므로 에디터 확대에 쓸 때 좌표 계산(마우스 hit-test)을 검증해야 한다.
- **스크롤바 유틸은 브라우저 지원 편차**가 있다(WebKit `::-webkit-scrollbar` vs 표준 `scrollbar-width`/`scrollbar-color`). Tauri 는 WebView2(Windows) / WKWebView(macOS) / WebKitGTK(Linux) 로 렌더러가 달라 플랫폼별 실제 확인이 필요하다.
- **`prefers-color-scheme` 는 Tauri 에서 OS 설정을 따른다.** 앱 내 테마와 어긋나지 않도록 `@custom-variant dark` 를 클래스 기반으로 고정하는 편이 안전하다.
- **v4.3 세부 유틸의 정확한 값 스케일**(예: `zoom-*` 이 지원하는 정수 목록)은 블로그 예시 범위(`zoom-75`/`100`/`125`, 임의값 지원)까지만 확인했다. 전체 스케일은 **미확인** — 구현 시 공식 유틸 레퍼런스로 재확인할 것.
- shadcn 문서 사이드바에 표시되는 `Data Table` · `Date Picker` · `Typography` 는 **레지스트리 컴포넌트가 아니라 조합 가이드**다. `npx shadcn add data-table` 은 동작하지 않을 수 있다.

---

## 참고 링크

- Tailwind v4.3 릴리스: https://tailwindcss.com/blog/tailwindcss-v4-3
- Tailwind v4.1 릴리스: https://tailwindcss.com/blog/tailwindcss-v4-1
- Tailwind v4.0 릴리스: https://tailwindcss.com/blog/tailwindcss-v4
- 함수·디렉티브: https://tailwindcss.com/docs/functions-and-directives
- 테마 변수: https://tailwindcss.com/docs/theme
- 다크 모드: https://tailwindcss.com/docs/dark-mode
- Vite 설치: https://tailwindcss.com/docs/installation/using-vite
- shadcn Tailwind v4 안내: https://ui.shadcn.com/docs/tailwind-v4
- shadcn CLI: https://ui.shadcn.com/docs/cli
- shadcn components.json: https://ui.shadcn.com/docs/components-json
- shadcn 테마: https://ui.shadcn.com/docs/theming
- shadcn 컴포넌트 목록: https://ui.shadcn.com/docs/components
- shadcn 레지스트리 인덱스(JSON): https://ui.shadcn.com/r/index.json
- shadcn base color 토큰(JSON 예): https://ui.shadcn.com/r/colors/neutral.json
- Radix Primitives 컴포넌트: https://www.radix-ui.com/primitives/docs/components
- Radix Primitives 레포(패키지 목록): https://github.com/radix-ui/primitives/tree/main/packages/react
- CVA: https://cva.style/docs
- tailwind-merge: https://github.com/dcastil/tailwind-merge
- tw-animate-css: https://github.com/Wombosvideo/tw-animate-css
