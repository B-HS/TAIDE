import { describe, expect, test } from 'bun:test'
import type { EditorConfigOptions } from '@shared/api/bindings'
import { resolveEditorConfigIndentProps, resolveEditorConfigModelIndent, resolveOnSaveCleanupFlags } from '@shared/lib/editorconfig'

const EMPTY_EDITOR_CONFIG: EditorConfigOptions = {
    indentStyle: null,
    indentSize: null,
    tabWidth: null,
    insertFinalNewline: null,
    trimTrailingWhitespace: null,
}

const editorConfig = (overrides: Partial<EditorConfigOptions>): EditorConfigOptions => ({ ...EMPTY_EDITOR_CONFIG, ...overrides })

describe('resolveEditorConfigIndentProps', () => {
    test('editorconfig 가 없으면 두 축 모두 오버라이드가 없다', () => {
        expect(resolveEditorConfigIndentProps(null)).toEqual({ editorConfigTabSize: null, editorConfigInsertSpaces: null })
        expect(resolveEditorConfigIndentProps(undefined)).toEqual({ editorConfigTabSize: null, editorConfigInsertSpaces: null })
        expect(resolveEditorConfigIndentProps(EMPTY_EDITOR_CONFIG)).toEqual({ editorConfigTabSize: null, editorConfigInsertSpaces: null })
    })

    test('indent_style 을 insertSpaces 로 옮긴다', () => {
        expect(resolveEditorConfigIndentProps(editorConfig({ indentStyle: 'space' })).editorConfigInsertSpaces).toBe(true)
        expect(resolveEditorConfigIndentProps(editorConfig({ indentStyle: 'tab' })).editorConfigInsertSpaces).toBe(false)
    })

    test('space 스타일은 indent_size 를, tab 스타일은 tab_width 를 tabSize 로 쓴다', () => {
        expect(resolveEditorConfigIndentProps(editorConfig({ indentStyle: 'space', indentSize: 2, tabWidth: 8 })).editorConfigTabSize).toBe(2)
        expect(resolveEditorConfigIndentProps(editorConfig({ indentStyle: 'tab', indentSize: 2, tabWidth: 8 })).editorConfigTabSize).toBe(8)
    })

    test('선호 프로퍼티가 비면 다른 쪽으로 폴백한다', () => {
        expect(resolveEditorConfigIndentProps(editorConfig({ indentStyle: 'tab', indentSize: 4 })).editorConfigTabSize).toBe(4)
        expect(resolveEditorConfigIndentProps(editorConfig({ indentStyle: 'space', tabWidth: 4 })).editorConfigTabSize).toBe(4)
    })

    test('스타일 없이 크기만 있으면 tabSize 만 오버라이드한다', () => {
        expect(resolveEditorConfigIndentProps(editorConfig({ indentSize: 3 }))).toEqual({
            editorConfigTabSize: 3,
            editorConfigInsertSpaces: null,
        })
    })
})

describe('resolveEditorConfigModelIndent', () => {
    test('오버라이드가 없으면 모델을 건드리지 않도록 null 을 돌려준다', () => {
        expect(resolveEditorConfigModelIndent({ editorConfigTabSize: null, editorConfigInsertSpaces: null })).toBeNull()
    })

    test('지정되지 않은 축은 undefined 로 남겨 모델의 기존 값을 보존한다', () => {
        expect(resolveEditorConfigModelIndent({ editorConfigTabSize: null, editorConfigInsertSpaces: false })).toEqual({
            tabSize: undefined,
            insertSpaces: false,
        })
        expect(resolveEditorConfigModelIndent({ editorConfigTabSize: 2, editorConfigInsertSpaces: null })).toEqual({
            tabSize: 2,
            insertSpaces: undefined,
        })
    })

    test('두 축이 모두 있으면 그대로 싣는다', () => {
        expect(resolveEditorConfigModelIndent({ editorConfigTabSize: 2, editorConfigInsertSpaces: true })).toEqual({
            tabSize: 2,
            insertSpaces: true,
        })
    })
})

describe('resolveOnSaveCleanupFlags', () => {
    const settings = { trimTrailingWhitespaceOnSave: true, insertFinalNewlineOnSave: false }

    test('editorconfig 가 없으면 전역 설정을 그대로 쓴다', () => {
        expect(resolveOnSaveCleanupFlags(settings, null)).toEqual(settings)
        expect(resolveOnSaveCleanupFlags(settings, EMPTY_EDITOR_CONFIG)).toEqual(settings)
    })

    test('editorconfig 값이 전역 설정을 이긴다', () => {
        expect(resolveOnSaveCleanupFlags(settings, editorConfig({ trimTrailingWhitespace: false, insertFinalNewline: true }))).toEqual({
            trimTrailingWhitespaceOnSave: false,
            insertFinalNewlineOnSave: true,
        })
    })

    test('언급하지 않은 프로퍼티만 전역 설정으로 폴백한다', () => {
        expect(resolveOnSaveCleanupFlags(settings, editorConfig({ insertFinalNewline: true }))).toEqual({
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: true,
        })
    })

    test('설정이 아직 로드되지 않았으면 undefined 를 유지한다', () => {
        expect(resolveOnSaveCleanupFlags({ trimTrailingWhitespaceOnSave: undefined, insertFinalNewlineOnSave: undefined }, null)).toEqual({
            trimTrailingWhitespaceOnSave: undefined,
            insertFinalNewlineOnSave: undefined,
        })
    })
})
