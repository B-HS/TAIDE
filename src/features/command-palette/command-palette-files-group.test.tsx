import { describe, expect, test } from 'bun:test'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { Command, CommandList } from '@shared/ui/command'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'
import { CommandPaletteFilesGroup } from '@features/command-palette/command-palette-files-group'

const COMPOSED_PATH = '/Users/x/project/src/한글/문서.ts'
const DECOMPOSED_PATH = COMPOSED_PATH.normalize('NFD')
const PROJECT_ROOT = '/Users/x/project/'

const renderGroup = (query: string, paths: string[], onOpenFile: (path: string) => void) => {
    const files = fuzzyFilter(query, paths, (path) => path.slice(PROJECT_ROOT.length))

    return renderWithProviders(
        <Command shouldFilter={false}>
            <CommandList>
                <CommandPaletteFilesGroup files={files} isRefreshing={false} onOpenFile={onOpenFile} />
            </CommandList>
        </Command>,
    )
}

/**
 * The palette's file rows carry two different strings on purpose: the NFC-normalized label the fuzzy
 * offsets belong to (what the row shows) and the untouched path the walk returned (what the row
 * opens). macOS names arrive decomposed, so these two differ byte-for-byte while looking identical.
 */
describe('CommandPaletteFilesGroup 경로 표기·열기', () => {
    test('NFD 경로도 NFC 라벨로 그리고, 질의에 맞는 부분만 강조한다', () => {
        renderGroup('문서', [DECOMPOSED_PATH], () => {})

        expect(screen.getByText('src/한글')).toBeDefined()
        expect(screen.getByText('문서').tagName).toBe('MARK')
    })

    test('여는 경로는 라벨이 아니라 원본 경로다', () => {
        const openedPaths: string[] = []
        renderGroup('문서', [DECOMPOSED_PATH], (path) => openedPaths.push(path))

        fireEvent.click(screen.getByRole('option'))

        expect(openedPaths).toEqual([DECOMPOSED_PATH])
    })
})
