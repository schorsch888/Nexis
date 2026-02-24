import type { ConnectionState } from '../../features/messages/messagesStore'

export type WebSocketEventType = 'message' | 'room_update' | 'member_update'

export interface WebSocketMessage {
  type: WebSocketEventType
  payload: unknown
}

export interface WebSocketClientOptions {
  onMessage?: (data: WebSocketMessage) => void
  onStateChange?: (state: ConnectionState) => void
  maxReconnectAttempts?: number
  reconnectDelay?: number
}
