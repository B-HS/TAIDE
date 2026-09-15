import { describe, expect, test } from 'bun:test'
import { deltaE76 } from '@shared/lib/color'
import {
    contrastRatio,
    isExemptComponentContrastViolation,
    repairComponentContrast,
    repairContrastPairs,
    validateComponentContrast,
    validateFixedForegroundContrast,
    validateOutputColors,
    validateSelectionRowContrast,
    validateTerminalAnsiContrast,
} from '@shared/lib/theme-convert/contrast'

const BASE_COLORS: Record<string, string> = {
    'app.foreground': '#d4d4d4',
    'app.background': '#1e1e1e',
    'editor.foreground': '#d4d4d4',
    'editor.background': '#1e1e1e',
    'panel.sectionHeader': '#d4d4d4',
    'panel.background': '#000000',
    'tooltip.background': '#1e1e1e',
    'panel.matchHighlight': '#569cd6',
    'list.foreground': '#d4d4d4',
    'list.background': '#1e1e1e',
    'list.hoverBackground': '#1e1e1e',
    'list.activeBackground': '#1e1e1e',
}

describe('validateOutputColors', () => {
    test('panel.matchHighlight 가 반투명(github-dark 실사례 #ffd33d22)이면 배경 위에 합성한 뒤 대비를 판정해 저대비로 잡는다', () => {
        const errors = validateOutputColors({ ...BASE_COLORS, 'panel.matchHighlight': '#ffd33d22' })

        expect(errors.some((error) => error.includes('matchHighlight'))).toBe(true)
    })

    test('panel.matchHighlight 가 불투명이면 알파 합성 없이 일반 대비 판정을 그대로 따른다', () => {
        const errors = validateOutputColors({ ...BASE_COLORS, 'panel.matchHighlight': '#ffd33d' })

        expect(errors.some((error) => error.includes('matchHighlight'))).toBe(false)
    })
})

describe('repairContrastPairs', () => {
    test('panel.matchHighlight 대비가 부족하면 업스트림 불투명 전경 후보(editor.foreground)로 수리한다', () => {
        const vscodeColors = { 'editor.foreground': '#d4d4d4' }

        const { colors, repairs } = repairContrastPairs({ ...BASE_COLORS, 'panel.matchHighlight': '#ffd33d22' }, vscodeColors)

        expect(colors['panel.matchHighlight']).toBe('#d4d4d4')
        expect(repairs.some((repair) => repair.includes('matchHighlight'))).toBe(true)
    })

    test('알파 합성 도입 후에도 기존 4쌍은 6자리 hex 에 대해 동일하게 수리된다(합성 항등)', () => {
        const vscodeColors = { 'editor.foreground': '#d4d4d4' }

        const { colors, repairs } = repairContrastPairs(
            { ...BASE_COLORS, 'app.foreground': '#000000', 'panel.matchHighlight': '#569cd6' },
            vscodeColors,
        )

        expect(colors['app.foreground']).toBe('#d4d4d4')
        expect(repairs.some((repair) => repair.startsWith('app.foreground'))).toBe(true)
        expect(colors['panel.matchHighlight']).toBe('#569cd6')
    })
})

describe('validateOutputColors — 8자리 전경을 가진 기존 4쌍(app 등)의 합성 의도', () => {
    test('알파를 무시하면 통과했을 값이 배경 위에 합성한 뒤에는 저대비로 뒤집힌다(vitesse-dark 8자리 전경 사례와 같은 형태)', () => {
        const errors = validateOutputColors({
            ...BASE_COLORS,
            'app.foreground': '#00000030',
            'app.background': '#ffffff',
            'panel.matchHighlight': '#d4d4d4',
        })

        expect(errors.some((error) => error.startsWith('app 대비 부족'))).toBe(true)
    })
})

