import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { SystemUsageProcess, SystemUsageProcessKind } from '@shared/api/bindings'
import { BYTES_PER_MEBIBYTE } from '@shared/constants/system-usage'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { systemUsageBreakdownQueryOptions } from '@entities/system/system.query'

type SystemUsageModalProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
}

const KIND_ORDER: SystemUsageProcessKind[] = ['app', 'terminal', 'lsp', 'agent', 'other']

const KIND_LABEL_KEY: Record<SystemUsageProcessKind, string> = {
    app: 'window.systemUsageKindApp',
    terminal: 'window.systemUsageKindTerminal',
    lsp: 'window.systemUsageKindLsp',
    agent: 'window.systemUsageKindAgent',
    other: 'window.systemUsageKindOther',
}

const groupByKind = (processes: SystemUsageProcess[]) =>
    KIND_ORDER.map((kind) => ({ kind, processes: processes.filter((process) => process.kind === kind) })).filter(
        (group) => group.processes.length > 0,
    )

export const SystemUsageModal: FC<SystemUsageModalProps> = ({ open, onOpenChange }) => {
    const { t } = useTranslation()
    const { data: processes = [] } = useQuery(systemUsageBreakdownQueryOptions(open))
    const groups = groupByKind(processes)

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className='flex h-[70vh] flex-col overflow-hidden sm:max-w-2xl'>
                <DialogHeader>
                    <DialogTitle>{t('window.systemUsageDetailTitle')}</DialogTitle>
                </DialogHeader>

                <div className='text-app-sidebar-icon-default flex shrink-0 items-center gap-3 px-3 text-[10px] tracking-wide uppercase'>
                    <span className='flex-1'>{t('window.systemUsageProcessColumn')}</span>
                    <span className='w-16 text-right'>{t('window.systemUsageCpuColumn')}</span>
                    <span className='w-20 text-right'>{t('window.systemUsageMemoryColumn')}</span>
                </div>

                <ScrollContainer className='min-h-0 flex-1'>
                    {groups.length === 0 ? (
                        <p className='text-app-sidebar-icon-default px-1 py-4 text-center text-xs'>{t('window.systemUsageEmpty')}</p>
                    ) : (
                        <div className='flex flex-col gap-3 pr-2'>
                            {groups.map((group) => (
                                <section key={group.kind} className='flex flex-col gap-1'>
                                    <h3 className='text-app-sidebar-icon-default px-1 text-[10px] font-semibold tracking-wide uppercase'>
                                        {t(KIND_LABEL_KEY[group.kind])}
                                    </h3>
                                    <ul className='flex flex-col'>
                                        {group.processes.map((process) => (
                                            <li
                                                key={process.pid}
                                                className='hover:bg-explorer-item-hover flex items-center gap-3 rounded-sm px-3 py-1 text-xs'>
                                                <span className='min-w-0 flex-1 truncate' title={process.label}>
                                                    {process.label}
                                                </span>
                                                <span className='text-app-sidebar-icon-default w-16 text-right tabular-nums'>
                                                    {Math.round(process.cpuPercent ?? 0)}%
                                                </span>
                                                <span className='text-app-sidebar-icon-default w-20 text-right tabular-nums'>
                                                    {Math.round((process.memoryBytes ?? 0) / BYTES_PER_MEBIBYTE)}MB
                                                </span>
                                            </li>
                                        ))}
                                    </ul>
                                </section>
                            ))}
                        </div>
                    )}
                </ScrollContainer>
            </DialogContent>
        </Dialog>
    )
}
