import { monaco } from '@shared/lib/monaco/setup'
import { computeBlameZoneAfterLineNumber, computeBlameZoneFontSize, computeBlameZoneHeightPx } from '@shared/lib/monaco/blame-zone-layout'

const BLAME_ZONE_AFTER_COLUMN = 1_000_000
const BLAME_ZONE_CLASS_NAME = 'taide-blame-zone'

type BlameZoneController = {
    show: (line: number, text: string) => void
    hide: () => void
    dispose: () => void
}

export const createBlameZoneController = (editor: monaco.editor.IStandaloneCodeEditor): BlameZoneController => {
    let zoneId: string | null = null
    let zone: monaco.editor.IViewZone | null = null
    let domNode: HTMLDivElement | null = null
    let zoneHeightPx = 0
    let appliedScrollDeltaPx = 0

    const applyDomNodeStyle = () => {
        if (!domNode) return
        const fontSize = computeBlameZoneFontSize(editor.getOption(monaco.editor.EditorOption.fontSize))
        domNode.style.fontSize = `${fontSize}px`
        domNode.style.lineHeight = `${zoneHeightPx}px`
    }

    const resetZoneState = () => {
        zoneId = null
        zone = null
        domNode = null
        zoneHeightPx = 0
        appliedScrollDeltaPx = 0
    }

    /**
     * Mirrors monaco's own `StableEditorScrollState` idiom (never touch scrollTop while the
     * editor is pinned to the very top of the file). The zone always sits just above the cursor
     * line, so at scrollTop 0 there is no content above it left to reveal — compensating would
     * scroll the zone itself out of the viewport instead of keeping the cursor line stable.
     */
    const applyScrollDelta = (deltaPx: number) => {
        if (deltaPx === 0 || editor.getScrollTop() === 0) return
        editor.setScrollTop(editor.getScrollTop() + deltaPx)
        appliedScrollDeltaPx += deltaPx
    }

    const show = (line: number, text: string) => {
        const editorFontSize = editor.getOption(monaco.editor.EditorOption.fontSize)
        const editorLineHeight = editor.getOption(monaco.editor.EditorOption.lineHeight)
        const heightPx = computeBlameZoneHeightPx(editorFontSize, editorLineHeight)
        const afterLineNumber = computeBlameZoneAfterLineNumber(line)
        const previousHeightPx = zoneHeightPx

        if (zoneId === null || zone === null || domNode === null) {
            const nextDomNode = document.createElement('div')
            nextDomNode.className = BLAME_ZONE_CLASS_NAME
            nextDomNode.textContent = text
            const nextZone: monaco.editor.IViewZone = {
                afterLineNumber,
                afterColumn: BLAME_ZONE_AFTER_COLUMN,
                heightInPx: heightPx,
                domNode: nextDomNode,
                suppressMouseDown: true,
            }
            editor.changeViewZones((accessor) => {
                zoneId = accessor.addZone(nextZone)
            })
            zone = nextZone
            domNode = nextDomNode
            zoneHeightPx = heightPx
            applyDomNodeStyle()
            applyScrollDelta(heightPx)
            return
        }

        domNode.textContent = text
        if (zone.afterLineNumber === afterLineNumber && zoneHeightPx === heightPx) return

        zone.afterLineNumber = afterLineNumber
        zone.heightInPx = heightPx
        const currentZoneId = zoneId
        editor.changeViewZones((accessor) => accessor.layoutZone(currentZoneId))
        zoneHeightPx = heightPx
        applyDomNodeStyle()
        applyScrollDelta(heightPx - previousHeightPx)
    }

    const hide = () => {
        if (zoneId === null) return
        const currentZoneId = zoneId
        const appliedDeltaPx = appliedScrollDeltaPx
        editor.changeViewZones((accessor) => accessor.removeZone(currentZoneId))
        resetZoneState()
        if (appliedDeltaPx !== 0) editor.setScrollTop(editor.getScrollTop() - appliedDeltaPx)
    }

    const recomputeForConfigChange = () => {
        if (zoneId === null || zone === null) return
        const editorFontSize = editor.getOption(monaco.editor.EditorOption.fontSize)
        const editorLineHeight = editor.getOption(monaco.editor.EditorOption.lineHeight)
        const nextHeightPx = computeBlameZoneHeightPx(editorFontSize, editorLineHeight)
        if (nextHeightPx === zoneHeightPx) {
            applyDomNodeStyle()
            return
        }

        const previousHeightPx = zoneHeightPx
        zone.heightInPx = nextHeightPx
        const currentZoneId = zoneId
        editor.changeViewZones((accessor) => accessor.layoutZone(currentZoneId))
        zoneHeightPx = nextHeightPx
        applyDomNodeStyle()
        applyScrollDelta(nextHeightPx - previousHeightPx)
    }

    const modelSubscription = editor.onDidChangeModel(resetZoneState)
    const configSubscription = editor.onDidChangeConfiguration((event) => {
        if (event.hasChanged(monaco.editor.EditorOption.fontSize) || event.hasChanged(monaco.editor.EditorOption.lineHeight))
            recomputeForConfigChange()
    })

    const dispose = () => {
        hide()
        modelSubscription.dispose()
        configSubscription.dispose()
    }

    return { show, hide, dispose }
}