describe('선택 행 축(d-40) — validateOutputColors 는 non-blocking, validateSelectionRowContrast 는 별도 감사', () => {
    test('선택 행 2쌍이 모두 저대비여도 기존 5쌍이 전부 통과하면 validateOutputColors 는 거부하지 않는다(임포트 거부 신설 금지)', () => {
        const colors = {
            ...BASE_COLORS,
            'panel.matchHighlight': '#569cd6',
            'list.activeBackground': '#569cd6',
        }

        expect(validateOutputColors(colors)).toEqual([])

        const selectionErrors = validateSelectionRowContrast(colors)
        expect(selectionErrors.some((error) => error.includes('selectionMatchHighlight'))).toBe(true)
        expect(selectionErrors.some((error) => error.includes('selectionForeground'))).toBe(true)
    })

    test('기존 5쌍 중 하나라도 실패하면 validateOutputColors 는 여전히 거부한다(선택 행 축 신설이 기존 판정을 약화시키지 않는다)', () => {
        const colors = { ...BASE_COLORS, 'app.foreground': '#101010', 'app.background': '#0e0e0e' }

        expect(validateOutputColors(colors).some((error) => error.startsWith('app 대비 부족'))).toBe(true)
    })
})

describe('repairContrastPairs — 선택 행 일반축(list.foreground)', () => {
    test('list.foreground 대비가 부족하면 업스트림 불투명 전경 후보(editor.foreground)로 수리한다', () => {
        const vscodeColors = { 'editor.foreground': '#d4d4d4' }

        const { colors, repairs } = repairContrastPairs({ ...BASE_COLORS, 'list.foreground': '#3a3a3a' }, vscodeColors)

        expect(colors['list.foreground']).toBe('#d4d4d4')
        expect(repairs.some((repair) => repair.startsWith('list.foreground'))).toBe(true)
    })
})

describe('repairContrastPairs — panel.matchHighlight 를 공유하는 두 배경(panel.background 블로킹·list.activeBackground 어드바이저리)', () => {
    test('두 배경을 동시에 만족하는 후보가 없으면, 어드바이저리 축 수리가 이미 통과한 블로킹 축 값을 깨지 않는다', () => {
        const colors = { ...BASE_COLORS, 'panel.matchHighlight': '#ffd33d22', 'list.activeBackground': '#606060' }
        const vscodeColors = { 'textLink.foreground': '#0d0d0d', 'editor.foreground': '#7f7f7f' }

        const { colors: repairedColors } = repairContrastPairs(colors, vscodeColors)

        expect(repairedColors['panel.matchHighlight']).toBe('#7f7f7f')
        expect(validateOutputColors(repairedColors)).toEqual([])
        expect(validateSelectionRowContrast(repairedColors).some((error) => error.includes('selectionMatchHighlight'))).toBe(true)
    })

    /**
     * d-40 검토 확정 재현 입력(`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md`
     * 검토 findings d40-advisory-repair-clobbers-d33-distinctness/d40-l2-02): 블로킹 `matchHighlight`
     * 축은 이미 통과 중(#907aa9 vs panel.background #faf4ed = 3.47)이고 구별성도 만족하는데,
     * 어드바이저리 `selectionMatchHighlight` 축만 저대비(#907aa9 vs list.activeBackground
     * #ADD6FF = 2.50)다. 수리 전 코드는 1패스 실패 시 구별성 없는 2패스로 떨어져
     * `editor.foreground`(app.foreground 와 동일색 #575279)를 채택 — 이미 통과·구별되던 블로킹 값을
     * 본문 전경과 동일색으로 덮어썼다(d-33 결함 재도입). 이 테스트는 어드바이저리 축이 조용히
     * 미수리로 남고 블로킹 값이 무손상인지 고정한다.
     */
    test('어드바이저리 매치 축이 구별 가능한 후보를 찾지 못하면(2패스 폴백 금지), 이미 통과 중인 블로킹 값을 그대로 둔다', () => {
        const colors = {
            ...BASE_COLORS,
            'app.foreground': '#575279',
            'app.background': '#faf4ed',
            'editor.foreground': '#575279',
            'editor.background': '#faf4ed',
            'panel.sectionHeader': '#575279',
            'panel.background': '#faf4ed',
            'tooltip.background': '#faf4ed',
            'panel.matchHighlight': '#907aa9',
            'list.activeBackground': '#ADD6FF',
        }
        const vscodeColors = { 'textLink.foreground': '#907aa9', 'editor.foreground': '#575279' }

        const { colors: repairedColors, repairs } = repairContrastPairs(colors, vscodeColors)

        expect(repairedColors['panel.matchHighlight']).toBe('#907aa9')
        expect(repairs.some((repair) => repair.startsWith('panel.matchHighlight'))).toBe(false)
        expect(validateOutputColors(repairedColors)).toEqual([])
        expect(validateSelectionRowContrast(repairedColors).some((error) => error.includes('selectionMatchHighlight'))).toBe(true)
    })
})

