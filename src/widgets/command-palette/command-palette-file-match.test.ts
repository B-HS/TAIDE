import { describe, expect, test } from 'bun:test'
import { splitFileMatchForDisplay } from '@widgets/command-palette/command-palette-file-match'

describe('splitFileMatchForDisplay', () => {
    test('디렉토리가 있으면 파일명과 상위 경로로 나눈다', () => {
        const result = splitFileMatchForDisplay('src/widgets/command-palette/command-palette.tsx', [])
        expect(result.fileName).toBe('command-palette.tsx')
        expect(result.dirPath).toBe('src/widgets/command-palette')
    })

    test('프로젝트 루트 직속 파일은 dirPath 가 null 이다', () => {
        const result = splitFileMatchForDisplay('package.json', [])
        expect(result.fileName).toBe('package.json')
        expect(result.dirPath).toBeNull()
    })

    test('루트 직속 파일의 매칭 인덱스는 그대로 fileNameIndices 에 담긴다', () => {
        const result = splitFileMatchForDisplay('package.json', [0, 1, 2])
        expect(result.fileNameIndices).toEqual([0, 1, 2])
        expect(result.dirPathIndices).toEqual([])
    })

    test("디렉토리 부분의 매칭 인덱스는 dirPathIndices 에 그대로 담긴다 — 'src/x.ts' 인덱스 0(s)은 dirPath('src') 안에 있다", () => {
        const result = splitFileMatchForDisplay('src/x.ts', [0])
        expect(result.dirPathIndices).toEqual([0])
        expect(result.fileNameIndices).toEqual([])
    })

    test("파일명 부분의 매칭 인덱스는 구분자 길이만큼 당겨진다 — 'src/x.ts' 인덱스 4(x)는 fileName('x.ts') 의 0번째로 당겨진다", () => {
        const result = splitFileMatchForDisplay('src/x.ts', [4])
        expect(result.fileNameIndices).toEqual([0])
        expect(result.dirPathIndices).toEqual([])
    })

    test('구분자(/) 자체가 매칭 인덱스면 양쪽 어디에도 포함되지 않는다', () => {
        const result = splitFileMatchForDisplay('src/x.ts', [3])
        expect(result.fileNameIndices).toEqual([])
        expect(result.dirPathIndices).toEqual([])
    })

    test("디렉토리·파일명 양쪽에 걸친 매칭 인덱스를 각각 올바르게 분리한다 — 'widgets/pane-node-view.tsx' 구분자(/)는 인덱스 7, 인덱스 0/9/17 은 각각 dirPath·fileName 양쪽에 걸쳐 있다", () => {
        const result = splitFileMatchForDisplay('widgets/pane-node-view.tsx', [0, 9, 17])
        expect(result.dirPath).toBe('widgets')
        expect(result.fileName).toBe('pane-node-view.tsx')
        expect(result.dirPathIndices).toEqual([0])
        expect(result.fileNameIndices).toEqual([1, 9])
    })
})
