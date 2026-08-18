import { describe, expect, test } from 'bun:test'
import { REMOTE_WINDOW_LABEL } from '@shared/lib/remote/tauri-internals-shim'

describe('REMOTE_WINDOW_LABEL', () => {
    test('데스크톱 main 창 라벨(tauri.conf.json)과 겹치지 않는다', () => {
        expect(REMOTE_WINDOW_LABEL).not.toBe('main')
    })

    test('보조 창 라벨 패턴(editor-<n>)과 겹치지 않는다', () => {
        expect(REMOTE_WINDOW_LABEL.startsWith('editor-')).toBe(false)
    })
})
