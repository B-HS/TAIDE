import { describe, expect, test } from 'bun:test'
import { resolveDiffViewSettingsProps } from '@shared/lib/diff-view-settings'

describe('resolveDiffViewSettingsProps', () => {
    test('settings 가 없으면 두 옵션 모두 VS Code 파리티 기본값 off 다', () => {
        expect(resolveDiffViewSettingsProps(undefined)).toEqual({ hideUnchangedRegions: false, showMoves: false })
    })

    test('settings 값이 있으면 그대로 사용한다', () => {
        const props = resolveDiffViewSettingsProps({ editorDiffHideUnchangedRegions: true, editorDiffShowMoves: true })
        expect(props).toEqual({ hideUnchangedRegions: true, showMoves: true })
    })

    test('필드가 비어 있으면 기본값으로 폴백한다', () => {
        expect(resolveDiffViewSettingsProps({})).toEqual({ hideUnchangedRegions: false, showMoves: false })
    })
})
