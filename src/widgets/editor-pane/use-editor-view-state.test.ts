import { describe, expect, test } from 'bun:test'
import { parsePersistedViewState } from '@widgets/editor-pane/use-editor-view-state'

describe('parsePersistedViewState', () => {
    test('null·undefined 는 복원할 것이 없으므로 null 이다', () => {
        expect(parsePersistedViewState(null)).toBeNull()
        expect(parsePersistedViewState(undefined)).toBeNull()
    })

    test('빈 문자열도 복원할 것이 없으므로 null 이다', () => {
        expect(parsePersistedViewState('')).toBeNull()
    })

    test('유효한 JSON 은 그대로 파싱해 돌려준다', () => {
        const raw = JSON.stringify({ cursorState: [], viewState: { scrollTop: 42 }, contributionsState: {} })
        expect(parsePersistedViewState(raw)).toEqual(JSON.parse(raw))
    })

    test('손상된 JSON(수동 편집된 layout.json 등)은 throw 대신 null 로 취급한다', () => {
        expect(parsePersistedViewState('{not valid json')).toBeNull()
    })
})
