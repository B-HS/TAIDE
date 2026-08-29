import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { remoteStatusQueryOptions, useClearRemotePassword, useRevokeRemoteSessions, useSetRemotePassword } from '@entities/remote/remote.query'
import type { useIssueRemoteLink } from '@entities/remote/remote.query'
import { RemoteSection } from '@features/settings/remote-section'
import { SettingsSection } from '@features/settings/settings-section'
import type { Settings } from '@shared/api/bindings'

type SettingsRemoteSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
    isUpdatingSettings: boolean
    issuedUrl: string | null
    onIssuedUrlChange: (url: string | null) => void
    issueRemoteLink: ReturnType<typeof useIssueRemoteLink>['mutate']
    isIssuingRemoteLink: boolean
}

export const SettingsRemoteSection: FC<SettingsRemoteSectionProps> = ({
    id,
    settings,
    updateSettings,
    isUpdatingSettings,
    issuedUrl,
    onIssuedUrlChange,
    issueRemoteLink,
    isIssuingRemoteLink,
}) => {
    const { data: remoteStatus } = useQuery(remoteStatusQueryOptions())
    const { mutate: revokeRemoteSessions, isPending: isRevokingRemoteSessions } = useRevokeRemoteSessions()
    const { mutate: setRemotePassword, isPending: isSettingRemotePassword } = useSetRemotePassword()
    const { mutate: clearRemotePassword, isPending: isClearingRemotePassword } = useClearRemotePassword()

    const { t } = useTranslation()

    const handleToggleRemote = (enabled: boolean) => {
        onIssuedUrlChange(null)
        updateSettings({ ...emptySettingsPatch(), remoteAccessEnabled: enabled })
    }
    const handleIssueRemoteLink = () =>
        issueRemoteLink(undefined, {
            onSuccess: (info) => {
                onIssuedUrlChange(info.url)
                void navigator.clipboard.writeText(info.url).then(
                    () => toast.success(t('remote.linkCopied')),
                    () => undefined,
                )
            },
            onError: () => toast.error(t('remote.startFailed')),
        })
    const handleRevokeRemoteSessions = () => revokeRemoteSessions(undefined, { onSuccess: () => toast.success(t('remote.sessionsRevoked')) })
    const handleSaveRemotePassword = (password: string) => setRemotePassword(password)
    const handleClearRemotePassword = () => clearRemotePassword()
    const handleTogglePasswordOnlyLogin = (checked: boolean) => updateSettings({ ...emptySettingsPatch(), remotePasswordOnlyLogin: checked })
    /**
     * No per-call `onError` here: `useUpdateSettings` now reports every settings-write failure with
     * the backend's own reason (audit §4-B B15), and TanStack v5 runs both the hook's and the call's
     * `onError`, which would show two toasts for the same failure. `remote.allowedHostsSaveFailed`
     * stays in the catalog as the coarser wording that generic path replaced.
     */
    const handleChangeRemoteAllowedHosts = (remoteAllowedHosts: string[]) => updateSettings({ ...emptySettingsPatch(), remoteAllowedHosts })

    return (
        <SettingsSection id={id} title={t('remote.title')} description={t('remote.description')}>
            <RemoteSection
                status={remoteStatus}
                enabled={settings.remoteAccessEnabled ?? false}
                issuedUrl={issuedUrl}
                issuing={isIssuingRemoteLink}
                revoking={isRevokingRemoteSessions}
                passwordSaving={isSettingRemotePassword || isClearingRemotePassword}
                passwordOnlyLogin={settings.remotePasswordOnlyLogin ?? false}
                allowedHosts={settings.remoteAllowedHosts ?? []}
                allowedHostsSaving={isUpdatingSettings}
                onToggle={handleToggleRemote}
                onIssueLink={handleIssueRemoteLink}
                onRevokeSessions={handleRevokeRemoteSessions}
                onSavePassword={handleSaveRemotePassword}
                onClearPassword={handleClearRemotePassword}
                onTogglePasswordOnlyLogin={handleTogglePasswordOnlyLogin}
                onChangeAllowedHosts={handleChangeRemoteAllowedHosts}
            />
        </SettingsSection>
    )
}
