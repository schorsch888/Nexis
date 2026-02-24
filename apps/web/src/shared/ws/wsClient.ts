import { useAuthStore } from '../../features/auth/authStore'
import type { ConnectionState } from '../../features/messages/messagesStore'
import type { WebSocketClientOptions, WebSocketMessage } from './types'

const WS_URL = import.meta.env.VITE_WS_URL || `ws://${window.location.host}/ws`

export class WebSocketClient {
  private ws: WebSocket | null = null
  private reconnectAttempts = 0
  private readonly maxReconnectAttempts: number
  private readonly reconnectDelay: number
  private readonly onMessage?: (data: WebSocketMessage) => void
  private readonly onStateChange?: (state: ConnectionState) => void
  private intentionalClose = false

  constructor(options: WebSocketClientOptions = {}) {
    this.maxReconnectAttempts = options.maxReconnectAttempts ?? 5
    this.reconnectDelay = options.reconnectDelay ?? 1000
    this.onMessage = options.onMessage
    this.onStateChange = options.onStateChange
  }

  connect(roomId?: string): void {
    this.intentionalClose = false
    this.onStateChange?.('connecting')

    const { token } = useAuthStore.getState()
    let url = WS_URL
    if (token) {
      url += `?token=${encodeURIComponent(token)}`
    }
    if (roomId) {
      url += `${token ? '&' : '?'}roomId=${encodeURIComponent(roomId)}`
    }

    try {
      this.ws = new WebSocket(url)
      this.setupListeners()
    } catch {
      this.onStateChange?.('disconnected')
      this.scheduleReconnect()
    }
  }

  private setupListeners(): void {
    if (!this.ws) return

    this.ws.onopen = () => {
      this.reconnectAttempts = 0
      this.onStateChange?.('connected')
    }

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as WebSocketMessage
        this.onMessage?.(data)
      } catch {
        console.warn('Failed to parse WebSocket message:', event.data)
      }
    }

    this.ws.onclose = () => {
      this.onStateChange?.('disconnected')
      if (!this.intentionalClose) {
        this.scheduleReconnect()
      }
    }

    this.ws.onerror = () => {
      this.onStateChange?.('disconnected')
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.warn('Max reconnect attempts reached')
      return
    }

    this.reconnectAttempts++
    this.onStateChange?.('reconnecting')
    
    const delay = Math.min(
      this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1),
      30000
    )

    setTimeout(() => {
      if (!this.intentionalClose) {
        this.connect()
      }
    }, delay)
  }

  disconnect(): void {
    this.intentionalClose = true
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
    this.onStateChange?.('disconnected')
  }

  send(data: unknown): boolean {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data))
      return true
    }
    return false
  }

  get state(): ConnectionState {
    if (!this.ws) return 'disconnected'
    switch (this.ws.readyState) {
      case WebSocket.CONNECTING:
        return 'connecting'
      case WebSocket.OPEN:
        return 'connected'
      case WebSocket.CLOSING:
      case WebSocket.CLOSED:
      default:
        return 'disconnected'
    }
  }
}

export const wsClient = new WebSocketClient()
