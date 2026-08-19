import type { FC, KeyboardEvent } from 'react'
import { useState } from 'react'
import { X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@shared/ui/button'
import { IconButton } from '@shared/ui/icon-button'

type RemoteAllowedHostsRowProps = {
    hosts: string[]
    saving: boolean
    onChange: (hosts: string[]) => void
}

const normalizeHost = (value: string) => value.trim().toLowerCase()

/**
 * RFC 1035 §3.1 label/name length limits, mirroring the backend's `DNS_LABEL_MAX_LEN` /
 * `DNS_HOSTNAME_MAX_LEN` (`src-tauri/src/domain/settings/service.rs`).
 */
const DNS_LABEL_MAX_LEN = 63
const DNS_HOSTNAME_MAX_LEN = 253

/**
 * Matches the backend's `is_valid_dns_label` (`src-tauri/src/domain/settings/service.rs`) —
 * rejected characters (scheme separators, `@`, `/`, whitespace, `:` for a port) mirror exactly
 * so a host accepted here is never silently dropped by the backend's own sanitize pass.
 */
const isValidAllowedHostLabel = (label: string) =>
    label.length > 0 && label.length <= DNS_LABEL_MAX_LEN && !label.startsWith('-') && !label.endsWith('-') && /^[a-z0-9-]+$/.test(label)

/**
 * Matches the backend's `is_valid_allowed_host` (`src-tauri/src/domain/settings/service.rs`) —
 * a bare hostname, or a `*.` wildcard whose remainder still has two or more labels (RFC 6125
 * single-label match). The wildcard never covers its own base domain — that RFC 6125 semantic
 * is enforced by `remote::service::host_matches_allowed_entry` at match time, not here; this
 * function only mirrors the backend's *shape* validation so an entry accepted here is never
 * silently dropped by the backend's own sanitize pass. The `'*.'` literal below mirrors the
 * single-owner `remote::service::ALLOWED_HOST_WILDCARD_PREFIX` Rust constant — TS has no way to
 * import a Rust `const`, so this stays a hand-kept literal instead.
 */
export const isValidAllowedHost = (value: string) => {
    const wildcardRemainder = value.startsWith('*.') ? value.slice(2) : null
    if (wildcardRemainder !== null)
        return (
            wildcardRemainder.includes('.') &&
            wildcardRemainder.length <= DNS_HOSTNAME_MAX_LEN &&
            wildcardRemainder.split('.').every(isValidAllowedHostLabel)
        )
    return value.length > 0 && value.length <= DNS_HOSTNAME_MAX_LEN && value.split('.').every(isValidAllowedHostLabel)
}

export const RemoteAllowedHostsRow: FC<RemoteAllowedHostsRowProps> = ({ hosts, saving, onChange }) => {
    const { t } = useTranslation()
    const [hostInput, setHostInput] = useState('')

    const trimmed = normalizeHost(hostInput)
    const isDuplicate = trimmed.length > 0 && hosts.some((host) => host.toLowerCase() === trimmed)
    const isInvalid = trimmed.length > 0 && !isValidAllowedHost(trimmed)
    const canAdd = trimmed.length > 0 && !isDuplicate && !isInvalid && !saving

    const handleAdd = () => {
        if (!canAdd) return
        setHostInput('')
        onChange([...hosts, trimmed])
    }
    const handleRemove = (host: string) => onChange(hosts.filter((existing) => existing !== host))
    const handleInputKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key !== 'Enter') return
        event.preventDefault()
        handleAdd()
    }

    return (
        <div className='flex flex-col gap-1.5 text-xs'>
            <div className='flex flex-col gap-0.5'>
                <span className='text-app-foreground'>{t('remote.allowedHostsLabel')}</span>
                <span className='text-app-sidebar-icon-default'>{t('remote.allowedHostsDescription')}</span>
            </div>
            {hosts.length > 0 && (
                <ul className='flex flex-col gap-1'>
                    {hosts.map((host) => (
                        <li key={host} className='border-app-border flex min-w-0 items-center gap-2 rounded-sm border px-2 py-1'>
                            <span className='text-app-foreground min-w-0 flex-1 truncate font-mono'>{host}</span>
                            <IconButton
                                label={t('remote.allowedHostsRemove', { host })}
                                icon={<X className='size-3.5' />}
                                disabled={saving}
                                onClick={() => handleRemove(host)}
                                className='text-app-sidebar-icon-default hover:text-app-foreground flex size-5 shrink-0 items-center justify-center rounded-sm'
                            />
                        </li>
                    ))}
                </ul>
            )}
            {isInvalid && <span className='text-status-warning'>{t('remote.allowedHostsInvalid')}</span>}
            <div className='flex items-center gap-1.5'>
                <input
                    type='text'
                    value={hostInput}
                    placeholder={t('remote.allowedHostsPlaceholder')}
                    aria-label={t('remote.allowedHostsLabel')}
                    onChange={(event) => setHostInput(event.target.value)}
                    onKeyDown={handleInputKeyDown}
                    className='bg-panel-input-background border-panel-input-border text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1'
                />
                <Button type='button' variant='outline' size='xs' disabled={!canAdd} onClick={handleAdd}>
                    {t('remote.allowedHostsAdd')}
                </Button>
            </div>
        </div>
    )
}
