import { commands } from '@shared/api/bindings'
import type { NotificationCategory } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export type NativeNotificationInput = { category: NotificationCategory; title: string; body: string }

/**
 * Hands one already-translated completion message to the backend gate
 * (`domain::notification::commands::notification_notify`), which decides whether the OS ever sees
 * it. Callers that want the app's own duplicate/mirror rules applied first should go through
 * `notify.ts`'s `notifyNative`; this direct path exists for the settings screen's test button,
 * whose whole purpose is to report the gate's verdict verbatim.
 */
export const sendNativeNotification = (input: NativeNotificationInput) =>
    unwrapResult(commands.notificationNotify(input.category, input.title, input.body))

export const openNotificationSystemSettings = () => unwrapResult(commands.notificationOpenSystemSettings())
