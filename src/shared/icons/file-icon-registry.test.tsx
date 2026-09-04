import { describe, expect, test } from 'bun:test'
import type { FileIconName } from '@shared/lib/file-icon'
import { resolveFileIcon, resolveFolderIcon } from '@shared/lib/file-icon'
import { FILE_ICON_COMPONENT_MAP } from '@shared/icons/file-icon-registry'
import { FileTypeIcon } from '@shared/icons/file-type-icon'
import { FolderTypeIcon } from '@shared/icons/folder-type-icon'
import { renderWithProviders } from '@shared/testing/render'

/**
 * The registry is the only bridge between `file-icon.ts`'s icon *names* and real lucide
 * components, and `FileTypeIcon` indexes it with no fallback — a name the map does not carry
 * renders `undefined` as a component, which throws during render and takes the whole explorer row
 * list down with it. TypeScript already forces the map to be total over the union, so what is left
 * to check at runtime is that the union has not silently grown a member the map answers with
 * `undefined`, and that each name resolves to a distinct glyph (a copy-pasted entry pointing two
 * names at one icon type-checks fine and is invisible until someone looks at the tree).
 */
const FILE_ICON_NAMES: FileIconName[] = [
    'file',
    'file-code',
    'component',
    'file-json',
    'book-text',
    'palette',
    'globe',
    'cog',
    'coffee',
    'terminal',
    'file-cog',
    'lock',
    'image',
    'file-text',
    'file-archive',
    'package',
    'settings-2',
    'git-branch',
    'git-fork',
    'container',
    'book-marked',
    'scale',
    'key',
    'folder',
    'folder-open',
    'folder-code',
    'box',
    'flask-conical',
]

/** File names chosen to reach every branch of `resolveFileIcon` — special name, prefix rules, extension table, and the default. */
const REPRESENTATIVE_FILE_NAMES = [
    'package.json',
    'tsconfig.json',
    'Cargo.toml',
    'Cargo.lock',
    '.gitignore',
    'Dockerfile',
    'README.md',
    'LICENSE',
    '.env.local',
    'main.rs',
    'app.tsx',
    'style.css',
    'photo.png',
    'archive.zip',
    'notes',
    'weird.unknownext',
]

const REPRESENTATIVE_FOLDER_NAMES = ['src', '.git', '.github', 'assets', 'node_modules', 'anything-else']

describe('FILE_ICON_COMPONENT_MAP', () => {
    test('알려진 아이콘 이름 전부에 컴포넌트가 있고 그 외 키는 없다', () => {
        expect(Object.keys(FILE_ICON_COMPONENT_MAP).toSorted()).toEqual([...FILE_ICON_NAMES].toSorted())
    })

    test('이름마다 서로 다른 글리프를 가리킨다', () => {
        expect(new Set(Object.values(FILE_ICON_COMPONENT_MAP)).size).toBe(FILE_ICON_NAMES.length)
    })

    test('resolveFileIcon 이 돌려주는 모든 이름이 레지스트리에 있다', () => {
        for (const fileName of REPRESENTATIVE_FILE_NAMES) {
            expect(FILE_ICON_COMPONENT_MAP[resolveFileIcon(fileName).icon]).toBeDefined()
        }
    })

    test('resolveFolderIcon 이 돌려주는 이름은 펼침 여부와 무관하게 레지스트리에 있다', () => {
        for (const folderName of REPRESENTATIVE_FOLDER_NAMES) {
            expect(FILE_ICON_COMPONENT_MAP[resolveFolderIcon(folderName, false).icon]).toBeDefined()
            expect(FILE_ICON_COMPONENT_MAP[resolveFolderIcon(folderName, true).icon]).toBeDefined()
        }
    })
})

describe('FileTypeIcon · FolderTypeIcon', () => {
    test('해석한 색 클래스와 호출부 className 을 합쳐 svg 를 그린다', () => {
        const { container } = renderWithProviders(<FileTypeIcon fileName='package.json' className='size-4' />)
        const icon = container.querySelector('svg')

        expect(icon).not.toBeNull()
        expect(icon?.getAttribute('class')).toContain('size-4')
        expect(icon?.getAttribute('class')).toContain(resolveFileIcon('package.json').colorClass)
    })

    test('이름 규칙에 없는 폴더는 펼침 여부에 따라 다른 글리프를 그린다', () => {
        const { container: collapsed } = renderWithProviders(<FolderTypeIcon folderName='anything-else' expanded={false} />)
        const { container: expanded } = renderWithProviders(<FolderTypeIcon folderName='anything-else' expanded={true} />)

        expect(collapsed.querySelector('svg')?.innerHTML).not.toBe(expanded.querySelector('svg')?.innerHTML)
    })

    test('이름 규칙에 걸린 폴더는 펼쳐도 그 규칙의 글리프를 유지한다', () => {
        const { container: collapsed } = renderWithProviders(<FolderTypeIcon folderName='src' expanded={false} />)
        const { container: expanded } = renderWithProviders(<FolderTypeIcon folderName='src' expanded={true} />)

        expect(collapsed.querySelector('svg')?.innerHTML).toBe(expanded.querySelector('svg')?.innerHTML)
    })

    test('확장자를 모르는 파일도 기본 아이콘으로 렌더된다 (레지스트리 미스로 인한 크래시 없음)', () => {
        const { container } = renderWithProviders(<FileTypeIcon fileName='weird.unknownext' />)

        expect(container.querySelector('svg')).not.toBeNull()
    })
})
