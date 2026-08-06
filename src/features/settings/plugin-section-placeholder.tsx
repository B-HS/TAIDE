import { useTranslation } from 'react-i18next'

export const PluginSectionPlaceholder = () => {
    const { t } = useTranslation()

    return (
        <div className='border-app-border text-app-sidebar-icon-default rounded-md border border-dashed px-3 py-4 text-xs'>
            <p>{t('settings.pluginsManifestHint')}</p>
            <p className='mt-1'>{t('settings.pluginsListPlaceholder')}</p>
        </div>
    )
}
