import { describe, expect, test } from 'bun:test'
import type { ReplaceSkippedFile } from '@shared/api/bindings'
import { buildReplaceSkipReport, REPLACE_SKIP_LIST_LIMIT, REPLACE_SKIP_REASON_MESSAGE_KEY } from '@entities/search/replace-skip-report'

const describeFile = (file: ReplaceSkippedFile) => `${file.path}:${file.reason}`

const skippedFiles = (count: number): ReplaceSkippedFile[] =>
    Array.from({ length: count }, (_, index) => ({ path: `src/file-${index}.bin`, reason: 'binary' }))

describe('buildReplaceSkipReport', () => {
    test('스킵이 없으면 리포트를 만들지 않는다', () => {
        expect(buildReplaceSkipReport({ skipped: [], skippedCount: 0 }, describeFile)).toBeNull()
    })

    test('스킵된 파일을 사유와 함께 나열한다', () => {
        const report = buildReplaceSkipReport(
            {
                skipped: [
                    { path: 'a.bin', reason: 'binary' },
                    { path: 'b.txt', reason: 'notUtf8' },
                ],
                skippedCount: 2,
            },
            describeFile,
        )

        expect(report).toEqual({ total: 2, lines: ['a.bin:binary', 'b.txt:notUtf8'], remaining: 0 })
    })

    test('나열 상한을 넘으면 남은 개수를 따로 알린다', () => {
        const listed = REPLACE_SKIP_LIST_LIMIT + 3
        const report = buildReplaceSkipReport({ skipped: skippedFiles(listed), skippedCount: listed }, describeFile)

        expect(report?.lines).toHaveLength(REPLACE_SKIP_LIST_LIMIT)
        expect(report?.remaining).toBe(3)
    })

    test('백엔드가 목록을 잘라 보낸 경우에도 전체 개수를 기준으로 남은 수를 센다', () => {
        const report = buildReplaceSkipReport({ skipped: skippedFiles(REPLACE_SKIP_LIST_LIMIT), skippedCount: 120 }, describeFile)

        expect(report?.total).toBe(120)
        expect(report?.remaining).toBe(120 - REPLACE_SKIP_LIST_LIMIT)
    })
})

describe('REPLACE_SKIP_REASON_MESSAGE_KEY', () => {
    test('모든 스킵 사유가 로케일 키를 가진다', () => {
        expect(Object.values(REPLACE_SKIP_REASON_MESSAGE_KEY).every((key) => key.startsWith('search.replaceSkipReason.'))).toBe(true)
        expect(Object.keys(REPLACE_SKIP_REASON_MESSAGE_KEY)).toEqual(['tooLarge', 'binary', 'notUtf8', 'unreadable', 'writeFailed'])
    })
})
