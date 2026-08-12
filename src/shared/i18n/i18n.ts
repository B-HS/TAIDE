import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'

const TRANSLATION_NAMESPACE = 'translation'
const FALLBACK_LANGUAGE = 'en'

void i18next.use(initReactI18next).init({
    resources: {},
    lng: FALLBACK_LANGUAGE,
    fallbackLng: FALLBACK_LANGUAGE,
    keySeparator: false,
    interpolation: { escapeValue: false },
    returnNull: false,
    react: { bindI18nStore: 'added removed' },
})

export const applyLocaleMessages = (localeId: string, messages: Record<string, string>) => {
    i18next.addResourceBundle(localeId, TRANSLATION_NAMESPACE, messages, true, true)
    if (i18next.language !== localeId) void i18next.changeLanguage(localeId)
}

export { i18next }
