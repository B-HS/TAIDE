import type { FC } from 'react'
import { Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { SnippetEntryDraft } from '@shared/lib/snippet-draft'
import { IconButton } from '@shared/ui/icon-button'

type SnippetEntryEditorProps = {
    draft: SnippetEntryDraft
    showScope: boolean
    onChange: (patch: Partial<SnippetEntryDraft>) => void
    onDelete: () => void
}

const FIELD_INPUT_CLASS_NAME =
    'bg-panel-input-background border-panel-input-border text-app-foreground w-full rounded-sm border px-2 py-1 text-xs outline-none'

const BODY_TEXTAREA_ROWS = 4

export const SnippetEntryEditor: FC<SnippetEntryEditorProps> = ({ draft, showScope, onChange, onDelete }) => {
    const { t } = useTranslation()

    return (
        <div className='border-app-border flex flex-col gap-2 rounded-md border p-3'>
            <div className='flex items-start justify-between gap-2'>
                <label className='flex min-w-0 flex-1 flex-col gap-1'>
                    <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.nameLabel')}</span>
                    <input
                        value={draft.name}
                        onChange={(event) => onChange({ name: event.target.value })}
                        placeholder={t('snippetEditor.namePlaceholder')}
                        className={FIELD_INPUT_CLASS_NAME}
                    />
                </label>
                <IconButton
                    onClick={onDelete}
                    label={t('snippetEditor.deleteSnippetButton')}
                    icon={<Trash2 className='size-3.5' />}
                    side='bottom'
                    className='text-app-sidebar-icon-default hover:text-status-error mt-5 flex size-6 shrink-0 items-center justify-center rounded-sm'
                />
            </div>

            <label className='flex flex-col gap-1'>
                <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.prefixLabel')}</span>
                <input
                    value={draft.prefix}
                    onChange={(event) => onChange({ prefix: event.target.value })}
                    placeholder={t('snippetEditor.prefixPlaceholder')}
                    className={FIELD_INPUT_CLASS_NAME}
                />
                <span className='text-app-sidebar-icon-default text-[11px]'>{t('snippetEditor.prefixHint')}</span>
            </label>

            <label className='flex flex-col gap-1'>
                <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.bodyLabel')}</span>
                <textarea
                    value={draft.body}
                    onChange={(event) => onChange({ body: event.target.value })}
                    placeholder={t('snippetEditor.bodyPlaceholder')}
                    rows={BODY_TEXTAREA_ROWS}
                    className={`${FIELD_INPUT_CLASS_NAME} resize-none font-mono`}
                />
                <span className='text-app-sidebar-icon-default text-[11px]'>{t('snippetEditor.bodyHint')}</span>
            </label>

            <label className='flex flex-col gap-1'>
                <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.descriptionLabel')}</span>
                <input
                    value={draft.description}
                    onChange={(event) => onChange({ description: event.target.value })}
                    placeholder={t('snippetEditor.descriptionPlaceholder')}
                    className={FIELD_INPUT_CLASS_NAME}
                />
            </label>

            {showScope && (
                <label className='flex flex-col gap-1'>
                    <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.scopeLabel')}</span>
                    <input
                        value={draft.scope}
                        onChange={(event) => onChange({ scope: event.target.value })}
                        placeholder={t('snippetEditor.scopePlaceholder')}
                        className={FIELD_INPUT_CLASS_NAME}
                    />
                    <span className='text-app-sidebar-icon-default text-[11px]'>{t('snippetEditor.scopeHint')}</span>
                </label>
            )}
        </div>
    )
}
