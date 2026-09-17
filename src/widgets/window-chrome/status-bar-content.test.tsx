import { afterAll, beforeAll, describe, expect, mock, spyOn, test } from 'bun:test'
import type { Settings } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { TooltipProvider } from '@shared/ui/tooltip'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The status bar's font controls stay, its keymap registration does not. ⌘=/⌘− now belong to
 * `KeybindingsRuntimeProvider`, which the main window mounts alongside this bar — leaving the old
 * `useGlobalKeymap({ 'font-size-up', 'font-size-down' })` here would have run the handler twice per
 * press in the main window and jumped the size two steps (audit #14/#15). Both halves are asserted:
 * the key does nothing here, the buttons still do.
 *
 * monaco is stubbed before the widget is pulled in through a *dynamic* `import()`; this component
 * reads `monaco.MarkerSeverity` for its error count and `use-monaco-markers.ts` builds a counts
 * object from all four at subscribe time.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: {
        Uri: { file: () => ({ toString: () => '' }) },
        editor: { getModelMarkers: () => [], onDidChangeMarkers: () => undefined },
        MarkerSeverity: { Hint: 1, Info: 2, Warning: 4, Error: 8 },
    },
}))

const importStatusBar = () => import('@widgets/window-chrome/status-bar-content')

const PROJECT_ID = 'project-1'
const SEEDED_EDITOR_FONT_SIZE = 20
const INCREASED_EDITOR_FONT_SIZE = 21
const FAKE_EVENT_ID = 1
const TAURI_EVENT_PLUGIN_INTERNALS_KEY = '__TAURI_EVENT_PLUGIN_INTERNALS__'
const EVENT_PLUGIN_COMMAND_PREFIX = 'plugin:event'
const IPC_UNAVAILABLE_ERROR = { code: 'Unknown', message: 'no backend in the test harness' }

const fakeInvoke = (command: string) =>
    command.startsWith(EVENT_PLUGIN_COMMAND_PREFIX) ? Promise.resolve(FAKE_EVENT_ID) : Promise.reject(IPC_UNAVAILABLE_ERROR)

const pressKey = (init: KeyboardEventInit) => window.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }))

const FONT_SIZE_UP_KEY: KeyboardEventInit = { key: '=', code: 'Equal', metaKey: true }

/** `TooltipProvider` is composed in because `renderWithProviders` carries only Query + i18n and the bar's controls are radix tooltips. */
const renderStatusBar = async () => {
    const queryClient = createTestQueryClient()
    queryClient.setQueryData<Partial<Settings>>(QUERY_KEY.SETTINGS.CURRENT, { editorFontSize: SEEDED_EDITOR_FONT_SIZE })
    const { StatusBarContent } = await importStatusBar()
    const rendered = renderWithProviders(
        <TooltipProvider>
            <StatusBarContent projectId={PROJECT_ID} isProblemsOpen={false} onToggleProblems={() => undefined} />
        </TooltipProvider>,
        { queryClient },
    )
    await act(async () => undefined)
    return rendered
}

describe('StatusBarContent 폰트 크기', () => {
    beforeAll(() => {
        window.__TAURI_INTERNALS__ = { transformCallback: () => FAKE_EVENT_ID, invoke: fakeInvoke }
        Reflect.set(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY, { unregisterListener: () => undefined })
    })

    afterAll(() => {
        window.__TAURI_INTERNALS__ = undefined
        Reflect.deleteProperty(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY)
    })

    test('⌘= 를 더 이상 직접 등록하지 않는다 (main 창 이중 등록 방지)', async () => {
        const settingsUpdate = spyOn(commands, 'settingsUpdate')

        await renderStatusBar()
        await act(async () => {
            pressKey(FONT_SIZE_UP_KEY)
        })

        expect(settingsUpdate.mock.calls).toEqual([])
    })

    test('폰트 크기 버튼은 그대로 동작한다 (공용 훅 재사용)', async () => {
        const settingsUpdate = spyOn(commands, 'settingsUpdate')

        await renderStatusBar()
        /**
         * The editor and terminal steppers carry the same `aria-label` here — the harness's i18n has
         * no resource bundles, so `t('window.increaseFontSize', { label })` renders the bare key — and
         * the editor stepper is the first of the two (`status-bar.tsx`).
         */
        const [increaseEditorFontSize] = screen.getAllByRole('button', { name: 'window.increaseFontSize' })
        await act(async () => {
            fireEvent.click(increaseEditorFontSize)
        })

        expect(settingsUpdate.mock.calls.map(([patch]) => patch.editorFontSize)).toEqual([INCREASED_EDITOR_FONT_SIZE])
    })
})
