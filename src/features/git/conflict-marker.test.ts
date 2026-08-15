import { describe, expect, test } from 'bun:test'
import { acceptBothChanges, acceptCurrentChange, acceptIncomingChange, parseConflictMarkers } from '@features/git/conflict-marker'

describe('parseConflictMarkers', () => {
    test('마커가 없으면 빈 배열을 반환한다', () => {
        expect(parseConflictMarkers('a\nb\nc')).toEqual([])
    })

    test('단일 충돌 구간의 라인 번호와 라벨을 파싱한다', () => {
        const content = ['line1', '<<<<<<< HEAD', 'ours A', 'ours B', '=======', 'theirs A', '>>>>>>> feature', 'line8'].join('\n')

        expect(parseConflictMarkers(content)).toEqual([
            { startLine: 2, baseLine: null, separatorLine: 5, endLine: 7, oursLabel: 'HEAD', theirsLabel: 'feature' },
        ])
    })

    test('한 파일에 여러 충돌 구간이 있으면 각각 독립적으로 파싱한다', () => {
        const content = [
            'a',
            '<<<<<<< HEAD',
            'ours1',
            '=======',
            'theirs1',
            '>>>>>>> feature',
            'b',
            '<<<<<<< HEAD',
            'ours2',
            '=======',
            'theirs2',
            '>>>>>>> feature',
            'c',
        ].join('\n')

        const regions = parseConflictMarkers(content)

        expect(regions).toHaveLength(2)
        expect(regions[0]).toMatchObject({ startLine: 2, separatorLine: 4, endLine: 6 })
        expect(regions[1]).toMatchObject({ startLine: 8, separatorLine: 10, endLine: 12 })
    })

    test('diff3 스타일의 base 마커를 인식한다', () => {
        const content = ['x', '<<<<<<< HEAD', 'ours', '||||||| merged common ancestors', 'base', '=======', 'theirs', '>>>>>>> feature', 'y'].join(
            '\n',
        )

        expect(parseConflictMarkers(content)).toEqual([
            { startLine: 2, baseLine: 4, separatorLine: 6, endLine: 8, oursLabel: 'HEAD', theirsLabel: 'feature' },
        ])
    })

    test('종료 마커가 없는 미완성 충돌 구간은 무시한다', () => {
        const content = ['a', '<<<<<<< HEAD', 'ours only, no end', 'b'].join('\n')

        expect(parseConflictMarkers(content)).toEqual([])
    })

    test('CRLF 줄바꿈에서도 마커를 인식한다', () => {
        const content = ['<<<<<<< HEAD', 'ours', '=======', 'theirs', '>>>>>>> feature'].join('\r\n')

        const regions = parseConflictMarkers(content)

        expect(regions).toHaveLength(1)
        expect(regions[0]).toMatchObject({ startLine: 1, separatorLine: 3, endLine: 5 })
    })
})

describe('acceptCurrentChange', () => {
    test('충돌 구간을 ours 쪽 내용으로만 치환한다', () => {
        const content = ['line1', '<<<<<<< HEAD', 'ours A', 'ours B', '=======', 'theirs A', '>>>>>>> feature', 'line8'].join('\n')
        const [region] = parseConflictMarkers(content)

        expect(acceptCurrentChange(content, region)).toBe(['line1', 'ours A', 'ours B', 'line8'].join('\n'))
    })

    test('다른 충돌 구간은 건드리지 않는다', () => {
        const content = [
            'a',
            '<<<<<<< HEAD',
            'ours1',
            '=======',
            'theirs1',
            '>>>>>>> feature',
            'b',
            '<<<<<<< HEAD',
            'ours2',
            '=======',
            'theirs2',
            '>>>>>>> feature',
            'c',
        ].join('\n')
        const [firstRegion] = parseConflictMarkers(content)

        expect(acceptCurrentChange(content, firstRegion)).toBe(
            ['a', 'ours1', 'b', '<<<<<<< HEAD', 'ours2', '=======', 'theirs2', '>>>>>>> feature', 'c'].join('\n'),
        )
    })

    test('파일 끝에 개행이 없어도 결과에 개행을 추가하지 않는다', () => {
        const content = 'a\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> feature'
        const [region] = parseConflictMarkers(content)

        expect(acceptCurrentChange(content, region)).toBe('a\nours')
    })

    test('ours 쪽이 비어 있는 충돌도 처리한다', () => {
        const content = ['<<<<<<< HEAD', '=======', 'theirs', '>>>>>>> feature'].join('\n')
        const [region] = parseConflictMarkers(content)

        expect(acceptCurrentChange(content, region)).toBe('')
    })
})

describe('acceptIncomingChange', () => {
    test('충돌 구간을 theirs 쪽 내용으로만 치환한다', () => {
        const content = ['line1', '<<<<<<< HEAD', 'ours A', 'ours B', '=======', 'theirs A', '>>>>>>> feature', 'line8'].join('\n')
        const [region] = parseConflictMarkers(content)

        expect(acceptIncomingChange(content, region)).toBe(['line1', 'theirs A', 'line8'].join('\n'))
    })
})

describe('acceptBothChanges', () => {
    test('ours 다음 theirs 순서로 두 내용을 모두 남긴다', () => {
        const content = ['line1', '<<<<<<< HEAD', 'ours A', 'ours B', '=======', 'theirs A', '>>>>>>> feature', 'line8'].join('\n')
        const [region] = parseConflictMarkers(content)

        expect(acceptBothChanges(content, region)).toBe(['line1', 'ours A', 'ours B', 'theirs A', 'line8'].join('\n'))
    })

    test('diff3 base 본문은 결과에 포함하지 않는다', () => {
        const content = ['x', '<<<<<<< HEAD', 'ours', '||||||| base', 'base body', '=======', 'theirs', '>>>>>>> feature', 'y'].join('\n')
        const [region] = parseConflictMarkers(content)

        expect(acceptBothChanges(content, region)).toBe(['x', 'ours', 'theirs', 'y'].join('\n'))
    })
})
