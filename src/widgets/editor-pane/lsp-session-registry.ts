import type { LspServerId, ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import type { LspClient } from '@shared/lib/lsp/client'
import { createLspClient } from '@shared/lib/lsp/client'
import { registerCompletion } from '@shared/lib/lsp/adapters/completion'
import { registerDefinition } from '@shared/lib/lsp/adapters/definition'
import { registerDiagnostics } from '@shared/lib/lsp/adapters/diagnostics'
import { registerDocumentSymbol } from '@shared/lib/lsp/adapters/document-symbol'
import { registerFormatting } from '@shared/lib/lsp/adapters/formatting'
import { registerHover } from '@shared/lib/lsp/adapters/hover'
import { registerInlayHints } from '@shared/lib/lsp/adapters/inlay-hints'
import { registerReferences } from '@shared/lib/lsp/adapters/references'
import { registerRename } from '@shared/lib/lsp/adapters/rename'
import { registerSignatureHelp } from '@shared/lib/lsp/adapters/signature-help'
import { sendLspMessage, spawnLspSession, stopLspSession } from '@entities/lsp/lsp.ipc'

type Disposable = { dispose: () => void }

const LANGUAGE_ADAPTER_REGISTRARS = [
    registerCompletion,
    registerDefinition,
    registerDocumentSymbol,
    registerFormatting,
    registerHover,
    registerInlayHints,
    registerReferences,
    registerRename,
    registerSignatureHelp,
]

export type SessionRecord = {
    refCount: number
    root: string
    ready: Promise<{ client: LspClient; sessionId: string }>
    languageDisposables: Map<string, Disposable[]>
    diagnosticsDisposable: Disposable | null
    openDocuments: Map<string, number>
}

const sessionsByKey = new Map<string, SessionRecord>()

const toSessionKey = (projectId: ProjectId, serverId: LspServerId) => `${projectId}::${serverId}`

const toWorkspaceFolderName = (root: string) => root.split('/').filter(Boolean).at(-1) ?? root

const buildInitializeParams = (root: string) => {
    const rootUri = monaco.Uri.file(root).toString()
    return {
        processId: null,
        clientInfo: { name: 'TAIDE' },
        rootUri,
        rootPath: root,
        workspaceFolders: [{ uri: rootUri, name: toWorkspaceFolderName(root) }],
        capabilities: {
            general: { positionEncodings: ['utf-16'] },
            workspace: { workspaceFolders: true, configuration: true },
            textDocument: {
                synchronization: { dynamicRegistration: false, didSave: true },
                completion: { completionItem: { snippetSupport: true }, contextSupport: true },
                hover: { contentFormat: ['markdown', 'plaintext'] },
                signatureHelp: {},
                definition: {},
                references: {},
                documentSymbol: {},
                formatting: {},
                rename: { prepareSupport: true },
                publishDiagnostics: { relatedInformation: true },
                inlayHint: {},
            },
        },
    }
}

const createSession = async (projectId: ProjectId, serverId: LspServerId, root: string) => {
    let sessionId: string | null = null

    const client = createLspClient({
        send: (message) => {
            if (!sessionId) return
            void sendLspMessage({ sessionId, message: JSON.stringify(message) }).catch(() => undefined)
        },
        onNotification: () => undefined,
    })

    sessionId = await spawnLspSession({
        projectId,
        serverId,
        root,
        onMessage: (raw) => {
            try {
                client.handleMessage(JSON.parse(raw))
            } catch {
                return
            }
        },
    })

    await client.initialize(buildInitializeParams(root))

    return { client, sessionId }
}

const disposeSession = async (key: string, record: SessionRecord) => {
    const session = await record.ready.catch(() => null)
    for (const disposables of record.languageDisposables.values()) {
        for (const disposable of disposables) disposable.dispose()
    }
    record.diagnosticsDisposable?.dispose()
    if (!session) return
    session.client.dispose()
    await stopLspSession(session.sessionId, record.root).catch(() => undefined)
    if (sessionsByKey.get(key) === record) sessionsByKey.delete(key)
}

export const acquireLspSession = (projectId: ProjectId, serverId: LspServerId, root: string) => {
    const key = toSessionKey(projectId, serverId)
    const existing = sessionsByKey.get(key)
    if (existing) {
        existing.refCount += 1
        return { key, record: existing }
    }

    const record: SessionRecord = {
        refCount: 1,
        root,
        ready: createSession(projectId, serverId, root),
        languageDisposables: new Map(),
        diagnosticsDisposable: null,
        openDocuments: new Map(),
    }
    void record.ready.catch(() => sessionsByKey.delete(key))
    sessionsByKey.set(key, record)
    return { key, record }
}

export const releaseLspSession = (key: string, record: SessionRecord) => {
    record.refCount -= 1
    if (record.refCount > 0) return
    if (sessionsByKey.get(key) === record) sessionsByKey.delete(key)
    void disposeSession(key, record)
}

export const ensureLanguageRegistered = (record: SessionRecord, client: LspClient, serverId: LspServerId, languageId: string) => {
    if (!record.diagnosticsDisposable) record.diagnosticsDisposable = registerDiagnostics(monaco, client, serverId)
    if (record.languageDisposables.has(languageId)) return

    const disposables = LANGUAGE_ADAPTER_REGISTRARS.map((register) => register(monaco, client, languageId))
    record.languageDisposables.set(languageId, disposables)
}

export const acquireDocument = (record: SessionRecord, client: LspClient, uri: string, languageId: string, text: string) => {
    const current = record.openDocuments.get(uri) ?? 0
    record.openDocuments.set(uri, current + 1)
    if (current === 0) client.didOpen({ uri, languageId, version: 0, text })
}

export const releaseDocument = (record: SessionRecord, client: LspClient, uri: string) => {
    const current = record.openDocuments.get(uri)
    if (!current) return
    if (current <= 1) {
        record.openDocuments.delete(uri)
        client.didClose(uri)
        return
    }
    record.openDocuments.set(uri, current - 1)
}
