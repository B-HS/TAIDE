import type { FC } from 'react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import {
    remoteStatusQueryOptions,
    useClearRemotePassword,
    useIssueRemoteLink,
    useRevokeRemoteSessions,
    useSetRemotePassword,
} from '@entities/remote/remote.query'
import { RemoteSection } from '@features/settings/remote-section'
import { SettingsSection } from '@features/settings/settings-section'
import type { Settings } from '@shared/api/bindings'

type SettingsRemoteSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
    isUpdatingSettings: boolean
}

export const SettingsRemoteSection: FC<SettingsRemoteSectionProps> = ({ id, settings, updateSettings, isUpdatingSettings }) => {
    const [issuedRemoteUrl, setIssuedRemoteUrl] = useState<string | null>(null)

    const { data: remoteStatus } = useQuery(remoteStatusQueryOptions())
    const { mutate: issueRemoteLink, isPending: isIssuingRemoteLink } = useIssueRemoteLink()
    const { mutate: revokeRemoteSessions, isPending: isRevokingRemoteSessions } = useRevokeRemoteSessions()
    const { mutate: setRemotePassword, isPending: isSettingRemotePassword } = useSetRemotePassword()
    const { mutate: clearRemotePassword, isPending: isClearingRemotePassword } = useClearRemotePassword()

    const { t } = useTranslation()

    const handleToggleRemote = (enabled: boolean) => {
        setIssuedRemoteUrl(null)
        updateSettings({ ...emptySettingsPatch(), remoteAccessEnabled: enabled })
    }
    const handleIssueRemoteLink = () =>
        issueRemoteLink(undefined, {
            onSuccess: (info) => {
                setIssuedRemoteUrl(info.url)
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
    const handleChangeRemoteAllowedHosts = (remoteAllowedHosts: string[]) =>
        updateSettings({ ...emptySettingsPatch(), remoteAllowedHosts }, { onError: () => toast.error(t('remote.allowedHostsSaveFailed')) })

    return (
        <SettingsSection id={id} title={t('remote.title')} description={t('remote.description')}>
            <RemoteSection
                status={remoteStatus}
                enabled={settings.remoteAccessEnabled ?? false}
                issuedUrl={issuedRemoteUrl}
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
