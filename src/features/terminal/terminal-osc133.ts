import type { IDecoration, IMarker, Terminal } from '@xterm/xterm'

const OSC_133_IDENT = 133
const COMMAND_BLOCK_GUTTER_BAR_WIDTH_PX = 2
const HEX_COLOR_PATTERN = /^#[0-9a-fA-F]{6}$/
/**
 * Upper bound on tracked command blocks. Blocks are normally trimmed for free when their
 * `startMarker`'s line scrolls out of xterm's scrollback, but that only happens on a newline — a
 * program that repeatedly emits bare `133;A` without ever advancing the cursor line (malicious or
 * just buggy) would otherwise accumulate blocks/markers forever on one screen line. Reusing this
 * cap's eviction to also fire `pruneDisposedBlocks` keeps the two cleanup paths (scrollback,
 * explicit eviction) sharing one code path.
 */
const MAX_TRACKED_COMMAND_BLOCKS = 500

export type TerminalCommandBlock = {
    startMarker: IMarker
    /**
     * Set from the `C` (command-output-start) event. Currently write-only —
     * kept (and correctly disposed alongside the other markers) as the
     * anchor a future "jump to command output" / "select command output"
     * action would need, matching the OSC 133 spec's own A/B/C/D block
     * boundaries; today's decorations and command-jump navigation only read
     * `startMarker`.
     */
    outputStartMarker: IMarker | null
    /** Set from the `D` (command-end) event. Currently write-only, kept for the same reason as `outputStartMarker`. */
    endMarker: IMarker | null
    exitCode: number | null
}

export type Osc133Kind = 'A' | 'B' | 'C' | 'D' | 'unknown'

export const parseOsc133Data = (data: string): { kind: Osc133Kind; params: string[] } => {
    const [rawKind, ...params] = data.split(';')
    const kind: Osc133Kind = rawKind === 'A' || rawKind === 'B' || rawKind === 'C' || rawKind === 'D' ? rawKind : 'unknown'
    return { kind, params }
}

export const parseOsc133ExitCode = (params: string[]): number | null => {
    const raw = params[0]
    if (raw === undefined) return null
    const parsed = Number(raw)
    return Number.isInteger(parsed) ? parsed : null
}

export type Osc133BlockTrackerState = {
    blocks: TerminalCommandBlock[]
    currentBlockIndex: number | null
}

export const INITIAL_OSC133_BLOCK_TRACKER_STATE: Osc133BlockTrackerState = { blocks: [], currentBlockIndex: null }

export type Osc133EventTransition = {
    state: Osc133BlockTrackerState
    changedBlock: TerminalCommandBlock | null
}

/**
 * Folds one parsed OSC133 event into the block-tracker state. `registerMarker` is injected so the
 * transition stays pure and testable without a live xterm `Terminal`. Unknown kinds and stray
 * `C`/`D` events (no open block) are no-ops rather than errors, matching the spec's "unknown
 * parameters are ignored" guidance for shell-emitted sequences.
 */
export const applyOsc133Event = (
    state: Osc133BlockTrackerState,
    event: { kind: Osc133Kind; params: string[] },
    registerMarker: () => IMarker | undefined,
): Osc133EventTransition => {
    if (event.kind === 'A') {
        const marker = registerMarker()
        if (!marker) return { state, changedBlock: null }
        const block: TerminalCommandBlock = { startMarker: marker, outputStartMarker: null, endMarker: null, exitCode: null }
        return { state: { blocks: [...state.blocks, block], currentBlockIndex: state.blocks.length }, changedBlock: block }
    }

    if (event.kind === 'C') {
        if (state.currentBlockIndex === null) return { state, changedBlock: null }
        const marker = registerMarker()
        if (!marker) return { state, changedBlock: null }
        const index = state.currentBlockIndex
        const blocks = state.blocks.map((block, i) => (i === index ? { ...block, outputStartMarker: marker } : block))
        return { state: { ...state, blocks }, changedBlock: blocks[index] }
    }

    if (event.kind === 'D') {
        if (state.currentBlockIndex === null) return { state, changedBlock: null }
        const index = state.currentBlockIndex
        const marker = registerMarker()
        const exitCode = parseOsc133ExitCode(event.params)
        const blocks = state.blocks.map((block, i) => (i === index ? { ...block, endMarker: marker ?? block.endMarker, exitCode } : block))
        return { state: { blocks, currentBlockIndex: null }, changedBlock: blocks[index] }
    }

    return { state, changedBlock: null }
}

