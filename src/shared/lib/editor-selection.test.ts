import { describe, expect, test } from 'bun:test'
import type { editor } from 'monaco-editor'
import { resolveSelectedTextOrCurrentLine } from '@shared/lib/editor-selection'

type FakeEditorInit = {
    model: { getValueInRange: (selection: unknown) => string; getLineContent: (line: number) => string } | null
    selection: { isEmpty: () => boolean } | null
    line: number | null
}

const createFakeEditor = ({ model, selection, line }: FakeEditorInit) =>
    ({
        getModel: () => model,
        getSelection: () => selection,
        getPosition: () => (line === null ? null : { lineNumber: line }),
    }) as unknown as editor.ICodeEditor

describe('resolveSelectedTextOrCurrentLine', () => {
    test('선택 영역이 비어있지 않으면 선택된 텍스트를 반환한다', () => {
        const fakeEditor = createFakeEditor({
            model: { getValueInRange: () => 'const selected = 1', getLineContent: () => '전체 줄' },
            selection: { isEmpty: () => false },
            line: 3,
        })

        expect(resolveSelectedTextOrCurrentLine(fakeEditor)).toBe('const selected = 1')
    })

    test('선택 영역이 비어있으면 커서가 위치한 줄 전체를 반환한다', () => {
        const fakeEditor = createFakeEditor({
            model: { getValueInRange: () => '', getLineContent: (line) => `line ${line}` },
            selection: { isEmpty: () => true },
            line: 5,
        })

        expect(resolveSelectedTextOrCurrentLine(fakeEditor)).toBe('line 5')
    })

    test('선택이 아예 없으면(null) 커서가 위치한 줄 전체를 반환한다', () => {
        const fakeEditor = createFakeEditor({
            model: { getValueInRange: () => '', getLineContent: (line) => `line ${line}` },
            selection: null,
            line: 7,
        })

        expect(resolveSelectedTextOrCurrentLine(fakeEditor)).toBe('line 7')
    })

    test('모델이 없으면 null 을 반환한다', () => {
        const fakeEditor = createFakeEditor({ model: null, selection: null, line: 1 })

        expect(resolveSelectedTextOrCurrentLine(fakeEditor)).toBeNull()
    })

    test('커서 위치가 없으면(포커스 상실 등) null 을 반환한다', () => {
        const fakeEditor = createFakeEditor({
            model: { getValueInRange: () => '', getLineContent: () => '' },
            selection: null,
            line: null,
        })

        expect(resolveSelectedTextOrCurrentLine(fakeEditor)).toBeNull()
    })
})
