import { createFireAndForgetBridge } from '@shared/lib/fire-and-forget-bridge'

export type CreateTagRequest = { target: string }

const openCreateTagDialogBridge = createFireAndForgetBridge<CreateTagRequest>({ emptyPolicy: 'queue-latest' })

export const requestOpenCreateTagDialog = openCreateTagDialogBridge.publish
export const subscribeOpenCreateTagDialog = openCreateTagDialogBridge.subscribe
