import { readdirSync, readFileSync } from 'node:fs'
import { join, relative, sep } from 'node:path'
import { describe, expect, test } from 'bun:test'
import { COLOR_NAMESPACES } from '@shared/lib/theme-convert/ui-token-vocabulary'
import { toCssVariableName } from '@shared/lib/theme-variables'

const SRC_DIR = join(import.meta.dir, '../..')
const GLOBAL_CSS_PATH = join(import.meta.dir, 'global.css')

const DEFINITION_PATH_PREFIXES = ['shared/lib/theme-convert/', 'entities/theme/theme-tokens.ts']

const CONSUMER_EXEMPTIONS: Record<string, string> = {
    'terminal.commandBlockBorder': 'OSC133 블록 테두리 데코레이션이 아직 없다. terminal-view 는 블록 성공/실패를 statusIndicator 색으로만 그린다.',
    'graph.refTag':
        '커밋 그래프가 ref 배지를 종류와 무관하게 전부 graph.refBranch 로 그린다(widgets/git-panel/commit-graph.tsx). 태그를 따로 그리게 되면 배선하고 이 예외를 지운다.',
    'graph.refHead': 'graph.refTag 와 같은 이유 — ref 배지가 HEAD 를 구분하지 않는다.',
    'popover.itemHover':
        '팝오버는 자유 콘텐츠만 담고 hover 대상 항목 목록이 없다. 드롭다운·컨텍스트메뉴는 menu.itemHover 를, 명령 팔레트는 list.activeBackground 를 쓴다.',
    'tooltip.itemHover': '툴팁이 한 줄 텍스트뿐이라 hover 대상 항목이 없다. 툴팁에 항목 목록이 생기면 배선하고 이 예외를 지운다.',
    'tooltip.separator': '툴팁이 한 줄 텍스트뿐이라 구분선이 없다. 툴팁에 구획이 생기면 배선하고 이 예외를 지운다.',
    'input.focusBorder':
        '입력 포커스 테두리를 전부 app.focusBorder 로 그린다(focus:border-app-focus-border). 입력 전용 포커스 색을 분리하게 되면 배선하고 이 예외를 지운다.',
}

const collectSourceFiles = (dir: string): string[] =>
    readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
        const fullPath = join(dir, entry.name)
        if (entry.isDirectory()) return collectSourceFiles(fullPath)
        if (!/\.tsx?$/.test(entry.name) || /\.test\.tsx?$/.test(entry.name)) return []
        const relativePath = relative(SRC_DIR, fullPath).split(sep).join('/')
        if (DEFINITION_PATH_PREFIXES.some((prefix) => relativePath.startsWith(prefix))) return []
        return [fullPath]
    })

const sources = collectSourceFiles(SRC_DIR)
    .map((file) => readFileSync(file, 'utf-8'))
    .join('\n')
const globalCss = readFileSync(GLOBAL_CSS_PATH, 'utf-8')

const TAIDE_VARIABLE_PREFIX = '--taide-'
const TAILWIND_COLOR_KEY_PREFIX = '--color-'

/**
 * A `--x: var(--y)` declaration — the shape every bridge in `global.css` has, both the shadcn aliases
 * in `:root` and the Tailwind color keys in `@theme inline`. Classifying by line shape rather than by
 * enclosing block needs no brace counting and cannot drift when a block moves: a rule that actually
 * paints assigns a real CSS property (`background-color`, `box-shadow`), never a custom property from
 * a bare `var()`.
 */
const CSS_BRIDGE_LINE = /^\s*(--[a-z0-9-]+):\s*var\((--[a-z0-9-]+)\)/

const cssLines = globalCss.split('\n')
const cssBridges = cssLines.flatMap((line) => {
    const matched = CSS_BRIDGE_LINE.exec(line)
    return matched ? [{ name: matched[1], target: matched[2] }] : []
})

/** `global.css` with every bridge line dropped, so what remains is the rules that paint a surface. */
const paintingCss = cssLines.filter((line) => !CSS_BRIDGE_LINE.test(line)).join('\n')

const aliasTargets = new Map(cssBridges.filter(({ name }) => !name.startsWith(TAILWIND_COLOR_KEY_PREFIX)).map(({ name, target }) => [name, target]))

