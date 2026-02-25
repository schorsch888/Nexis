import type { ConnectionState as ConnectionStatus } from '../../shared/ws/types'
import { ConnectionState } from './ConnectionState'

interface ConnectionBadgeProps {
  state: ConnectionStatus
}

export function ConnectionBadge({ state }: ConnectionBadgeProps) {
  return <ConnectionState state={state} unreadCount={0} />
}
