import { describe, expect, test } from 'bun:test'
import { fileNameOf } from '@shared/lib/relative-path'

describe('fileNameOf', () => {
    test('경로에서 마지막 세그먼트를 파일명으로 반환한다', () => {
        expect(fileNameOf('/project/src/widgets/search-editor/search-editor-pane.tsx')).toBe('search-editor-pane.tsx')
    })

    test('슬래시가 없으면 전체 문자열을 그대로 반환한다', () => {
        expect(fileNameOf('package.json')).toBe('package.json')
    })

    test('디렉토리로 끝나면 빈 문자열을 반환한다', () => {
        expect(fileNameOf('/project/src/widgets/')).toBe('')
    })
})