/**
 * d-40 검토 확정 재현(finding d40-listfg-multisurface-regression/d40-l2-01/d40-l2-03,
 * D40-L3-01): `list.foreground` 는 선택 행 전용이 아니라 `list.background`/`list.hoverBackground`
 * 위에도(비선택 행·hover 상태·`--accent-foreground`) 그려지는 공용 전경이다. `selectionForeground`
 * 축(vs `list.activeBackground`) 수리가 이 두 표면을 확인하지 않으면, 선택 행 하나를 고치려다
 * 훨씬 넓게 쓰이는 비선택/hover 표면을 깨뜨릴 수 있다 — nord 번들 데이터에서 실제로 발생했던
 * 형태를 그대로 재현한다.
 */
describe('repairContrastPairs — list.foreground 수리는 list.background/list.hoverBackground 도 보호한다(nord 형 입력)', () => {
    test('전경 후보가 선택 배경은 만족해도 list.background/list.hoverBackground 를 깨면 채택하지 않는다', () => {
        const colors = {
            ...BASE_COLORS,
            'list.foreground': '#d8dee9',
            'list.background': '#2e3440',
            'list.hoverBackground': '#3b4252',
            'list.activeBackground': '#88c0d0',
        }
        const vscodeColors = { 'editor.foreground': '#2e3440' }

        const { colors: repairedColors, repairs } = repairContrastPairs(colors, vscodeColors)

        expect(repairedColors['list.foreground']).toBe('#d8dee9')
        expect(repairs.some((repair) => repair.startsWith('list.foreground'))).toBe(false)
        expect(validateSelectionRowContrast(repairedColors).some((error) => error.includes('selectionForeground'))).toBe(true)
    })
})

/**
 * d-61 §1.B. These exercise `component-contrast-pairs.ts`'s axes through the three entry points the
 * catalog gate and `scripts/repair-theme-contrast.ts` use.
 */
describe('validateComponentContrast — 배경을 표면 위에 합성한 뒤 판정한다', () => {
    test('완전 투명한 탭 배경(rose-pine 계열 #00000000)은 raw RGB(검정)가 아니라 그 아래 앱 배경으로 읽는다', () => {
        const colors = {
            'app.background': '#ffffff',
            'tabBar.tabInactiveBackground': '#00000000',
            'tabBar.tabInactiveForeground': '#111111',
        }

        expect(validateComponentContrast(colors)).toEqual([])
    })

    test('불투명 배경이면 합성이 항등이라 일반 대비 판정을 그대로 따른다', () => {
        const colors = {
            'app.background': '#ffffff',
            'tabBar.tabInactiveBackground': '#ffffff',
            'tabBar.tabInactiveForeground': '#dddddd',
        }

        expect(validateComponentContrast(colors).some((error) => error.startsWith('tabInactive'))).toBe(true)
    })
})

