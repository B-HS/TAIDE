import type { FC } from 'react'
import { CheckCircle2, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { LspServerDetection } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'

type LspServerStatusListProps = {
    servers: LspServerDetection[]
}

export const LspServerStatusList: FC<LspServerStatusListProps> = ({ servers }) => {
    const { t } = useTranslation()

    return (
        <ul className='flex flex-col gap-1.5'>
            {servers.map((server) => (
                <li key={server.id} className='border-app-border flex min-w-0 flex-col gap-0.5 rounded-md border px-3 py-2 text-xs'>
                    <div className='flex min-w-0 items-center gap-2'>
                        {server.available ? (
                            <CheckCircle2 className='text-app-sidebar-icon-agent-running size-3.5 shrink-0' />
                        ) : (
                            <XCircle className='text-app-sidebar-badge size-3.5 shrink-0' />
                        )}
                        <span className='text-app-foreground min-w-0 truncate font-medium'>{server.name}</span>
                        <span className={cn('ml-auto shrink-0', server.available ? 'text-app-sidebar-icon-agent-running' : 'text-app-sidebar-badge')}>
                            {server.available ? t('settings.lspInstalled') : t('settings.lspNotInstalled')}
                        </span>
                    </div>
                    {server.resolvedPath && (
                        <span className='text-app-sidebar-icon-default min-w-0 pl-5.5 font-mono break-all'>{server.resolvedPath}</span>
                    )}
                    {!server.available && server.installHint && (
                        <span className='text-app-sidebar-icon-default min-w-0 pl-5.5'>{server.installHint}</span>
                    )}
                </li>
            ))}
        </ul>
    )
}
