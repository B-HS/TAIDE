import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { Bell, ExternalLink } from 'lucide-react'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { openNotificationSystemSettings, sendNativeNotification } from '@entities/notification/notification.ipc'
import { SettingsSection } from '@features/settings/settings-section'
import { SwitchField } from '@features/settings/switch-field'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import type { NotificationSuppressionReason, Settings } from '@shared/api/bindings'
import { Button } from '@shared/ui/button'

/**
 * Names the switch that swallowed a test notification, using that switch's own label so the toast
 * points at a control the user is already looking at. There is no reason for "macOS itself has
 * notifications off for TAIDE": `tauri-plugin-notification`'s desktop backend answers every
 * permission query with a `Granted` stub and drops delivery failures, so that state is invisible
 * from inside the app — which is exactly why "Open notification settings" is offered
 * unconditionally rather than in response to a denial that never arrives
 * (`domain::notification::types::NotificationDelivery`).
 */
const SUPPRESSION_REASON_LABEL_KEY: Record<NotificationSuppressionReason, string> = {
    notificationsDisabled: 'settings.notificationsEnabled',
    categoryDisabled: 'settings.notificationsError',
    windowFocused: 'settings.notificationsOnlyWhenUnfocused',
}

/**
 * The test notification is sent as `error` — the category the {@link SUPPRESSION_REASON_LABEL_KEY}
 * `categoryDisabled` entry names — because the command takes a category and none of the six means
 * "diagnostic". `error` is the least surprising choice: it is the category a user is least likely
 * to have turned off, and its "Failures" switch sits in this same section.
 */
const TEST_NOTIFICATION_CATEGORY = 'error'

type SettingsNotificationSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
}

export const SettingsNotificationSection: FC<SettingsNotificationSectionProps> = ({ id, settings, updateSettings }) => {
    const { t } = useTranslation()

    const handleOpenSystemSettings = () => void openNotificationSystemSettings().catch((error: unknown) => toast.error(describeIpcError(error)))

    const handleSendTestNotification = async () => {
        try {
            const delivery = await sendNativeNotification({
                category: TEST_NOTIFICATION_CATEGORY,
                title: t('settings.notificationsSendTest'),
                body: t('notification.enableHint'),
            })
            if (delivery.outcome === 'suppressed') {
                toast.warning(t(SUPPRESSION_REASON_LABEL_KEY[delivery.reason]))
                return
            }
            toast.success(t('notification.enableHint'))
        } catch (error) {
            toast.error(describeIpcError(error))
        }
    }

    return (
        <SettingsSection id={id} title={t('settings.notifications')}>
            <SwitchField
                label={t('settings.notificationsEnabled')}
                description={t('settings.notificationsEnabledDescription')}
                checked={settings.notificationsEnabled ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notificationsEnabled: checked })}
            />
            <SwitchField
                label={t('settings.notificationsOnlyWhenUnfocused')}
                description={t('settings.notificationsOnlyWhenUnfocusedDescription')}
                checked={settings.notificationsOnlyWhenUnfocused ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notificationsOnlyWhenUnfocused: checked })}
            />
            <SwitchField
                label={t('settings.notificationsAgentCompleted')}
                checked={settings.notifyAgentCompleted ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notifyAgentCompleted: checked })}
            />
            <SwitchField
                label={t('settings.notificationsTaskCompleted')}
                checked={settings.notifyTaskCompleted ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notifyTaskCompleted: checked })}
            />
            <SwitchField
                label={t('settings.notificationsGitRemote')}
                checked={settings.notifyGitRemote ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notifyGitRemote: checked })}
            />
            <SwitchField
                label={t('settings.notificationsSearchReplace')}
                checked={settings.notifySearchReplace ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notifySearchReplace: checked })}
            />
            <SwitchField
                label={t('settings.notificationsLspInstall')}
                checked={settings.notifyLspInstall ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notifyLspInstall: checked })}
            />
            <SwitchField
                label={t('settings.notificationsError')}
                checked={settings.notifyError ?? true}
                onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), notifyError: checked })}
            />
            <div className='flex items-center gap-2'>
                <Button variant='outline' size='xs' onClick={handleOpenSystemSettings}>
                    <ExternalLink className='size-3.5' />
                    {t('settings.notificationsOpenSystemSettings')}
                </Button>
                <Button variant='outline' size='xs' onClick={() => void handleSendTestNotification()}>
                    <Bell className='size-3.5' />
                    {t('settings.notificationsSendTest')}
                </Button>
            </div>
            {import.meta.env.DEV && <span className='text-app-sidebar-icon-default text-xs'>{t('settings.notificationsDevHint')}</span>}
        </SettingsSection>
    )
}
