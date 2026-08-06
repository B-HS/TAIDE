import { describe, expect, test } from 'bun:test'
import { toCssVariableName, toCssVariables } from '@shared/lib/theme-variables'

describe('toCssVariableName', () => {
    test('네임스페이스와 토큰을 kebab-case 변수명으로 바꾼다', () => {
        expect(toCssVariableName('tabBar.dirtyDot')).toBe('--taide-tab-bar-dirty-dot')
    })

    test('단일 단어 토큰도 처리한다', () => {
        expect(toCssVariableName('app.background')).toBe('--taide-app-background')
    })

    test('숫자가 붙은 토큰을 쪼개지 않는다', () => {
        expect(toCssVariableName('graph.lane12')).toBe('--taide-graph-lane12')
    })

    test('연속 대문자 경계를 처리한다', () => {
        expect(toCssVariableName('editorGutter.addedBackground')).toBe('--taide-editor-gutter-added-background')
    })
})

describe('toCssVariables', () => {
    test('전체 토큰 맵을 변수 맵으로 변환한다', () => {
        expect(toCssVariables({ 'app.background': '#000', 'tabBar.dirtyDot': '#fff' })).toEqual({
            '--taide-app-background': '#000',
            '--taide-tab-bar-dirty-dot': '#fff',
        })
    })
})
