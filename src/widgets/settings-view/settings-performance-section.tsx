import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { readDesktopPerfSnapshot, resetDesktopPerfMetrics } from '@entities/app/perf.ipc'
import { SettingsSection } from '@features/settings/settings-section'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { buildPerfReport, resetPerfMetrics } from '@shared/lib/perf-mark'
import type { PerfSnapshot } from '@shared/api/bindings'
import { Button } from '@shared/ui/button'

type SettingsPerformanceSectionProps = {
    id: string
}

const ANIMATION_FRAME_PROBE_TIMEOUT_MS = 1000

export const SettingsPerformanceSection: FC<SettingsPerformanceSectionProps> = ({ id }) => {
    const [frontendSnapshot, setFrontendSnapshot] = useState<ReturnType<typeof buildPerfReport> | null>(null)
    const [nativeSnapshot, setNativeSnapshot] = useState<PerfSnapshot | null>(null)
    const [memoryCounts, setMemoryCounts] = useState<{ monacoModels: number; queryCacheEntries: number } | null>(null)
    const [animationFrameProbe, setAnimationFrameProbe] = useState<'fired' | 'timed-out' | null>(null)
    const [isReading, setIsReading] = useState(false)

    const queryClient = useQueryClient()
    const { t } = useTranslation()

    const handleRead = async () => {
        setIsReading(true)
        try {
            const frameProbe = await new Promise<'fired' | 'timed-out'>((resolve) => {
                const timeoutId = window.setTimeout(() => resolve('timed-out'), ANIMATION_FRAME_PROBE_TIMEOUT_MS)
                window.requestAnimationFrame(() => {
                    window.clearTimeout(timeoutId)
                    resolve('fired')
                })
            })
            const nextNativeSnapshot = await readDesktopPerfSnapshot()
            const { monaco } = await import('@shared/lib/monaco/setup')
            setFrontendSnapshot(buildPerfReport())
            setNativeSnapshot(nextNativeSnapshot)
            setMemoryCounts({ monacoModels: monaco.editor.getModels().length, queryCacheEntries: queryClient.getQueryCache().getAll().length })
            setAnimationFrameProbe(frameProbe)
        } catch (error) {
            toast.error(describeIpcError(error))
        } finally {
            setIsReading(false)
        }
    }

    const handleReset = async () => {
        setIsReading(true)
        try {
            await resetDesktopPerfMetrics()
            resetPerfMetrics()
            setFrontendSnapshot(null)
            setNativeSnapshot(null)
            setMemoryCounts(null)
            setAnimationFrameProbe(null)
        } catch (error) {
            toast.error(describeIpcError(error))
        } finally {
            setIsReading(false)
        }
    }

    return (
        <SettingsSection id={id} title={t('settings.performance')} description={t('settings.performanceDescription')}>
            <div className='flex gap-2'>
                <Button type='button' variant='outline' size='xs' disabled={isReading} onClick={() => void handleRead()}>
                    {t('settings.performanceRead')}
                </Button>
                <Button type='button' variant='outline' size='xs' disabled={isReading} onClick={() => void handleReset()}>
                    {t('settings.performanceReset')}
                </Button>
            </div>
            {frontendSnapshot && (
                <div className='min-w-0'>
                    <h3 className='text-xs font-medium'>{t('settings.performanceFrontend')}</h3>
                    <pre className='overflow-auto text-xs'>{JSON.stringify({ ...frontendSnapshot, animationFrameProbe }, null, 2)}</pre>
                </div>
            )}
            {nativeSnapshot && (
                <div className='min-w-0'>
                    <h3 className='text-xs font-medium'>{t('settings.performanceNative')}</h3>
                    <pre className='overflow-auto text-xs'>{JSON.stringify(nativeSnapshot, null, 2)}</pre>
                </div>
            )}
            {memoryCounts && (
                <div className='min-w-0'>
                    <h3 className='text-xs font-medium'>{t('settings.performanceMemory')}</h3>
                    <pre className='overflow-auto text-xs'>{JSON.stringify(memoryCounts, null, 2)}</pre>
                </div>
            )}
        </SettingsSection>
    )
}