/**
 * Every Tailwind color key paired with the theme variable it ultimately reads. `@theme inline`
 * reaches a variable either directly (`--color-menu-background: var(--taide-menu-background)`) or
 * through a shadcn alias declared in `:root` (`--color-primary: var(--primary)` together with
 * `--primary: var(--taide-button-primary-background)`), so the second hop is resolved here — without
 * it every shadcn-bridged token would look utility-less.
 */
const utilityBridges = cssBridges
    .filter(({ name }) => name.startsWith(TAILWIND_COLOR_KEY_PREFIX))
    .map(({ name, target }) => ({
        utility: name.slice(TAILWIND_COLOR_KEY_PREFIX.length),
        variable: target.startsWith(TAIDE_VARIABLE_PREFIX) ? target : (aliasTargets.get(target) ?? ''),
    }))
    .filter(({ variable }) => variable.startsWith(TAIDE_VARIABLE_PREFIX))

const utilityNamesFor = (variable: string) => utilityBridges.filter((bridge) => bridge.variable === variable).map(({ utility }) => utility)

/** Tailwind utility families that take a color key, as they appear in `className` strings. */
const COLOR_UTILITY_PREFIXES = [
    'border-t',
    'border-r',
    'border-b',
    'border-l',
    'border-x',
    'border-y',
    'border-s',
    'border-e',
    'border',
    'bg',
    'text',
    'ring-offset',
    'ring',
    'outline',
    'fill',
    'stroke',
    'decoration',
    'caret',
    'accent',
    'shadow',
    'divide',
    'placeholder',
    'from',
    'via',
    'to',
]

/**
 * Whether component code writes any utility generated from a color key. The trailing guard stops a
 * shorter key from being found inside a longer one's class (`bg-graph-lane1` inside
 * `bg-graph-lane12`) while still allowing the opacity modifier (`bg-app-accent/20`) and variants
 * (`focus:bg-menu-item-hover`).
 */
const usesUtilityClass = (utility: string) => new RegExp(`\\b(?:${COLOR_UTILITY_PREFIXES.join('|')})-${utility}(?![a-zA-Z0-9-])`).test(sources)

/**
 * Theme variables whose full name is assembled at runtime — `commit-graph.tsx` picks a lane color
 * with `var(--taide-graph-lane${n})`, so `graph.lane2` appears nowhere as a literal even though the
 * graph paints with it.
 */
