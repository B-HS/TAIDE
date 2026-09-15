import { describe, expect, test } from 'bun:test'
import { deltaE76 } from '@shared/lib/color'
import { repairStateDistinctness, validateStateDistinctness } from '@shared/lib/theme-convert/state-distinctness'
import {
    APP_SHADOW_MIN_ALPHA,
    STATE_DISTINCTNESS_PAIRS,
    STATE_MIN_DISTINCT_DELTA_E,
    SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
} from '@shared/lib/theme-convert/state-distinctness-pairs'

/**
 * A theme that already satisfies every pair, so each test can collapse exactly one axis and read the
 * result without other pairs firing. Values are the builtin dark palette (`global.css` `:root`),
 * which is the catalog's own reference point for "these colors work together".
 */
const BASE_COLORS: Record<string, string> = {
    'app.background': '#1e1e2e',
    'app.foreground': '#cdd6f4',
    'app.focusBorder': '#89b4fa',
    'app.shadow': '#00000066',
    'appSidebar.background': '#181825',
    'appSidebar.itemHover': '#313244',
    'appSidebar.itemActive': '#45475a',
    'appSidebar.badge': '#f38ba8',
    'tabBar.background': '#181825',
    'tabBar.tabActiveBackground': '#1e1e2e',
    'tabBar.tabInactiveBackground': '#11111b',
    'tabBar.tabActiveIndicator': '#89b4fa',
    'explorer.background': '#181825',
    'explorer.itemHover': '#313244',
    'explorer.itemSelected': '#585b70',
    'explorer.itemFocused': '#45475a',
    'panel.background': '#181825',
    'panel.inputBackground': '#1e1e2e',
    'panel.inputBorder': '#45475a',
    'editor.background': '#1e1e2e',
    'editor.widgetBackground': '#181825',
    'editor.lineHighlight': '#313244',
    'editor.cursor': '#f5e0dc',
    'editor.selection': '#45475a',
    'editor.inactiveSelection': '#313244',
    'editor.bracketMatch': '#f9e2af',
    'editor.findMatch': '#f9e2af',
    'editor.findMatchHighlight': '#585b70',
    'terminal.background': '#1e1e2e',
    'terminal.cursor': '#f5e0dc',
    'terminal.selection': '#45475a',
    'menu.background': '#181825',
    'menu.itemHover': '#45475a',
    'modal.background': '#181825',
    'modal.itemHover': '#45475a',
    'scrollbar.thumb': '#45475a',
    'scrollbar.track': 'transparent',
    'input.background': '#1e1e2e',
    'input.border': '#585b70',
    'input.focusBorder': '#89b4fa',
    'button.background': '#313244',
    'button.hoverBackground': '#45475a',
    'button.primaryBackground': '#89b4fa',
    'list.background': '#181825',
    'list.hoverBackground': '#313244',
    'list.activeBackground': '#45475a',
}

const EMPTY_PALETTE: Record<string, string> = {}

const labelsOf = (messages: string[]) => messages.map((message) => message.split(' ')[0])