/**
 * Filters out blocks whose `startMarker` has been disposed (scrollback eviction or explicit
 * cleanup) and re-derives `currentBlockIndex` for the surviving array by identity rather than by
 * shifting the old numeric index — the currently-open block is always the *last* element before
 * pruning, so removing any earlier disposed entries would otherwise leave a stale, out-of-range
 * index that silently breaks that command's eventual `D` (exit code never applied). If the
 * currently-open block itself was the one disposed, `currentBlockIndex` correctly becomes `null`.
 */
export const pruneDisposedBlocks = (state: Osc133BlockTrackerState): Osc133BlockTrackerState => {
    const currentBlock = state.currentBlockIndex === null ? null : (state.blocks[state.currentBlockIndex] ?? null)
    const blocks = state.blocks.filter((block) => !block.startMarker.isDisposed)
    if (currentBlock === null) return { blocks, currentBlockIndex: null }
    const currentBlockIndex = blocks.indexOf(currentBlock)
    return { blocks, currentBlockIndex: currentBlockIndex === -1 ? null : currentBlockIndex }
}

export const findPreviousCommandLine = (commandLines: number[], currentLine: number): number | null => {
    const candidates = commandLines.filter((line) => line < currentLine)
    return candidates.length === 0 ? null : Math.max(...candidates)
}

export const findNextCommandLine = (commandLines: number[], currentLine: number): number | null => {
    const candidates = commandLines.filter((line) => line > currentLine)
    return candidates.length === 0 ? null : Math.min(...candidates)
}

/** Accepts `#RRGGBB` as-is and truncates `#RRGGBBAA` (alpha) to `#RRGGBB`; anything else (missing, `rgba(...)`, keyword) is rejected since `IDecorationOptions` colors only support `#RRGGBB`. */
export const normalizeDecorationHexColor = (value: string | null | undefined): string | null => {
    if (!value) return null
    const truncated = value.length > 7 && value.startsWith('#') ? value.slice(0, 7) : value
    return HEX_COLOR_PATTERN.test(truncated) ? truncated : null
}

export type CommandBlockDecorationColors = {
    success: string | null
    failure: string | null
}

export const resolveCommandBlockDecorationColor = (exitCode: number, colors: CommandBlockDecorationColors) =>
    exitCode === 0 ? colors.success : colors.failure

export type TerminalOsc133Tracker = {
    getCommandStartLines: () => number[]
    jumpToPreviousCommand: () => void
    jumpToNextCommand: () => void
    dispose: () => void
}

const disposeBlockMarkers = (block: TerminalCommandBlock) => {
    if (!block.startMarker.isDisposed) block.startMarker.dispose()
    if (block.outputStartMarker && !block.outputStartMarker.isDisposed) block.outputStartMarker.dispose()
    if (block.endMarker && !block.endMarker.isDisposed) block.endMarker.dispose()
}

/**
 * Wires the pure OSC133 reducer into a live xterm `Terminal`: registers the OSC133 handler,
 * turns completed blocks into gutter + overview-ruler decorations colored by exit status, and
 * prunes blocks whose start marker was dropped by xterm once its line scrolls out of scrollback.
 * `colorsRef` is read at each block completion so a later theme change is picked up without
 * re-creating the tracker.
 *
 * Decorations are keyed by `startMarker` rather than the `TerminalCommandBlock` object itself:
 * the reducer replaces that object on every `C`/`D` transition (immutable updates), so a block
 * reference captured at `A` time would already be stale by the time its marker disposes. The
 * marker is the one field that stays the same object across the block's whole lifecycle.
 *
 * This tracker deliberately owns no "a command finished" side channel: it lives and dies with the
 * xterm instance, which `pane-node-view.tsx` unmounts whenever the terminal's tab goes to the
 * background — exactly when a long command most needs reporting. That signal is measured on the pty
 * reader thread instead and arrives as the `terminal:command-finished` event
 * (`domain::terminal::commands::report_command_marker`).
 */
