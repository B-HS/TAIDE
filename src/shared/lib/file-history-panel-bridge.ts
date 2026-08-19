import { createFireAndForgetBridge } from '@shared/lib/fire-and-forget-bridge'

const openFileHistoryBridge = createFireAndForgetBridge<string>()

export const requestOpenFileHistory = openFileHistoryBridge.publish
export const subscribeOpenFileHistory = openFileHistoryBridge.subscribe
