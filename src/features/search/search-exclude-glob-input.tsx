import type { FC } from 'react'
import { useTranslation } from 'react-i18next'

type SearchExcludeGlobInputProps = {
    value: string
    onChange: (value: string) => void
}

export const SearchExcludeGlobInput: FC<SearchExcludeGlobInputProps> = ({ value, onChange }) => {
    const { t } = useTranslation()

    return (
        <input
            value={value}
            onChange={(event) => onChange(event.target.value)}
            placeholder={t('search.excludeGlobPlaceholder')}
            className='bg-panel-input-background border-panel-input-border focus-within:border-app-focus-border min-w-0 flex-1 rounded-sm border bg-transparent px-2 py-1.5 text-xs outline-none'
        />
    )
}
