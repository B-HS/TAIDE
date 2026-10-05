import { useEffect, useRef } from 'react'
import { createRoot } from 'react-dom/client'
import { createInstance } from 'i18next'
import { initReactI18next, I18nextProvider } from 'react-i18next'
import type { FC } from 'react'
import type { TerminalAttachHandle } from '@features/terminal/terminal-view'
import { TerminalView } from '@features/terminal/terminal-view'
import { TerminalContextMenu } from '@features/terminal/terminal-context-menu'
import '@shared/styles/global.css'

const FONT_SIZE = 14
const SCROLLBACK = 100
const LEFT = 40
const TOP = 40
const WIDTH = 480
const HEIGHT = 280
const MODE = new URL(window.location.href).searchParams.get('mode')
const INITIAL_OUTPUT = new TextEncoder().encode(`Synthetic terminal\r\n\x1b[?1004h${MODE === 'sgr' ? '\x1b[?1000h\x1b[?1006h' : ''}`)
const LOCALE = {
    terminal: { copy: 'Copy', paste: 'Paste', selectAll: 'Select all', clear: 'Clear', kill: 'Kill' },
    tab: { split: 'Split', newTerminal: 'New terminal' },
}
const i18n = createInstance()
await i18n.use(initReactI18next).init({ lng: 'en', resources: { en: { translation: LOCALE } }, interpolation: { escapeValue: false } })

export const TerminalMenuReference: FC = () => {
    const attachRef = useRef<TerminalAttachHandle | null>(null)
    const recordsRef = useRef<HTMLOutputElement | null>(null)
    const readyRef = useRef<HTMLOutputElement | null>(null)
    const handleRecord = (record: string) => {
        if (recordsRef.current) recordsRef.current.textContent += `${record}\n`
    }
    const handleReady = () => {
        queueMicrotask(() => attachRef.current?.write(INITIAL_OUTPUT, INITIAL_OUTPUT.byteLength))
    }
    const handleBacklog = (pending: number) => {
        if (pending === 0 && readyRef.current) readyRef.current.textContent = 'ready'
    }
    const handleRestoreFocus = () => {
        handleRecord('restore-focus')
        attachRef.current?.focus()
    }

    useEffect(() => {
        const eventNames = ['pointerdown', 'mousedown', 'contextmenu', 'pointerup', 'mouseup', 'focus', 'blur', 'keydown', 'keyup'] as const
        const handleEvent = (event: Event) => {
            const target = event.target instanceof Element ? event.target : null
            const owner = target?.closest('[role="menu"]') ? 'menu' : target?.tagName.toLowerCase()
            const button = event instanceof MouseEvent ? `:${event.button}` : ''
            const key = event instanceof KeyboardEvent ? `:${event.key}` : ''
            handleRecord(`dom:${event.type}:${owner}${button}${key}:trusted=${event.isTrusted}`)
        }
        for (const name of eventNames) document.addEventListener(name, handleEvent, true)
        return () => {
            for (const name of eventNames) document.removeEventListener(name, handleEvent, true)
        }
    }, [])

    return (
        <I18nextProvider i18n={i18n}>
            <output data-records ref={recordsRef} hidden />
            <output data-ready ref={readyRef} hidden />
            <TerminalContextMenu
                canCopy={false}
                canPaste={false}
                splitAvailability={{ left: true, right: true, top: true, bottom: true }}
                onOpenChange={(open) => handleRecord(`menu:${open ? 'open' : 'closed'}`)}
                onRestoreFocus={handleRestoreFocus}
                onCopy={() => handleRecord('action:copy')}
                onPaste={() => handleRecord('action:paste')}
                onSelectAll={() => handleRecord('action:select-all')}
                onClear={() => handleRecord('action:clear')}
                onSplit={(edge) => handleRecord(`action:split:${edge}`)}
                onNewTerminal={() => handleRecord('action:new')}
                onKill={() => handleRecord('action:kill')}>
                <div data-terminal style={{ position: 'fixed', left: LEFT, top: TOP, width: WIDTH, height: HEIGHT }}>
                    <TerminalView
                        autoFocus={false}
                        fontSize={FONT_SIZE}
                        fontFamily='monospace'
                        theme={{ background: '#000000', foreground: '#ffffff' }}
                        scrollback={SCROLLBACK}
                        cursorStyle='block'
                        cursorBlink={false}
                        commandSuccessColor={null}
                        commandFailureColor={null}
                        onData={(data) => handleRecord(`data:${JSON.stringify(data)}`)}
                        onResize={(cols, rows) => handleRecord(`resize:${cols}:${rows}`)}
                        onReady={handleReady}
                        onWriteBacklogChange={handleBacklog}
                        onFocusChange={(focused) => handleRecord(`terminal-focus:${focused}`)}
                        onOpenLink={() => handleRecord('action:link')}
                        onOpenFileLink={() => handleRecord('action:file-link')}
                        getCwd={() => null}
                        resolveFileLinkCandidates={async () => []}
                        attachRef={attachRef}
                    />
                </div>
            </TerminalContextMenu>
        </I18nextProvider>
    )
}

const root = document.getElementById('root')
if (!root) throw new Error('Missing synthetic terminal root')
createRoot(root).render(<TerminalMenuReference />)
