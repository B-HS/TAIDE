import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

const ZEN_HINT_VISIBLE_DURATION_MS = 3000

/**
 * Mounted only while `zen` is true (see {@link ZenModeHint} below) — its own lifecycle *is* one
 * Zen-mode session, so `visible` can start `true` unconditionally and the auto-dismiss timer only
 * ever needs to turn it `false` from within its `setTimeout` callback (a genuinely async state
 * update, unlike calling `setState` synchronously inside the effect body). Re-entering Zen mode
 * later simply remounts a fresh instance — that's what resets the hint to visible again, with no
 * explicit reset logic needed.
 */
const ZenModeHintBanner: FC = () => {
    const [visible, setVisible] = useState(true)

    const { t } = useTranslation()

    useEffect(() => {
        const timeout = setTimeout(() => setVisible(false), ZEN_HINT_VISIBLE_DURATION_MS)
        return () => clearTimeout(timeout)
    }, [])

    if (!visible) return null

    return (
        <div className='bg-panel-background border-app-border text-app-foreground pointer-events-none absolute top-3 left-1/2 z-20 -translate-x-1/2 rounded-md border px-3 py-1.5 text-xs shadow-lg'>
            <span className='font-medium'>{t('zen.hint')}</span>
            <span className='text-app-sidebar-icon-default ml-2'>{t('zen.hintExit')}</span>
        </div>
    )
}

type ZenModeHintProps = {
    zen: boolean
}

export const ZenModeHint: FC<ZenModeHintProps> = ({ zen }) => (zen ? <ZenModeHintBanner /> : null)