describe('validateStateDistinctness', () => {
    test('기준 팔레트는 어떤 쌍도 위반하지 않는다', () => {
        expect(validateStateDistinctness(BASE_COLORS)).toEqual([])
    })

    test('상태색이 바탕색과 같은 값이면 잡는다(vscode-dark-plus 의 editor.selection 실사례)', () => {
        const errors = validateStateDistinctness({ ...BASE_COLORS, 'editor.selection': '#1e1e2e' })

        expect(labelsOf(errors)).toEqual(['editorSelection'])
    })

    test('반투명 상태색은 바탕 위에 합성한 뒤 판정한다 — 알파만 다른 같은 색은 구별되지 않는다', () => {
        const errors = validateStateDistinctness({ ...BASE_COLORS, 'terminal.selection': '#1e1e2e80' })

        expect(labelsOf(errors)).toEqual(['terminalSelection'])
    })

    test('반투명 컨테이너는 surface 위에 먼저 합성한다 — dracula 식 hover(#44475A75)/selected(#44475A) 조합을 오탐하지 않는다', () => {
        const colors = {
            ...BASE_COLORS,
            'explorer.background': '#21222c',
            'explorer.itemHover': '#44475a75',
            'explorer.itemSelected': '#44475a',
            'explorer.itemFocused': '#44475a75',
        }

        expect(validateStateDistinctness(colors)).toEqual([])
    })

    test('컨테이너가 hex 가 아니면(scrollbar.track: transparent) 그 쌍은 측정 불가로 건너뛴다', () => {
        const errors = validateStateDistinctness({ ...BASE_COLORS, 'scrollbar.thumb': '#1e1e2e' })

        expect(labelsOf(errors)).not.toContain('scrollbarThumb')
    })

    test('관례상 옅은 축(현재 줄 강조)은 완전히 접힌 경우만 잡고 미묘한 차이는 통과시킨다', () => {
        const collapsed = validateStateDistinctness({ ...BASE_COLORS, 'editor.lineHighlight': '#1e1e2e' })
        const subtle = validateStateDistinctness({ ...BASE_COLORS, 'editor.lineHighlight': '#222232' })

        expect(labelsOf(collapsed)).toEqual(['editorCurrentLine'])
        expect(labelsOf(subtle)).toEqual([])
        expect(SUBTLE_STATE_MIN_DISTINCT_DELTA_E).toBeLessThan(STATE_MIN_DISTINCT_DELTA_E)
    })

    /**
     * The sibling rule (d-61 review finding A-1). ayu ships one translucent overlay for both the
     * hovered and the selected tree row, so the two render as the same pixel color; measuring the
     * state *on* the container instead tints it a second time and reports a distinction that is not
     * on screen. Values are `ayu-dark`'s as committed before this fix.
     */
    test('형제 쌍은 양쪽을 표면 위에 각각 합성해 비교한다 — ayu-dark 의 같은 오버레이 hover/selected 를 잡는다', () => {
        const colors = {
            ...BASE_COLORS,
            'app.foreground': '#5a6378',
            'explorer.background': '#0d1017',
            'explorer.itemHover': '#47526640',
            'explorer.itemSelected': '#47526640',
            'explorer.itemFocused': '#47526640',
        }

        expect(labelsOf(validateStateDistinctness(colors))).toContain('explorerSelectedVsHover')
    })

    /**
     * The other side of the same rule: `everforest-light`'s selected row is a genuinely different
     * color from its hover, just barely — ΔE 2.42 against a 2.3 threshold. A boundary value the
     * sibling rule must pass, or the lint would start rewriting deliberately close palettes.
     */
    test('형제 쌍의 경계값은 통과시킨다 — everforest-light 의 listActiveVsHover(ΔE 2.42)', () => {
        const colors = {
            ...BASE_COLORS,
            'list.background': '#fdf6e3',
            'list.hoverBackground': '#f5efde',
            'list.activeBackground': '#e6e2cc80',
            'tabBar.tabActiveBackground': '#fdf6e3',
            'tabBar.tabInactiveBackground': '#fdf6e3',
        }

        expect(labelsOf(validateStateDistinctness(colors))).not.toContain('listActiveVsHover')
        expect(labelsOf(validateStateDistinctness({ ...colors, 'list.activeBackground': '#f5efde' }))).toContain('listActiveVsHover')
    })

    test('형제 쌍은 전부 surfaceKey 를 선언한다(합성 기준면이 없으면 규칙이 성립하지 않는다)', () => {
        const withoutSurface = STATE_DISTINCTNESS_PAIRS.filter((pair) => pair.sibling && !pair.surfaceKey).map((pair) => pair.label)

        expect(withoutSurface).toEqual([])
    })

    test('app.shadow 알파가 하한 미만이면 잡고, 하한값 자체는 통과시킨다', () => {
        const transparent = validateStateDistinctness({ ...BASE_COLORS, 'app.shadow': '#ffffff00' })
        const atBound = validateStateDistinctness({ ...BASE_COLORS, 'app.shadow': '#00000026' })

        expect(labelsOf(transparent)).toEqual(['appShadow'])
        expect(atBound).toEqual([])
        expect(APP_SHADOW_MIN_ALPHA).toBeLessThan(1)
    })

    test('대안 토큰이 구별을 감당하면(평평한 탭 스트립 + 활성 탭 인디케이터) 통과한다', () => {
        const flatTabs = { ...BASE_COLORS, 'tabBar.tabActiveBackground': '#181825', 'tabBar.tabInactiveBackground': '#181825' }

        expect(validateStateDistinctness(flatTabs)).toEqual([])
        expect(labelsOf(validateStateDistinctness({ ...flatTabs, 'tabBar.tabActiveIndicator': '#181825' }))).toEqual([
            'tabActive',
            'tabActiveVsInactive',
        ])
    })
})

