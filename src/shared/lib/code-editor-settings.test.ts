import { describe, expect, test } from 'bun:test'
import { resolveCodeEditorSettingsProps } from '@shared/lib/code-editor-settings'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'

describe('resolveCodeEditorSettingsProps', () => {
    test('settings 가 없으면 21개 prop 전부 기본값으로 채운다', () => {
        const props = resolveCodeEditorSettingsProps(undefined)
        expect(props.fontFamily).toBe(buildMonospaceFontStack(null))
        expect(props.fontSize).toBe(13)
        expect(props.minimap).toBe(true)
        expect(props.wordWrap).toBe(false)
        expect(props.lineNumbers).toBe(true)
        expect(props.tabSize).toBe(4)
        expect(props.insertSpaces).toBe(true)
        expect(props.detectIndentation).toBe(true)
        expect(props.renderWhitespace).toBe('selection')
        expect(props.bracketPairColorization).toBe(true)
        expect(props.fontLigatures).toBe(false)
        expect(props.cursorStyle).toBe('line')
        expect(props.cursorBlinking).toBe('blink')
        expect(props.scrollBeyondLastLine).toBe(true)
        expect(props.stickyScroll).toBe(true)
        expect(props.bracketPairGuides).toBe(false)
        expect(props.smoothScrolling).toBe(false)
        expect(props.cursorSmoothCaretAnimation).toBe(false)
        expect(props.suggestPreview).toBe(false)
        expect(props.rulers).toEqual([])
        expect(props.aiAutoTabEnabled).toBe(false)
    })

    test('settings 값이 있으면 21개 prop 전부 그 값을 그대로 사용한다', () => {
        const props = resolveCodeEditorSettingsProps({
            editorFontFamily: 'Fira Code',
            editorFontSize: 16,
            editorMinimap: false,
            editorWordWrap: true,
            editorLineNumbers: false,
            editorTabSize: 2,
            editorInsertSpaces: false,
            editorDetectIndentation: true,
            editorRenderWhitespace: 'all',
            editorBracketPairColorization: false,
            editorFontLigatures: true,
            editorCursorStyle: 'block',
            editorCursorBlinking: 'smooth',
            editorScrollBeyondLastLine: false,
            editorStickyScrollEnabled: true,
            editorBracketPairGuides: true,
            editorSmoothScrolling: true,
            editorCursorSmoothCaretAnimation: true,
            editorSuggestPreview: true,
            editorRulers: [80, 120],
            aiAutoTabEnabled: true,
        })
        expect(props.fontFamily).toBe(buildMonospaceFontStack('Fira Code'))
        expect(props.fontSize).toBe(16)
        expect(props.minimap).toBe(false)
        expect(props.wordWrap).toBe(true)
        expect(props.lineNumbers).toBe(false)
        expect(props.tabSize).toBe(2)
        expect(props.insertSpaces).toBe(false)
        expect(props.detectIndentation).toBe(true)
        expect(props.renderWhitespace).toBe('all')
        expect(props.bracketPairColorization).toBe(false)
        expect(props.fontLigatures).toBe(true)
        expect(props.cursorStyle).toBe('block')
        expect(props.cursorBlinking).toBe('smooth')
        expect(props.scrollBeyondLastLine).toBe(false)
        expect(props.stickyScroll).toBe(true)
        expect(props.bracketPairGuides).toBe(true)
        expect(props.smoothScrolling).toBe(true)
        expect(props.cursorSmoothCaretAnimation).toBe(true)
        expect(props.suggestPreview).toBe(true)
        expect(props.rulers).toEqual([80, 120])
        expect(props.aiAutoTabEnabled).toBe(true)
    })

    test('필드가 없으면 기본값으로 폴백한다', () => {
        const props = resolveCodeEditorSettingsProps({ editorFontFamily: null })
        expect(props.fontSize).toBe(13)
        expect(props.tabSize).toBe(4)
        expect(props.renderWhitespace).toBe('selection')
    })
})
