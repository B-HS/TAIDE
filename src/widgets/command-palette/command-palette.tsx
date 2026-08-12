import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { File, Terminal } from 'lucide-react'
import { toast } from 'sonner'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import {
    formatCategorizedLabel,
    getRegisteredCommand,
    isCommandRunnable,
    listRegisteredCommands,
    parsePaletteQuery,
} from '@shared/lib/command-registry'
import { useKeydownCapture } from '@shared/hooks/use-keydown-capture'
import { buildKeybindingRows, findKeybindingRowById, findRunnableCommandBinding } from '@shared/lib/keybinding-catalog'
import { APP_KEYMAP, applyKeymapOverrides, formatKeymapShortcut, parseKeymapOverrides } from '@shared/lib/keymap'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandShortcut } from '@shared/ui/command'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { treeRowsQueryOptions } from '@entities/tree/tree.query'
import { useOpenTab, useReopenClosedTab } from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'

const FILE_RESULT_LIMIT = 200

export const CommandPalette = () => {
    const [open, setOpen] = useState(false)
    const [query, setQuery] = useState('')

    const { t } = useTranslation()
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mode, searchTerm } = parsePaletteQuery(query)
    const { data: treePage } = useQuery({ ...treeRowsQueryOptions(activeProjectId), enabled: open && mode === 'files' && !!activeProjectId })
    const { mutate: openTab } = useOpenTab(activeProjectId)
    const { mutate: reopenClosedTabMutate } = useReopenClosedTab(activeProjectId)

    const keymapOverrides = parseKeymapOverrides(settings?.keymapOverrides ?? null)
    const keymapEntries = applyKeymapOverrides(APP_KEYMAP, keymapOverrides)

    const handleOpenChange = (next: boolean) => {
        setOpen(next)
        if (!next) setQuery('')
    }

    const openTerminalTab = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
        )
    }

    const openSettingsTab = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
        )
    }

    const reopenClosedTab = () => {
        if (!activeProjectId) return
        reopenClosedTabMutate(activeProjectId, { onError: (error) => toast.error(error.message) })
    }

    useGlobalKeymap(
        {
            'quick-open': () => {
                setQuery('')
                setOpen(true)
            },
            'command-palette': () => {
                setQuery('>')
                setOpen(true)
            },
            'new-terminal': openTerminalTab,
            'reopen-closed-tab': reopenClosedTab,
        },
        keymapEntries,
    )

    const commandContext: CommandContext = {
        activeProjectId,
        openSettingsTab,
        openTerminalTab,
        reopenClosedTab,
        switchToFileSearchMode: () => setQuery(''),
    }

    const commandKeybindingRows = buildKeybindingRows(listRegisteredCommands(), keymapOverrides)

    useKeydownCapture((event) => {
        const row = findRunnableCommandBinding(commandKeybindingRows, event)
        if (!row?.commandId) return
        const command = getRegisteredCommand(row.commandId)
        if (!command || !isCommandRunnable(command, commandContext)) return
        event.preventDefault()
        void command.run(commandContext)
    })

    const fileRows = (treePage?.rows ?? []).filter((row) => row.kind === 'file')
    const filteredFiles = fuzzyFilter(searchTerm, fileRows, (row) => row.path).slice(0, FILE_RESULT_LIMIT)
    const filteredCommands = fuzzyFilter(searchTerm, listRegisteredCommands(), (command) =>
        formatCategorizedLabel(t, command.categoryKey, command.titleKey),
    )

    const runCommand = (command: AppCommand) => {
        if (!isCommandRunnable(command, commandContext)) return
        void command.run(commandContext)
        if (command.id !== 'file.quickOpen') handleOpenChange(false)
    }

    const openFile = (path: string) => {
        if (!activeProjectId) return
        const name = path.slice(path.lastIndexOf('/') + 1)
        openTab(
            { projectId: activeProjectId, kind: { kind: 'file', path }, title: name, target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
        )
        handleOpenChange(false)
    }

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogHeader className='sr-only'>
                <DialogTitle>{t('palette.title')}</DialogTitle>
            </DialogHeader>
            <DialogContent className='overflow-hidden p-0' showCloseButton={false}>
                <Command shouldFilter={false} className='bg-panel-background text-app-foreground'>
                    <CommandInput
                        value={query}
                        onValueChange={setQuery}
                        placeholder={mode === 'files' ? t('palette.filePlaceholder') : t('palette.commandPlaceholder')}
                    />
                    <CommandList>
                        <CommandEmpty>{t('palette.noResults')}</CommandEmpty>
                        {mode === 'commands' && (
                            <CommandGroup heading={t('palette.commands')}>
                                {filteredCommands.map(({ item }) => {
                                    const keybindingRow = findKeybindingRowById(commandKeybindingRows, item.keymapId ?? item.id)
                                    const runnable = isCommandRunnable(item, commandContext)
                                    return (
                                        <CommandItem key={item.id} disabled={!runnable} onSelect={() => runCommand(item)}>
                                            <Terminal className='size-4' />
                                            <span>{formatCategorizedLabel(t, item.categoryKey, item.titleKey)}</span>
                                            {keybindingRow?.key && <CommandShortcut>{formatKeymapShortcut(keybindingRow)}</CommandShortcut>}
                                        </CommandItem>
                                    )
                                })}
                            </CommandGroup>
                        )}
                        {mode === 'files' && (
                            <CommandGroup heading={t('palette.files')}>
                                {filteredFiles.map(({ item }) => (
                                    <CommandItem key={item.path} onSelect={() => openFile(item.path)}>
                                        <File className='size-4' />
                                        <span className='truncate'>{item.path}</span>
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        )}
                    </CommandList>
                </Command>
            </DialogContent>
        </Dialog>
    )
}
