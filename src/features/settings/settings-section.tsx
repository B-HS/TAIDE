import type { FC, PropsWithChildren } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@shared/ui/card'

type SettingsSectionProps = PropsWithChildren<{
    id: string
    title: string
    description?: string
}>

export const SettingsSection: FC<SettingsSectionProps> = ({ id, title, description, children }) => (
    <Card id={id} className='scroll-mt-8'>
        <CardHeader>
            <CardTitle className='text-sm'>{title}</CardTitle>
            {description && <CardDescription>{description}</CardDescription>}
        </CardHeader>
        <CardContent className='flex flex-col gap-3'>{children}</CardContent>
    </Card>
)
