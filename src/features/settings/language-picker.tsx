import type { FC, KeyboardEvent } from 'react'
import { useState } from 'react'
import { Check, ChevronDown } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { LocaleSummary } from '@shared/api/bindings'
import { Button } from '@shared/ui/button'
import { Command, CommandGroup, CommandItem, CommandList } from '@shared/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@shared/ui/popover'

export const SYSTEM_LANGUAGE_ID = 'system'

type LanguagePickerProps = {
    locales: LocaleSummary[]
    activeLanguage: string
    systemLabel: string
    onSelect: (language: string) => void
}

export const LanguagePicker: FC<LanguagePickerProps> = ({ locales, activeLanguage, systemLabel, onSelect }) => {
    const [open, setOpen] = useState(false)

    const options = [{ id: SYSTEM_LANGUAGE_ID, name: systemLabel, builtin: true }, ...locales]
    const activeOption = options.find((option) => option.id === activeLanguage) ?? options[0]
    const handleSelect = (language: string) => {
        onSelect(language)
        setOpen(false)
    }
    const handleTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
        if (event.key !== 'ArrowDown') return
        event.preventDefault()
        setOpen(true)
    }

    const { t } = useTranslation()

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <Button
                    type='button'
                    variant='outline'
                    size='sm'
                    role='combobox'
                    aria-expanded={open}
                    aria-label={t('settings.languageSelectPlaceholder')}
                    className='w-full justify-between font-normal'
                    onKeyDown={handleTriggerKeyDown}>
                    <span className='truncate text-xs'>{activeOption.name}</span>
                    <ChevronDown className='text-app-sidebar-icon-default size-4 shrink-0' />
                </Button>
            </PopoverTrigger>
            <PopoverContent align='start' className='w-(--radix-popover-trigger-width) p-0'>
                <Command>
                    <CommandList className='max-h-48'>
                        <CommandGroup>
                            {options.map((option) => (
                                <CommandItem key={option.id} value={option.name} onSelect={() => handleSelect(option.id)} className='text-xs'>
                                    <span className='flex-1 truncate'>{option.name}</span>
                                    {option.id === activeLanguage && <Check className='text-app-accent size-4 shrink-0' />}
                                </CommandItem>
                            ))}
                        </CommandGroup>
                    </CommandList>
                </Command>
            </PopoverContent>
        </Popover>
    )
}
