import { describe, expect, test } from 'bun:test'
import { repairContrastPairs, validateOutputColors, validateSelectionRowContrast } from '@shared/lib/theme-convert/contrast'

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