const composedVariablePrefixes = [...sources.matchAll(/var\((--taide-[a-z0-9-]*)\$\{/g)].map(([, prefix]) => prefix)

const allTokens = COLOR_NAMESPACES.flatMap((namespace) => namespace.tokens.map((token) => `${namespace.id}.${token}`))

const hasDirectReference = (token: string) => {
    const reference = `var(${toCssVariableName(token)})`
    return paintingCss.includes(reference) || sources.includes(reference) || sources.includes(token)
}

const hasComposedReference = (token: string) => composedVariablePrefixes.some((prefix) => toCssVariableName(token).startsWith(prefix))

const hasUtilityConsumer = (token: string) => utilityNamesFor(toCssVariableName(token)).some(usesUtilityClass)

/**
 * A token counts as consumed when something actually paints with it: a `var()` reference in a
 * painting CSS rule (`@layer base`, `@utility`), a `var()` reference in component code — literal or
 * runtime-composed — the token id itself in a Monaco/xterm color map, or a Tailwind utility generated
 * from the token's color key appearing in a `className`. A bridge line on its own is not
 * consumption: republishing `--taide-popover-item-hover` as `--color-popover-item-hover` says nothing
 * about whether any surface uses it. The vocabulary and the converter's own mapping table are
 * excluded as source files — they define tokens, so counting them would make every token look wired
 * even when nothing paints with it.
 */
const isConsumed = (token: string) => hasDirectReference(token) || hasComposedReference(token) || hasUtilityConsumer(token)

describe('UI 토큰 소비자 존재 게이트', () => {
    test('검사 입력이 비어 있지 않다', () => {
        expect(allTokens.length).toBeGreaterThan(0)
        expect(sources.length).toBeGreaterThan(0)
        expect(globalCss.length).toBeGreaterThan(0)
        expect(paintingCss.length).toBeGreaterThan(0)
        expect(utilityBridges.length).toBeGreaterThan(0)
        expect(composedVariablePrefixes.length).toBeGreaterThan(0)
    })

    test('vocabulary colors 토큰은 전부 소비처가 있거나 예외 목록에 등재돼 있다', () => {
        const unwired = allTokens.filter((token) => !isConsumed(token) && !(token in CONSUMER_EXEMPTIONS))

        expect(unwired).toEqual([])
    })

    test('예외 목록은 실제로 소비처가 없는 토큰만 담는다', () => {
        const stale = Object.keys(CONSUMER_EXEMPTIONS).filter((token) => isConsumed(token))

        expect(stale).toEqual([])
    })

    test('예외 목록은 vocabulary 에 실재하는 토큰만 담는다', () => {
        const unknown = Object.keys(CONSUMER_EXEMPTIONS).filter((token) => !allTokens.includes(token))

        expect(unknown).toEqual([])
    })

    test('유틸리티 클래스 실사용만으로도 소비처로 인정한다', () => {
        const utilityOnly = allTokens.filter((token) => !hasDirectReference(token) && !hasComposedReference(token) && hasUtilityConsumer(token))

        expect(utilityOnly.length).toBeGreaterThan(0)
    })

    test('브릿지 줄만 있고 칠하는 곳이 없는 토큰은 소비로 치지 않는다', () => {
        const bridgedExemptions = Object.keys(CONSUMER_EXEMPTIONS).filter((token) => utilityNamesFor(toCssVariableName(token)).length > 0)

        expect(bridgedExemptions.length).toBeGreaterThan(0)
        expect(bridgedExemptions.filter(isConsumed)).toEqual([])
    })
})

/**
 * Tailwind's built-in achromatic color utilities — the raw colors a component can reach for instead
 * of a theme token. They are not themeable: `text-white` stays white in every theme, so one of these
 * in a component is a hole in the theme system unless it is there on purpose.
 */
const RAW_COLOR_CLASS =
    /(?:^|[\s'"`])((?:bg|text|border|ring|fill|stroke|from|via|to|divide|outline|decoration|placeholder|caret|accent|shadow)-(?:white|black)(?:\/\d+)?)\b/g

/**
 * The raw color usages that are deliberate, keyed by `<path>: <class>` with the reason. Anything else
 * must use a token — `docs/theme-system.md` §3 records the same list as the user-facing policy.
 */
const RAW_COLOR_EXEMPTIONS: Record<string, string> = {
    'shared/ui/button.tsx: text-white':
        '파괴적 동작 버튼의 라벨. 배경이 어떤 테마에서도 붉은 statusIndicator.error 라 라벨은 테마와 무관하게 흰색이어야 한다. 토큰(button.primaryForeground)으로 바꿨더니 47종 중 19종이 3:1 미만으로 떨어졌다(d-61 검토 G-1). 계측은 component-contrast-pairs.ts 의 FIXED_FOREGROUND_CONTRAST_PAIRS 가 자문으로 남긴다.',
}

const rawColorUsages = collectSourceFiles(SRC_DIR).flatMap((file) => {
    const relativePath = relative(SRC_DIR, file).split(sep).join('/')
    return [...readFileSync(file, 'utf-8').matchAll(RAW_COLOR_CLASS)].map((matched) => `${relativePath}: ${matched[1]}`)
})

describe('컴포넌트 raw 색 게이트', () => {
    test('검사 입력이 비어 있지 않다', () => {
        expect(collectSourceFiles(SRC_DIR).length).toBeGreaterThan(0)
    })

    test('raw 색 유틸리티는 예외 목록에 등재된 것만 쓴다', () => {
        const unregistered = [...new Set(rawColorUsages)].filter((usage) => !(usage in RAW_COLOR_EXEMPTIONS))

        expect(unregistered).toEqual([])
    })

    test('예외 목록은 실제로 쓰이는 것만 담는다', () => {
        const stale = Object.keys(RAW_COLOR_EXEMPTIONS).filter((usage) => !rawColorUsages.includes(usage))

        expect(stale).toEqual([])
    })
})
