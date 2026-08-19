import type { FC, KeyboardEvent } from 'react'
import { useState } from 'react'
import { Check, ChevronDown } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { FontFamily } from '@shared/api/bindings'
import { Button } from '@shared/ui/button'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@shared/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@shared/ui/popover'
import { Switch } from '@shared/ui/switch'

const SYSTEM_FONT_FAMILY_VALUE = 'system-default'

type FontPickerProps = {
    label: string
    fonts: FontFamily[]
    value: string | null
    onSelect: (fontFamily: string | null) => void
}

export const FontPicker: FC<FontPickerProps> = ({ label, fonts, value, onSelect }) => {
    const [open, setOpen] = useState(false)
    const [monospaceOnly, setMonospaceOnly] = useState(true)

    const visibleFonts = monospaceOnly ? fonts.filter((font) => font.monospaced) : fonts
    const handleSelect = (fontFamily: string | null) => {
        onSelect(fontFamily)
        setOpen(false)
    }
    const handleTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
        if (event.key !== 'ArrowDown') return
        event.preventDefault()
        setOpen(true)
    }

    const { t } = useTranslation()

    return (
        <div className='flex flex-col gap-2'>
            <div className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{label}</span>
                <label className='flex items-center gap-2'>
                    <span className='text-app-sidebar-icon-default'>{t('settings.fontFamilyMonospaceOnly')}</span>
                    <Switch checked={monospaceOnly} onCheckedChange={setMonospaceOnly} />
                </label>
            </div>
            <Popover open={open} onOpenChange={setOpen}>
                <PopoverTrigger asChild>
                    <Button
                        type='button'
                        variant='outline'
                        size='sm'
                        role='combobox'
                        aria-expanded={open}
                        aria-label={t('settings.fontFamilySelectPlaceholder')}
                        className='w-full justify-between font-normal'
                        onKeyDown={handleTriggerKeyDown}>
                        <span className='truncate text-xs' style={value ? { fontFamily: `"${value}"` } : undefined}>
                            {value ?? t('settings.fontFamilySystemDefault')}
                        </span>
                        <ChevronDown className='text-app-sidebar-icon-default size-4 shrink-0' />
                    </Button>
                </PopoverTrigger>
                <PopoverContent align='start' className='w-(--radix-popover-trigger-width) p-0'>
                    <Command>
                        <CommandInput placeholder={t('settings.fontFamilySearchPlaceholder')} className='text-xs' />
                        <CommandList className='max-h-48'>
                            <CommandEmpty className='text-app-sidebar-icon-default py-4 text-center text-xs'>
                                {t('settings.fontFamilyNoResults')}
                            </CommandEmpty>
                            <CommandGroup>
                                <CommandItem value={SYSTEM_FONT_FAMILY_VALUE} onSelect={() => handleSelect(null)} className='text-xs'>
                                    <span className='flex-1'>{t('settings.fontFamilySystemDefault')}</span>
                                    {value === null && <Check className='text-app-accent size-4 shrink-0' />}
                                </CommandItem>
                                {visibleFonts.map((font) => (
                                    <CommandItem key={font.name} value={font.name} onSelect={() => handleSelect(font.name)} className='text-xs'>
                                        <span className='flex-1 truncate' style={{ fontFamily: `"${font.name}"` }}>
                                            {font.name}
                                        </span>
                                        {value === font.name && <Check className='text-app-accent size-4 shrink-0' />}
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        </CommandList>
                    </Command>
                </PopoverContent>
            </Popover>
        </div>
    )
}
