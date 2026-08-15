import { describe, expect, test } from 'bun:test'
import type { languages } from 'monaco-editor'
import type { TreeRow } from '@shared/api/bindings'
import {
    buildSegmentPaths,
    containsPosition,
    filterDirectChildren,
    findEnclosingSymbolChain,
    parentDirOf,
    splitRelativePathSegments,
} from '@widgets/editor-pane/breadcrumb-path'

const range = (startLineNumber: number, startColumn: number, endLineNumber: number, endColumn: number) => ({
    startLineNumber,
    startColumn,
    endLineNumber,
    endColumn,
})

const symbol = (name: string, symbolRange: ReturnType<typeof range>, children?: languages.DocumentSymbol[]): languages.DocumentSymbol => ({
    name,
    detail: '',
    kind: 11,
    tags: [],
    range: symbolRange,
    selectionRange: symbolRange,
    children,
})

describe('containsPosition', () => {
    test('range 시작줄보다 앞이면 false 를 반환한다', () => {
        expect(containsPosition(range(5, 1, 10, 1), { lineNumber: 4, column: 1 })).toBe(false)
    })

    test('range 끝줄보다 뒤면 false 를 반환한다', () => {
        expect(containsPosition(range(5, 1, 10, 1), { lineNumber: 11, column: 1 })).toBe(false)
    })

    test('시작줄과 같지만 시작컬럼보다 앞이면 false 를 반환한다', () => {
        expect(containsPosition(range(5, 10, 10, 1), { lineNumber: 5, column: 9 })).toBe(false)
    })

    test('끝줄과 같지만 끝컬럼보다 뒤면 false 를 반환한다', () => {
        expect(containsPosition(range(5, 1, 10, 10), { lineNumber: 10, column: 11 })).toBe(false)
    })

    test('경계값(시작/끝 정확히 그 컬럼)을 포함한다', () => {
        expect(containsPosition(range(5, 10, 5, 20), { lineNumber: 5, column: 10 })).toBe(true)
        expect(containsPosition(range(5, 10, 5, 20), { lineNumber: 5, column: 20 })).toBe(true)
    })

    test('시작줄과 끝줄 사이의 중간 줄은 컬럼과 무관하게 포함한다', () => {
        expect(containsPosition(range(5, 10, 10, 5), { lineNumber: 7, column: 1 })).toBe(true)
    })
})

describe('findEnclosingSymbolChain', () => {
    test('빈 심볼 목록이면 빈 배열을 반환한다', () => {
        expect(findEnclosingSymbolChain([], { lineNumber: 1, column: 1 })).toEqual([])
    })

    test('어떤 심볼 range 에도 속하지 않으면 빈 배열을 반환한다', () => {
        const symbols = [symbol('foo', range(1, 1, 5, 1))]
        expect(findEnclosingSymbolChain(symbols, { lineNumber: 10, column: 1 })).toEqual([])
    })

    test('자식이 없는 최상위 심볼이면 해당 심볼 하나만 반환한다', () => {
        const target = symbol('foo', range(1, 1, 5, 1))
        expect(findEnclosingSymbolChain([target], { lineNumber: 3, column: 1 })).toEqual([target])
    })

    test('중첩 심볼이면 바깥→안쪽 순서의 체인을 반환한다', () => {
        const inner = symbol('inner', range(2, 1, 4, 1))
        const outer = symbol('outer', range(1, 1, 5, 1), [inner])
        const chain = findEnclosingSymbolChain([outer], { lineNumber: 3, column: 1 })
        expect(chain).toEqual([outer, inner])
    })

    test('부모 range 안이지만 어느 자식 range 에도 속하지 않으면 부모까지만 반환한다', () => {
        const inner = symbol('inner', range(2, 1, 3, 1))
        const outer = symbol('outer', range(1, 1, 10, 1), [inner])
        const chain = findEnclosingSymbolChain([outer], { lineNumber: 8, column: 1 })
        expect(chain).toEqual([outer])
    })

    test('형제 심볼 중 커서를 포함하는 것만 선택한다', () => {
        const first = symbol('first', range(1, 1, 3, 1))
        const second = symbol('second', range(4, 1, 6, 1))
        expect(findEnclosingSymbolChain([first, second], { lineNumber: 5, column: 1 })).toEqual([second])
    })
})

describe('splitRelativePathSegments', () => {
    test('슬래시로 구분된 세그먼트 배열을 반환한다', () => {
        expect(splitRelativePathSegments('src/widgets/editor-pane/editor-pane.tsx')).toEqual(['src', 'widgets', 'editor-pane', 'editor-pane.tsx'])
    })

    test('빈 세그먼트(중복 슬래시)를 제거한다', () => {
        expect(splitRelativePathSegments('/src//foo.ts/')).toEqual(['src', 'foo.ts'])
    })
})

describe('buildSegmentPaths', () => {
    test('root 에 트레일링 슬래시가 없어도 각 세그먼트 깊이별 절대경로를 만든다', () => {
        expect(buildSegmentPaths('/project', ['src', 'widgets', 'foo.ts'])).toEqual([
            '/project/src',
            '/project/src/widgets',
            '/project/src/widgets/foo.ts',
        ])
    })

    test('root 에 트레일링 슬래시가 있으면 정규화한다', () => {
        expect(buildSegmentPaths('/project/', ['src', 'foo.ts'])).toEqual(['/project/src', '/project/src/foo.ts'])
    })
})

describe('parentDirOf', () => {
    test('중첩 경로의 부모 디렉터리를 반환한다', () => {
        expect(parentDirOf('/project/src/foo.ts')).toBe('/project/src')
    })

    test('루트 바로 아래 경로는 루트를 반환한다', () => {
        expect(parentDirOf('/project')).toBe('/')
    })
})

describe('filterDirectChildren', () => {
    const rows: TreeRow[] = [
        { path: '/project/src', name: 'src', kind: 'directory', depth: 0, expanded: true, hasChildren: true },
        { path: '/project/src/foo.ts', name: 'foo.ts', kind: 'file', depth: 1, expanded: false, hasChildren: false },
        { path: '/project/src/bar.ts', name: 'bar.ts', kind: 'file', depth: 1, expanded: false, hasChildren: false },
        { path: '/project/src/widgets', name: 'widgets', kind: 'directory', depth: 1, expanded: false, hasChildren: true },
        { path: '/project/src/widgets/deep.ts', name: 'deep.ts', kind: 'file', depth: 2, expanded: false, hasChildren: false },
        { path: '/project/README.md', name: 'README.md', kind: 'file', depth: 0, expanded: false, hasChildren: false },
    ]

    test('해당 부모 경로의 직계 자식만 반환한다', () => {
        const children = filterDirectChildren(rows, '/project/src')
        expect(children.map((row) => row.path)).toEqual(['/project/src/foo.ts', '/project/src/bar.ts', '/project/src/widgets'])
    })

    test('더 깊은 손자 항목은 포함하지 않는다', () => {
        const children = filterDirectChildren(rows, '/project/src')
        expect(children.some((row) => row.path === '/project/src/widgets/deep.ts')).toBe(false)
    })

    test('일치하는 부모가 없으면 빈 배열을 반환한다', () => {
        expect(filterDirectChildren(rows, '/project/other')).toEqual([])
    })
})
