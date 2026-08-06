import { describe, expect, test } from 'bun:test'
import { diagnosticsOwnerFor } from '@shared/lib/lsp/adapters/diagnostics'

describe('diagnosticsOwnerFor', () => {
    test('서버 id 별로 서로 다른 marker owner 를 만든다', () => {
        expect(diagnosticsOwnerFor('basedPyright')).toBe('lsp-basedPyright')
        expect(diagnosticsOwnerFor('ruff')).toBe('lsp-ruff')
    })

    test('같은 서버 id 는 같은 owner 를 반환한다', () => {
        expect(diagnosticsOwnerFor('vtsls')).toBe(diagnosticsOwnerFor('vtsls'))
    })

    test('서로 다른 서버의 owner 는 겹치지 않는다', () => {
        expect(diagnosticsOwnerFor('basedPyright')).not.toBe(diagnosticsOwnerFor('ruff'))
    })
})
