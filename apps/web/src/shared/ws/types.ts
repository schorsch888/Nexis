export type WebSocketEventType = 'message' | 'room_update' | 'member_update'
export type ConnectionState = 'connected' | 'connecting' | 'disconnected' | 'reconnecting'
export type MessageDeliveryStatus = 'sending' | 'sent' | 'delivered' | 'read' | 'failed'

export interface MessageStatusPayload {
  messageId: string
  status: MessageDeliveryStatus
}

export interface ReadReceiptPayload {
  messageId: string
}

export type RealtimeEventType =
  | WebSocketEventType
  | 'message_status'
  | 'read_receipt'
  | 'heartbeat'
  | 'pong'

export interface WebSocketMessage {
  type: RealtimeEventType
  payload: unknown
}

export interface WebSocketClientOptions {
  onMessage?: (data: WebSocketMessage) => void
  onStateChange?: (state: ConnectionState) => void
  maxReconnectAttempts?: number
  reconnectDelay?: number
  heartbeatInterval?: number
  heartbeatTimeout?: number
}