describe('repairComponentContrast', () => {
    test('테마 고유 색상(hue)을 유지한 채 최소한만 이동한다 — 연한 초록 git 데코는 진한 초록으로 돌아온다', () => {
        const colors = { 'app.background': '#ffffff', 'explorer.background': '#f6f8fa', 'explorer.gitAdded': '#28a745' }

        const { colors: repairedColors, repairs } = repairComponentContrast(colors)
        const repaired = repairedColors['explorer.gitAdded']

        expect(repairs.some((repair) => repair.startsWith('explorer.gitAdded'))).toBe(true)
        expect(validateComponentContrast(repairedColors)).toEqual([])
        expect(contrastRatio(repaired, '#f6f8fa')).toBeGreaterThanOrEqual(3)
        expect(deltaE76(repaired, '#28a745')).toBeLessThan(5)
    })

    test('한 토큰이 여러 표면에 그려지면 한 번만 이동해 전부를 동시에 만족시킨다', () => {
        const colors = {
            'app.background': '#282828',
            'appSidebar.background': '#282828',
            'panel.background': '#1e1f1c',
            'statusIndicator.error': '#cc241d',
        }

        const { colors: repairedColors, repairs } = repairComponentContrast(colors)

        expect(repairs.filter((repair) => repair.startsWith('statusIndicator.error'))).toHaveLength(1)
        expect(repairs[0]).toContain('statusBarError/problemError')
        expect(validateComponentContrast(repairedColors)).toEqual([])
    })

    test('두 표면이 서로 반대쪽 명도라 한 값으로 둘 다 만족할 수 없으면 수리하지 않는다(한쪽을 고치려고 다른 쪽을 깨지 않는다)', () => {
        const colors = {
            'app.background': '#ffffff',
            'appSidebar.background': '#9d9d9d',
            'panel.background': '#333333',
            'statusIndicator.error': '#ff0000',
        }

        const { colors: repairedColors, repairs } = repairComponentContrast(colors)

        expect(repairs).toEqual([])
        expect(repairedColors['statusIndicator.error']).toBe('#ff0000')
    })

    /**
     * `list.foreground` is shared with the d-40 `selectionForeground` pair. A component-axis repair
     * has to keep that surface legible, and — since `bundled-theme-contrast.test.ts` pins exactly
     * which themes still fail that axis and why — must never be the thing that repairs it either.
     */
    test('list.foreground 수리는 선택 행 축(list.activeBackground)도 만족해야 하며, 만족 못 하면 미수리로 남는다', () => {
        const colors = {
            'app.background': '#000000',
            'list.foreground': '#888888',
            'list.background': '#000000',
            'list.hoverBackground': '#777777',
            'list.activeBackground': '#ffffff',
        }

        const { colors: repairedColors, repairs } = repairComponentContrast(colors)

        expect(repairs).toEqual([])
        expect(repairedColors['list.foreground']).toBe('#888888')
        expect(validateComponentContrast(repairedColors).some((error) => error.startsWith('listHoverRow'))).toBe(true)
    })

    test('컴포넌트 축이 전부 통과하면 선택 행 축만 미달이어도 건드리지 않는다(예외 등재분 보호)', () => {
        const colors = {
            'app.background': '#ffffff',
            'list.foreground': '#797593',
            'list.background': '#ffffff',
            'list.hoverBackground': '#ffffff',
            'list.activeBackground': '#6e6a86',
        }

        const { colors: repairedColors, repairs } = repairComponentContrast(colors)

        expect(repairs).toEqual([])
        expect(repairedColors['list.foreground']).toBe('#797593')
    })
})

describe('validateTerminalAnsiContrast — 자문 전용', () => {
    test('배경에 묻힌 ANSI 색을 보고한다', () => {
        const errors = validateTerminalAnsiContrast({ background: '#1e1e1e', black: '#101010', white: '#e5e5e5' })

        expect(errors).toHaveLength(1)
        expect(errors[0]).toStartWith('ansi black')
    })

    test('전부 읽히는 팔레트면 아무것도 보고하지 않는다', () => {
        const errors = validateTerminalAnsiContrast({ background: '#1e1e1e', red: '#f14c4c', white: '#e5e5e5' })

        expect(errors).toEqual([])
    })
})

