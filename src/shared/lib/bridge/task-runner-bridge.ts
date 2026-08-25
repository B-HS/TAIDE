import { createFireAndForgetBridge } from '@shared/lib/bridge/fire-and-forget-bridge'

type Listener = () => void

const openTaskRunnerBridge = createFireAndForgetBridge<undefined>()

export const requestOpenTaskRunner = () => openTaskRunnerBridge.publish(undefined)
export const subscribeOpenTaskRunner = (listener: Listener) => openTaskRunnerBridge.subscribe(() => listener())