export const attachOsc133BlockTracker = (term: Terminal, colorsRef: { current: CommandBlockDecorationColors }): TerminalOsc133Tracker => {
    let state = INITIAL_OSC133_BLOCK_TRACKER_STATE
    const decorationsByStartMarker = new WeakMap<IMarker, IDecoration>()

    const pruneBlockByStartMarker = (startMarker: IMarker) => {
        decorationsByStartMarker.get(startMarker)?.dispose()
        decorationsByStartMarker.delete(startMarker)
        state = pruneDisposedBlocks(state)
    }

    /**
     * Enforces `MAX_TRACKED_COMMAND_BLOCKS` by disposing the oldest blocks' markers once the cap
     * is exceeded. Every block's `startMarker` already has `pruneBlockByStartMarker` wired to its
     * `onDispose` (below, on `A`), so calling `.dispose()` here synchronously reuses that same
     * cleanup path (decoration disposal + state/index pruning) instead of duplicating it.
     */
    const evictOldestBlocksBeyondCap = () => {
        const excess = state.blocks.length - MAX_TRACKED_COMMAND_BLOCKS
        if (excess <= 0) return
        for (const block of state.blocks.slice(0, excess)) disposeBlockMarkers(block)
    }

    const applyDecoration = (block: TerminalCommandBlock) => {
        if (block.exitCode === null) return
        const color = resolveCommandBlockDecorationColor(block.exitCode, colorsRef.current)
        if (!color) return
        const decoration = term.registerDecoration({ marker: block.startMarker, overviewRulerOptions: { color, position: 'right' } })
        if (!decoration) return
        decoration.onRender((element) => {
            element.style.boxShadow = `inset ${COMMAND_BLOCK_GUTTER_BAR_WIDTH_PX}px 0 0 0 ${color}`
            element.style.pointerEvents = 'none'
        })
        decorationsByStartMarker.set(block.startMarker, decoration)
    }

    const handleOscData = (data: string) => {
        const { kind, params } = parseOsc133Data(data)
        const { state: nextState, changedBlock } = applyOsc133Event(state, { kind, params }, () => term.registerMarker(0))
        state = nextState
        if (changedBlock && kind === 'A') {
            changedBlock.startMarker.onDispose(() => pruneBlockByStartMarker(changedBlock.startMarker))
            evictOldestBlocksBeyondCap()
        }
        if (changedBlock && kind === 'D') applyDecoration(changedBlock)
        return false
    }

    const oscHandler = term.parser.registerOscHandler(OSC_133_IDENT, handleOscData)

    const jumpTo = (resolveLine: (commandLines: number[], currentLine: number) => number | null) => {
        const commandLines = state.blocks.map((block) => block.startMarker.line)
        const targetLine = resolveLine(commandLines, term.buffer.active.viewportY)
        if (targetLine === null) return
        term.scrollToLine(targetLine)
    }

    return {
        getCommandStartLines: () => state.blocks.map((block) => block.startMarker.line),
        jumpToPreviousCommand: () => jumpTo(findPreviousCommandLine),
        jumpToNextCommand: () => jumpTo(findNextCommandLine),
        dispose: () => {
            oscHandler.dispose()
            for (const block of state.blocks) {
                decorationsByStartMarker.get(block.startMarker)?.dispose()
                disposeBlockMarkers(block)
            }
            state = INITIAL_OSC133_BLOCK_TRACKER_STATE
        },
    }
}
