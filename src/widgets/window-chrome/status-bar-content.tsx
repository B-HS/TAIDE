import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { LspSessionStatus } from '@shared/api/bindings'
import { lspSessionsQueryOptions } from '@entities/lsp/lsp.query'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { systemUsageQueryOptions } from '@entities/system/system.query'
import { toProblemSeverity } from '@features/problems/problem-severity'
import { useMonacoMarkers } from '@shared/hooks/use-monaco-markers'
import { CODE_FONT_SIZE_STEP, DEFAULT_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { StatusBar } from '@features/window/status-bar'

const clampFontSize = (value: number) => Math.min(MAX_CODE_FONT_SIZE, Math.max(MIN_CODE_FONT_SIZE, value))

const isLspRunning = (status: LspSessionStatus) => status === 'running'

const isLspCrashed = (status: LspSessionStatus) => status === 'crashed'

type StatusBarContentProps = {
    isProblemsOpen: boolean
    onToggleProblems: () => void
}

export const StatusBarContent: FC<StatusBarContentProps> = ({ isProblemsOpen, onToggleProblems }) => {
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: lspSessions = [] } = useQuery(lspSessionsQueryOptions(activeProjectId))
    const showSystemUsage = settings?.showSystemUsage ?? false
    const { data: systemUsage = null } = useQuery(systemUsageQueryOptions(showSystemUsage))
    const { mutate: updateSettings } = useUpdateSettings()
    const markers = useMonacoMarkers()

    const editorFontSize = settings?.editorFontSize ?? DEFAULT_CODE_FONT_SIZE
    const terminalFontSize = settings?.terminalFontSize ?? DEFAULT_CODE_FONT_SIZE
    const errorCount = markers.filter((marker) => toProblemSeverity(marker.severity) === 'error').length

    const lspSummary =
        lspSessions.length > 0
            ? {
                  running: lspSessions.filter((session) => isLspRunning(session.status)).length,
                  total: lspSessions.length,
                  hasCrashed: lspSessions.some((session) => isLspCrashed(session.status)),
              }
            : null

    return (
        <StatusBar
            lspSummary={lspSummary}
            errorCount={errorCount}
            isProblemsOpen={isProblemsOpen}
            onToggleProblems={onToggleProblems}
            systemUsage={showSystemUsage ? systemUsage : null}
            editorFontSize={editorFontSize}
            terminalFontSize={terminalFontSize}
            onEditorFontSizeDecrease={() =>
                updateSettings({ ...emptySettingsPatch(), editorFontSize: clampFontSize(editorFontSize - CODE_FONT_SIZE_STEP) })
            }
            onEditorFontSizeIncrease={() =>
                updateSettings({ ...emptySettingsPatch(), editorFontSize: clampFontSize(editorFontSize + CODE_FONT_SIZE_STEP) })
            }
            onEditorFontSizeReset={() => updateSettings({ ...emptySettingsPatch(), editorFontSize: DEFAULT_CODE_FONT_SIZE })}
            onTerminalFontSizeDecrease={() =>
                updateSettings({ ...emptySettingsPatch(), terminalFontSize: clampFontSize(terminalFontSize - CODE_FONT_SIZE_STEP) })
            }
            onTerminalFontSizeIncrease={() =>
                updateSettings({ ...emptySettingsPatch(), terminalFontSize: clampFontSize(terminalFontSize + CODE_FONT_SIZE_STEP) })
            }
            onTerminalFontSizeReset={() => updateSettings({ ...emptySettingsPatch(), terminalFontSize: DEFAULT_CODE_FONT_SIZE })}
        />
    )
}
