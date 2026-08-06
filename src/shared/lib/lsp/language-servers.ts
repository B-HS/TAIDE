import type { LspServerId } from '@shared/api/bindings'

export const LANGUAGE_SERVERS_BY_LANGUAGE_ID: Record<string, LspServerId[] | undefined> = {
    typescript: ['vtsls'],
    typescriptreact: ['vtsls'],
    javascript: ['vtsls'],
    javascriptreact: ['vtsls'],
    rust: ['rustAnalyzer'],
    python: ['basedPyright', 'ruff'],
    markdown: ['marksman'],
}
