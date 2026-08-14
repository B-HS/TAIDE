import { describe, expect, test } from 'bun:test'
import {
    buildMonacoKeybinding,
    buildMonacoKeybindingOverrideRules,
    isMonacoCommandId,
    resolveMonacoKeyCode,
    toMonacoActionId,
} from '@shared/lib/monaco-keybinding'

describe('resolveMonacoKeyCode', () => {
    test('알파벳·숫자 키를 KeyCode 값으로 변환한다', () => {
        expect(resolveMonacoKeyCode('f')).toBe(36)
        expect(resolveMonacoKeyCode('0')).toBe(21)
    })

    test('대소문자를 구분하지 않는다', () => {
        expect(resolveMonacoKeyCode('F')).toBe(resolveMonacoKeyCode('f'))
    })

    test('기능키·화살표·구두점을 변환한다', () => {
        expect(resolveMonacoKeyCode('F12')).toBe(70)
        expect(resolveMonacoKeyCode('ArrowUp')).toBe(16)
        expect(resolveMonacoKeyCode('\\')).toBe(93)
    })

    test("canonical 'space' 와 레거시 ' ' 모두 Space KeyCode(10)로 변환한다", () => {
        expect(resolveMonacoKeyCode('space')).toBe(10)
        expect(resolveMonacoKeyCode(' ')).toBe(10)
    })

    test('알 수 없는 키는 null 을 반환한다', () => {
        expect(resolveMonacoKeyCode('Unknown')).toBeNull()
    })
})

describe('buildMonacoKeybinding', () => {
    test('mod 는 CtrlCmd 비트(2048)를 더한다', () => {
        expect(buildMonacoKeybinding('f', ['mod'])).toBe(2048 | 36)
    })

    test('ctrl 은 WinCtrl 비트(256)를 더한다', () => {
        expect(buildMonacoKeybinding('g', ['ctrl'])).toBe(256 | 37)
    })

    test('여러 modifier 를 비트 OR 로 조합한다', () => {
        expect(buildMonacoKeybinding('l', ['mod', 'shift'])).toBe(2048 | 1024 | 42)
    })

    test('modifier 없이 키만 지정할 수 있다', () => {
        expect(buildMonacoKeybinding('F12', [])).toBe(70)
    })

    test('Option+Space(alt+space) 는 Alt 비트(512)와 Space(10)의 조합이 된다', () => {
        expect(buildMonacoKeybinding('space', ['alt'])).toBe(512 | 10)
    })

    test('알 수 없는 키면 null 을 반환한다', () => {
        expect(buildMonacoKeybinding('Unknown', ['mod'])).toBeNull()
    })
})

describe('monaco 커맨드 id 유틸', () => {
    test('isMonacoCommandId 는 monaco. 접두사를 판별한다', () => {
        expect(isMonacoCommandId('monaco.editor.action.rename')).toBe(true)
        expect(isMonacoCommandId('editor.save')).toBe(false)
    })

    test('toMonacoActionId 는 접두사를 제거한다', () => {
        expect(toMonacoActionId('monaco.editor.action.rename')).toBe('editor.action.rename')
    })

    test('toMonacoActionId 는 접두사가 없으면 그대로 반환한다', () => {
        expect(toMonacoActionId('editor.action.rename')).toBe('editor.action.rename')
    })
})

