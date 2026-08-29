import { describe, expect, test } from 'bun:test'
import { INSERT_FINAL_NEW_LINE_ACTION_ID, TRIM_TRAILING_WHITESPACE_ACTION_ID } from '@shared/lib/monaco/monaco-actions'
import { runOnSaveCleanup } from '@shared/lib/monaco/on-save-cleanup'

type RecordedRun = { actionId: string; args: unknown }

/**
 * `run` is written as a plain function returning a promise rather than an `async` one so
 * `throwingSynchronously` can reproduce monaco's real failure shape: `InternalEditorAction.run`
 * forwards to a synchronous `invokeFunction`, so an action that fails before its own
 * `Promise.resolve` throws out of `run()` instead of returning a rejected promise.
 */
const createFakeEditor = (
    runs: RecordedRun[],
    options: { failing?: boolean; throwingSynchronously?: boolean; missingActionIds?: string[] } = {},
) => ({
    getAction: (actionId: string) =>
        options.missingActionIds?.includes(actionId)
            ? null
            : {
                  id: actionId,
                  label: actionId,
                  alias: actionId,
                  metadata: undefined,
                  isSupported: () => true,
                  run: (args?: unknown) => {
                      runs.push({ actionId, args })
                      if (options.throwingSynchronously) throw new Error('action threw synchronously')
                      return options.failing ? Promise.reject(new Error('action failed')) : Promise.resolve()
                  },
              },
})

describe('runOnSaveCleanup (d-53 U2 — 저장 파이프라인 공통 on-save 정리)', () => {
    test('두 설정이 모두 꺼져 있으면(기본값) 어떤 액션도 실행하지 않는다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs),
            trimTrailingWhitespaceOnSave: false,
            insertFinalNewlineOnSave: false,
            isAutoSave: false,
        })

        expect(runs).toEqual([])
    })

    test('설정이 undefined 여도(설정 쿼리 미도착) 아무 것도 실행하지 않는다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs),
            trimTrailingWhitespaceOnSave: undefined,
            insertFinalNewlineOnSave: undefined,
            isAutoSave: false,
        })

        expect(runs).toEqual([])
    })

    test('둘 다 켜면 후행 공백 제거 → 마지막 줄바꿈 순서로 실행한다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs),
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: true,
            isAutoSave: false,
        })

        expect(runs.map((run) => run.actionId)).toEqual([TRIM_TRAILING_WHITESPACE_ACTION_ID, INSERT_FINAL_NEW_LINE_ACTION_ID])
    })

    test('켠 설정의 액션만 실행한다', async () => {
        const trimOnlyRuns: RecordedRun[] = []
        const newlineOnlyRuns: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(trimOnlyRuns),
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: false,
            isAutoSave: false,
        })
        await runOnSaveCleanup({
            editor: createFakeEditor(newlineOnlyRuns),
            trimTrailingWhitespaceOnSave: false,
            insertFinalNewlineOnSave: true,
            isAutoSave: false,
        })

        expect(trimOnlyRuns.map((run) => run.actionId)).toEqual([TRIM_TRAILING_WHITESPACE_ACTION_ID])
        expect(newlineOnlyRuns.map((run) => run.actionId)).toEqual([INSERT_FINAL_NEW_LINE_ACTION_ID])
    })

    test('수동 저장(⌘S)은 인자 없이 실행해 커서가 있는 줄까지 정리한다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs),
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: false,
            isAutoSave: false,
        })

        expect(runs[0]?.args).toBeUndefined()
    })

    test('자동 저장은 타이핑 중인 커서 줄의 공백을 남기도록 auto-save 사유를 넘긴다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs),
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: false,
            isAutoSave: true,
        })

        expect(runs[0]?.args).toEqual({ reason: 'auto-save' })
    })

    test('마지막 줄바꿈 액션에는 사유를 넘기지 않는다(자동 저장에서도 동일)', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs),
            trimTrailingWhitespaceOnSave: false,
            insertFinalNewlineOnSave: true,
            isAutoSave: true,
        })

        expect(runs[0]?.args).toBeUndefined()
    })

    test('editor 가 없으면(마운트 전·프리뷰 전용 탭) 조용히 건너뛴다', async () => {
        await runOnSaveCleanup({ editor: null, trimTrailingWhitespaceOnSave: true, insertFinalNewlineOnSave: true, isAutoSave: false })
    })

    test('액션이 등록돼 있지 않으면 건너뛰고 나머지 단계는 계속한다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs, { missingActionIds: [TRIM_TRAILING_WHITESPACE_ACTION_ID] }),
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: true,
            isAutoSave: false,
        })

        expect(runs.map((run) => run.actionId)).toEqual([INSERT_FINAL_NEW_LINE_ACTION_ID])
    })

    test('액션이 실패해도 저장을 막지 않는다 — 실패를 삼키고 다음 단계로 넘어간다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs, { failing: true }),
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: true,
            isAutoSave: false,
        })

        expect(runs.map((run) => run.actionId)).toEqual([TRIM_TRAILING_WHITESPACE_ACTION_ID, INSERT_FINAL_NEW_LINE_ACTION_ID])
    })

    test('액션이 동기적으로 던져도(monaco 의 invokeFunction 경로) 삼키고 다음 단계로 넘어간다', async () => {
        const runs: RecordedRun[] = []

        await runOnSaveCleanup({
            editor: createFakeEditor(runs, { throwingSynchronously: true }),
            trimTrailingWhitespaceOnSave: true,
            insertFinalNewlineOnSave: true,
            isAutoSave: false,
        })

        expect(runs.map((run) => run.actionId)).toEqual([TRIM_TRAILING_WHITESPACE_ACTION_ID, INSERT_FINAL_NEW_LINE_ACTION_ID])
    })
})
