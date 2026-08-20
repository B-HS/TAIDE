import { describe, expect, test } from 'bun:test'
import { resolveCodeEditorSettingsProps } from '@shared/lib/code-editor-settings'

describe('resolveCodeEditorSettingsProps', () => {
    test('settings 가 없으면 기본값으로 채운다', () => {
        const props = resolveCodeEditorSettingsProps(undefined)
        expect(props.fontSize).toBe(13)
        expect(props.minimap).toBe(true)
        expect(props.wordWrap).toBe(false)
        expect(props.tabSize).toBe(4)
        expect(props.renderWhitespace).toBe('selection')
        expect(props.cursorStyle).toBe('line')
        expect(props.cursorBlinking).toBe('blink')
    })

    test('settings 값이 있으면 그 값을 그대로 사용한다', () => {
        const props = resolveCodeEditorSettingsProps({
            editorFontFamily: 'Fira Code',
            editorFontSize: 16,
            editorMinimap: false,
            editorWordWrap: true,
            editorLineNumbers: false,
            editorTabSize: 2,
            editorInsertSpaces: false,
            editorDetectIndentation: false,
            editorRenderWhitespace: 'all',
            editorBracketPairColorization: false,
            editorFontLigatures: true,
            editorCursorStyle: 'block',
            editorCursorBlinking: 'smooth',
            editorScrollBeyondLastLine: false,
            editorStickyScrollEnabled: false,
            aiAutoTabEnabled: true,
        })
        expect(props.fontSize).toBe(16)
        expect(props.minimap).toBe(false)
        expect(props.tabSize).toBe(2)
        expect(props.renderWhitespace).toBe('all')
        expect(props.cursorStyle).toBe('block')
        expect(props.cursorBlinking).toBe('smooth')
        expect(props.aiAutoTabEnabled).toBe(true)
    })

    test('필드가 없으면 기본값으로 폴백한다', () => {
        const props = resolveCodeEditorSettingsProps({ editorFontFamily: null })
        expect(props.fontSize).toBe(13)
        expect(props.tabSize).toBe(4)
        expect(props.renderWhitespace).toBe('selection')
    })
})
