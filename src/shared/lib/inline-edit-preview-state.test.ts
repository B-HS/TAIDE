import { describe, expect, test } from 'bun:test'
import type { AiInlineEditPreviewState } from '@shared/lib/inline-edit-preview-state'
import { AI_INLINE_EDIT_PREVIEW_IDLE_STATE, advanceAiInlineEditPreview } from '@shared/lib/inline-edit-preview-state'

describe('advanceAiInlineEditPreview', () => {
    test('idle 상태에서 submit 하면 loading 으로 전이한다', () => {
        expect(advanceAiInlineEditPreview(AI_INLINE_EDIT_PREVIEW_IDLE_STATE, { type: 'submit' })).toEqual({ status: 'loading' })
    })

    test('loading 상태에서 resolve 하면 응답 텍스트를 담은 preview 로 전이한다', () => {
        const loading: AiInlineEditPreviewState = { status: 'loading' }
        expect(advanceAiInlineEditPreview(loading, { type: 'resolve', text: 'const a = 1' })).toEqual({
            status: 'preview',
            text: 'const a = 1',
        })
    })

    test('loading 상태에서 rejectResponse(빈 응답·실패) 하면 idle 로 되돌아간다', () => {
        const loading: AiInlineEditPreviewState = { status: 'loading' }
        expect(advanceAiInlineEditPreview(loading, { type: 'rejectResponse' })).toEqual(AI_INLINE_EDIT_PREVIEW_IDLE_STATE)
    })

    test('loading 상태에서 cancel 하면 idle 로 되돌아간다', () => {
        const loading: AiInlineEditPreviewState = { status: 'loading' }
        expect(advanceAiInlineEditPreview(loading, { type: 'cancel' })).toEqual(AI_INLINE_EDIT_PREVIEW_IDLE_STATE)
    })

    test('preview 상태에서 accept 하면 idle 로 되돌아간다', () => {
        const preview: AiInlineEditPreviewState = { status: 'preview', text: 'const a = 1' }
        expect(advanceAiInlineEditPreview(preview, { type: 'accept' })).toEqual(AI_INLINE_EDIT_PREVIEW_IDLE_STATE)
    })

    test('preview 상태에서 reject 하면 idle 로 되돌아간다', () => {
        const preview: AiInlineEditPreviewState = { status: 'preview', text: 'const a = 1' }
        expect(advanceAiInlineEditPreview(preview, { type: 'reject' })).toEqual(AI_INLINE_EDIT_PREVIEW_IDLE_STATE)
    })

    test('preview 상태에서 문서 편집으로 invalidate 되면 idle 로 되돌아간다', () => {
        const preview: AiInlineEditPreviewState = { status: 'preview', text: 'const a = 1' }
        expect(advanceAiInlineEditPreview(preview, { type: 'invalidate' })).toEqual(AI_INLINE_EDIT_PREVIEW_IDLE_STATE)
    })

    test('loading 상태에서 문서 편집으로 invalidate 되면 idle 로 되돌아간다 (응답 도착 전 취소)', () => {
        const loading: AiInlineEditPreviewState = { status: 'loading' }
        expect(advanceAiInlineEditPreview(loading, { type: 'invalidate' })).toEqual(AI_INLINE_EDIT_PREVIEW_IDLE_STATE)
    })

    test('idle 상태에서 submit 이 아닌 이벤트는 무시된다', () => {
        expect(advanceAiInlineEditPreview(AI_INLINE_EDIT_PREVIEW_IDLE_STATE, { type: 'accept' })).toEqual(AI_INLINE_EDIT_PREVIEW_IDLE_STATE)
    })

    test('preview 상태에서 submit 은 무시되고 그대로 유지된다 (진행 중 재제출 불가)', () => {
        const preview: AiInlineEditPreviewState = { status: 'preview', text: 'const a = 1' }
        expect(advanceAiInlineEditPreview(preview, { type: 'submit' })).toEqual(preview)
    })

    test('loading 상태에서 resolve 이외의 재제출은 무시된다', () => {
        const loading: AiInlineEditPreviewState = { status: 'loading' }
        expect(advanceAiInlineEditPreview(loading, { type: 'submit' })).toEqual(loading)
    })
})