describe('menu.itemHover — 배경 쪽 수리', () => {
    /**
     * `vscode-light-modern` as shipped upstream: the menu hover is the VS Code accent
     * (`menu.selectionBackground` `#005FB8`) while the row label stays `app.foreground`, which the
     * d-61 review measured at 1.78:1 (finding G-3).
     */
    const ACCENT_MENU_COLORS: Record<string, string> = {
        'app.background': '#ffffff',
        'app.foreground': '#3b3b3b',
        'menu.background': '#ffffff',
        'menu.itemHover': '#005fb8',
        'list.hoverBackground': '#f1f1f1',
    }

    test('본문 전경이 아니라 menu.itemHover 를 list.hoverBackground 로 교체해 대비를 확보한다', () => {
        const { colors, repairs } = repairComponentContrast(ACCENT_MENU_COLORS)

        expect(colors['app.foreground']).toBe('#3b3b3b')
        expect(colors['menu.itemHover']).toBe('#f1f1f1')
        expect(repairs).toEqual(['menu.itemHover: #005fb8 -> #f1f1f1 (menuItemHoverText 대비 확보)'])
        expect(validateComponentContrast(colors).some((error) => error.startsWith('menuItemHoverText '))).toBe(false)
    })

    /**
     * The substitute has to keep reading as a hover — `state-distinctness-pairs.ts`'s `menuItemHover`
     * axis measures the same two tokens — so a candidate that equals the menu background is refused
     * rather than trading this lint's failure for that one's.
     */
    test('후보가 menu.background 와 구별되지 않으면 교체하지 않는다', () => {
        const { colors, repairs } = repairComponentContrast({ ...ACCENT_MENU_COLORS, 'list.hoverBackground': '#ffffff' })

        expect(colors['menu.itemHover']).toBe('#005fb8')
        expect(repairs).toEqual([])
    })

    test('후보가 임계를 못 넘으면 원래 값을 그대로 둔다(ayu 식 — 더 나쁜 값으로 바꾸지 않는다)', () => {
        const ayuLike = {
            'app.background': '#0d1017',
            'app.foreground': '#5a6378',
            'menu.background': '#0f131a',
            'menu.itemHover': '#47526633',
            'list.hoverBackground': '#47526640',
        }

        const { colors, repairs } = repairComponentContrast(ayuLike)

        expect(colors['menu.itemHover']).toBe('#47526633')
        expect(repairs).toEqual([])
    })

    test('등재된 예외만 그 축에서 무시한다', () => {
        expect(isExemptComponentContrastViolation('ayu-dark', 'menuItemHoverText 대비 부족: ...')).toBe(true)
        expect(isExemptComponentContrastViolation('ayu-dark', 'listRow 대비 부족: ...')).toBe(false)
        expect(isExemptComponentContrastViolation('dracula', 'menuItemHoverText 대비 부족: ...')).toBe(false)
    })
})

describe('validateFixedForegroundContrast — 자문 전용', () => {
    test('테마가 못 바꾸는 고정 전경(파괴적 버튼의 흰 라벨)의 미달을 보고한다', () => {
        const errors = validateFixedForegroundContrast({
            'app.background': '#1e1e1e',
            'panel.background': '#1e1e1e',
            'statusIndicator.error': '#f38ba8',
        })

        expect(errors).toHaveLength(1)
        expect(errors[0]).toStartWith('destructiveButtonLabel')
    })

    test('충분히 어두운 배경이면 아무것도 보고하지 않는다', () => {
        const errors = validateFixedForegroundContrast({
            'app.background': '#1e1e1e',
            'panel.background': '#1e1e1e',
            'statusIndicator.error': '#a31515',
        })

        expect(errors).toEqual([])
    })
})