describe('repairStateDistinctness', () => {
    test('바탕이 아니라 상태 토큰을 고치고, 임계를 막 넘는 최소 변화로 수리한다', () => {
        const { colors, repairs } = repairStateDistinctness({ ...BASE_COLORS, 'editor.selection': '#1e1e2e' }, EMPTY_PALETTE)

        expect(colors['editor.background']).toBe('#1e1e2e')
        expect(colors['editor.selection']).not.toBe('#1e1e2e')
        expect(deltaE76(colors['editor.selection'], '#1e1e2e')).toBeGreaterThanOrEqual(STATE_MIN_DISTINCT_DELTA_E)
        expect(deltaE76(colors['editor.selection'], '#1e1e2e')).toBeLessThan(STATE_MIN_DISTINCT_DELTA_E + 1)
        expect(repairs).toEqual(['editor.selection: #1e1e2e -> #232333 (editorSelection 구별성 확보)'])
    })

    test('수리 결과는 결정적이고 멱등이다', () => {
        const first = repairStateDistinctness({ ...BASE_COLORS, 'terminal.selection': '#1e1e2e' }, EMPTY_PALETTE)
        const second = repairStateDistinctness(first.colors, EMPTY_PALETTE)

        expect(repairStateDistinctness({ ...BASE_COLORS, 'terminal.selection': '#1e1e2e' }, EMPTY_PALETTE).colors).toEqual(first.colors)
        expect(second.repairs).toEqual([])
        expect(validateStateDistinctness(first.colors)).toEqual([])
    })

    test('테마 palette 에 조건을 만족하는 색이 있으면 파생 대신 원래 색에 가장 가까운 palette 색을 쓴다', () => {
        const palette = { far: '#f38ba8', near: '#45475a' }

        const { colors } = repairStateDistinctness({ ...BASE_COLORS, 'editor.selection': '#1e1e2e' }, palette)

        expect(colors['editor.selection']).toBe('#45475a')
    })

    test('대안 토큰이 있는 쌍은 대안 쪽을 고쳐 평평한 탭 디자인을 보존한다', () => {
        const flatTabs = {
            ...BASE_COLORS,
            'tabBar.tabActiveBackground': '#181825',
            'tabBar.tabInactiveBackground': '#181825',
            'tabBar.tabActiveIndicator': '#181825',
        }

        const { colors } = repairStateDistinctness(flatTabs, EMPTY_PALETTE)

        expect(colors['tabBar.tabActiveBackground']).toBe('#181825')
        expect(colors['tabBar.tabActiveIndicator']).not.toBe('#181825')
        expect(validateStateDistinctness(colors)).toEqual([])
    })

    test('같은 토큰이 걸린 쌍이 여러 개면 전부 만족할 때까지 반복한다', () => {
        const collapsed = { ...BASE_COLORS, 'input.border': '#1e1e2e', 'panel.background': '#1e1e2e', 'input.background': '#1e1e2e' }

        const { colors } = repairStateDistinctness(collapsed, EMPTY_PALETTE)

        expect(validateStateDistinctness(colors)).toEqual([])
    })

    test('형제 쌍의 수리 결과도 표면 위에서 비교했을 때 임계를 넘는다', () => {
        const collapsed = {
            ...BASE_COLORS,
            'app.foreground': '#5a6378',
            'explorer.background': '#0d1017',
            'explorer.itemHover': '#47526640',
            'explorer.itemSelected': '#47526640',
            'explorer.itemFocused': '#47526640',
        }

        const { colors } = repairStateDistinctness(collapsed, EMPTY_PALETTE)

        expect(colors['explorer.itemHover']).toBe('#47526640')
        expect(validateStateDistinctness(colors)).toEqual([])
    })

    test('알파가 비어 버린 app.shadow 는 선언 RGB 를 따르지 않고 검정 하한값으로 되돌린다', () => {
        const { colors, repairs } = repairStateDistinctness({ ...BASE_COLORS, 'app.shadow': '#ffffff00' }, EMPTY_PALETTE)

        expect(colors['app.shadow']).toBe('#00000026')
        expect(repairs).toEqual(['app.shadow: #ffffff00 -> #00000026 (appShadow 구별성 확보)'])
        expect(repairStateDistinctness(colors, EMPTY_PALETTE).repairs).toEqual([])
    })

    test('본문 전경색을 읽을 수 없으면 색을 만들어내지 않고 그대로 둔다', () => {
        const { colors, repairs } = repairStateDistinctness(
            { ...BASE_COLORS, 'app.foreground': 'transparent', 'editor.selection': '#1e1e2e' },
            EMPTY_PALETTE,
        )

        expect(colors['editor.selection']).toBe('#1e1e2e')
        expect(repairs).toEqual([])
    })
})