describe('buildMonacoKeybindingOverrideRules', () => {
    test('monaco 접두사가 없는 오버라이드는 규칙을 만들지 않는다', () => {
        expect(buildMonacoKeybindingOverrideRules([{ actionId: 'save', key: 's', mods: ['mod'] }])).toEqual([])
    })

    test('일반 액션 재바인딩은 unbind(keybinding 0) + rebind 두 규칙만 만든다', () => {
        expect(buildMonacoKeybindingOverrideRules([{ actionId: 'monaco.editor.action.rename', key: 'r', mods: ['mod', 'shift'] }])).toEqual([
            { keybinding: 0, command: '-editor.action.rename' },
            { keybinding: 2048 | 1024 | 48, command: 'editor.action.rename' },
        ])
    })

    test('key 가 빈 오버라이드(언바인드)는 unbind 규칙만 만든다', () => {
        expect(buildMonacoKeybindingOverrideRules([{ actionId: 'monaco.editor.action.rename', key: '', mods: [] }])).toEqual([
            { keybinding: 0, command: '-editor.action.rename' },
        ])
    })

    test('KeyCode 매핑이 없는 키는 규칙 전체를 생략해 monaco 기본을 유지한다', () => {
        expect(buildMonacoKeybindingOverrideRules([{ actionId: 'monaco.editor.action.rename', key: 'Unknown', mods: ['mod'] }])).toEqual([])
    })

    test('triggerSuggest 를 ⌥Space 로 재바인딩하면 ⌥ 유지 상태의 위젯 탐색 규칙을 함께 만든다', () => {
        const rules = buildMonacoKeybindingOverrideRules([{ actionId: 'monaco.editor.action.triggerSuggest', key: 'space', mods: ['alt'] }])
        const suggestNavWhen = 'suggestWidgetVisible && textInputFocus && (suggestWidgetMultipleSuggestions || !suggestWidgetHasFocusedSuggestion)'
        const suggestArrowNavWhen = `${suggestNavWhen} && !(parameterHintsVisible && parameterHintsMultipleSignatures)`
        expect(rules).toEqual([
            { keybinding: 0, command: '-editor.action.triggerSuggest' },
            { keybinding: 512 | 10, command: 'editor.action.triggerSuggest' },
            { keybinding: 512 | 16, command: 'selectPrevSuggestion', when: suggestArrowNavWhen },
            { keybinding: 512 | 18, command: 'selectNextSuggestion', when: suggestArrowNavWhen },
            { keybinding: 512 | 11, command: 'selectPrevPageSuggestion', when: suggestNavWhen },
            { keybinding: 512 | 12, command: 'selectNextPageSuggestion', when: suggestNavWhen },
        ])
    })

    test('⌥ 유지 상태의 suggest Up/Down 탐색 규칙은 시그니처가 여러 개 열린 parameter hints 에 순서 탐색을 양보한다 (parameterHints.js 의 Alt+Up/Down escape hatch 보존)', () => {
        const rules = buildMonacoKeybindingOverrideRules([{ actionId: 'monaco.editor.action.triggerSuggest', key: 'space', mods: ['alt'] }])
        const arrowUpCompanion = rules.find((rule) => rule.keybinding === (512 | 16) && rule.command === 'selectPrevSuggestion')
        const arrowDownCompanion = rules.find((rule) => rule.keybinding === (512 | 18) && rule.command === 'selectNextSuggestion')
        expect(arrowUpCompanion?.when).toContain('!(parameterHintsVisible && parameterHintsMultipleSignatures)')
        expect(arrowDownCompanion?.when).toContain('!(parameterHintsVisible && parameterHintsMultipleSignatures)')
    })

    test('triggerParameterHints 재바인딩은 시그니처 탐색 규칙을 함께 만든다 (monaco 기본과 동일하게 editorFocus 를 포함)', () => {
        const rules = buildMonacoKeybindingOverrideRules([
            { actionId: 'monaco.editor.action.triggerParameterHints', key: 'space', mods: ['mod', 'shift'] },
        ])
        const hintsNavWhen = 'editorFocus && parameterHintsVisible && parameterHintsMultipleSignatures'
        expect(rules).toEqual([
            { keybinding: 0, command: '-editor.action.triggerParameterHints' },
            { keybinding: 2048 | 1024 | 10, command: 'editor.action.triggerParameterHints' },
            { keybinding: 2048 | 1024 | 16, command: 'showPrevParameterHint', when: hintsNavWhen },
            { keybinding: 2048 | 1024 | 18, command: 'showNextParameterHint', when: hintsNavWhen },
        ])
    })

    test('동반 위젯-탐색 규칙은 재바인딩 저장 순서와 무관하게 항상 다른 액션의 재바인딩보다 뒤(=높은 우선순위)에 배치된다 (KeybindingResolver 는 마지막 매치를 채택하므로 순서 의존 섀도잉을 방지)', () => {
        const suggestFirst = buildMonacoKeybindingOverrideRules([
            { actionId: 'monaco.editor.action.triggerSuggest', key: 'space', mods: ['alt'] },
            { actionId: 'monaco.editor.action.formatDocument', key: 'ArrowUp', mods: ['alt'] },
        ])
        const suggestLast = buildMonacoKeybindingOverrideRules([
            { actionId: 'monaco.editor.action.formatDocument', key: 'ArrowUp', mods: ['alt'] },
            { actionId: 'monaco.editor.action.triggerSuggest', key: 'space', mods: ['alt'] },
        ])

        for (const rules of [suggestFirst, suggestLast]) {
            const companionIndex = rules.findIndex((rule) => rule.command === 'selectPrevSuggestion')
            const unrelatedRebindIndex = rules.findIndex((rule) => rule.command === 'editor.action.formatDocument')
            expect(companionIndex).toBeGreaterThan(unrelatedRebindIndex)
        }
    })

    test('modifier 가 없는 재바인딩(레거시 데이터)에는 탐색 규칙을 만들지 않는다', () => {
        const rules = buildMonacoKeybindingOverrideRules([{ actionId: 'monaco.editor.action.triggerSuggest', key: 'F5', mods: [] }])
        expect(rules).toEqual([
            { keybinding: 0, command: '-editor.action.triggerSuggest' },
            { keybinding: 63, command: 'editor.action.triggerSuggest' },
        ])
    })

    test('여러 오버라이드를 순서대로 평탄화한다', () => {
        const rules = buildMonacoKeybindingOverrideRules([
            { actionId: 'monaco.editor.action.rename', key: '', mods: [] },
            { actionId: 'save', key: 's', mods: ['mod'] },
            { actionId: 'monaco.editor.action.formatDocument', key: 'f', mods: ['alt', 'shift'] },
        ])
        expect(rules).toEqual([
            { keybinding: 0, command: '-editor.action.rename' },
            { keybinding: 0, command: '-editor.action.formatDocument' },
            { keybinding: 512 | 1024 | 36, command: 'editor.action.formatDocument' },
        ])
    })
})
