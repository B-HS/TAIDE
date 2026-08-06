import type { FC } from 'react'
import { useState } from 'react'
import { Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { FontFamily } from '@shared/api/bindings'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@shared/ui/command'
import { Switch } from '@shared/ui/switch'

export const SYSTEM_FONT_FAMILY_VALUE = 'system-default'

type FontPickerProps = {
    label: string
    fonts: FontFamily[]
    value: string | null
    onSelect: (fontFamily: string | null) => void
}

export const FontPicker: FC<FontPickerProps> = ({ label, fonts, value, onSelect }) => {
    const [monospaceOnly, setMonospaceOnly] = useState(true)

    const { t } = useTranslation()

    const visibleFonts = monospaceOnly ? fonts.filter((font) => font.monospaced) : fonts

    return (
        <div className='flex flex-col gap-2'>
            <div className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{label}</span>
                <label className='flex items-center gap-2'>
                    <span className='text-app-sidebar-icon-default'>{t('settings.fontFamilyMonospaceOnly')}</span>
                    <Switch checked={monospaceOnly} onCheckedChange={setMonospaceOnly} />
                </label>
            </div>
            <Command className='border-app-border bg-panel-input-background rounded-md border'>
                <CommandInput placeholder={t('settings.fontFamilySearchPlaceholder')} className='text-xs' />
                <CommandList className='max-h-48'>
                    <CommandEmpty className='text-app-sidebar-icon-default py-4 text-center text-xs'>
                        {t('settings.fontFamilyNoResults')}
                    </CommandEmpty>
                    <CommandGroup>
                        <CommandItem value={SYSTEM_FONT_FAMILY_VALUE} onSelect={() => onSelect(null)} className='text-xs'>
                            <span className='flex-1'>{t('settings.fontFamilySystemDefault')}</span>
                            {value === null && <Check className='text-app-accent size-4 shrink-0' />}
                        </CommandItem>
                        {visibleFonts.map((font) => (
                            <CommandItem key={font.name} value={font.name} onSelect={() => onSelect(font.name)} className='text-xs'>
                                <span className='flex-1 truncate' style={{ fontFamily: `"${font.name}"` }}>
                                    {font.name}
                                </span>
                                {value === font.name && <Check className='text-app-accent size-4 shrink-0' />}
                            </CommandItem>
                        ))}
                    </CommandGroup>
                </CommandList>
            </Command>
        </div>
    )
}
