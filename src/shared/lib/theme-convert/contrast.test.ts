import { describe, expect, test } from 'bun:test'
import { repairContrastPairs, validateOutputColors } from '@shared/lib/theme-convert/contrast'

const BASE_COLORS: Record<string, string> = {
    'app.foreground': '#d4d4d4',
    'app.background': '#1e1e1e',
    'editor.foreground': '#d4d4d4',
    'editor.background': '#1e1e1e',
    'panel.sectionHeader': '#d4d4d4',
    'panel.background': '#000000',
    'tooltip.background': '#1e1e1e',
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
