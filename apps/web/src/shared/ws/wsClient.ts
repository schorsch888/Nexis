import { useAuthStore } from '../../features/auth/authStore'
import type { ConnectionState, WebSocketClientOptions, WebSocketMessage } from './types'

const WS_URL = import.meta.env.VITE_WS_URL || `ws://${window.location.host}/ws`
const MAX_RECONNECT_DELAY = 30000
const DEFAULT_HEARTBEAT_INTERVAL = 15000
const DEFAULT_HEARTBEAT_TIMEOUT = 7000

export class WebSocketClient {
  private ws: WebSocket | null = null
  private reconnectAttempts = 0
  private readonly maxReconnectAttempts: number
  private readonly reconnectDelay: number
  private readonly heartbeatInterval: number
  private readonly heartbeatTimeout: number
  private onMessage?: (data: WebSocketMessage) => void
  private onStateChange?: (state: ConnectionState) => void
  private intentionalClose = false
  private currentState: ConnectionState = 'disconnected'
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null
  private heartbeatTimeoutTimer: ReturnType<typeof setTimeout> | null = null
  private lastRoomId?: string
  private offlineQueue: string[] = []
  private readonly onOnline = () => {
    if (!this.intentionalClose && this.currentState !== 'connected') {
      this.connect(this.lastRoomId)
    }
  }
  private readonly onOffline = () => {
    this.setState('disconnected')
  }

  constructor(options: WebSocketClientOptions = {}) {
    this.maxReconnectAttempts = options.maxReconnectAttempts ?? 5
    this.reconnectDelay = options.reconnectDelay ?? 1000
    this.heartbeatInterval = options.heartbeatInterval ?? DEFAULT_HEARTBEAT_INTERVAL
    this.heartbeatTimeout = options.heartbeatTimeout ?? DEFAULT_HEARTBEAT_TIMEOUT
    this.onMessage = options.onMessage
    this.onStateChange = options.onStateChange

    if (typeof window !== 'undefined') {
      window.addEventListener('online', this.onOnline)
      window.addEventListener('offline', this.onOffline)
    }
  }

  connect(roomId?: string): void {
    this.intentionalClose = false
    this.lastRoomId = roomId ?? this.lastRoomId
    this.clearReconnectTimer()
    this.clearHeartbeatTimers()
    this.setState(this.reconnectAttempts > 0 ? 'reconnecting' : 'connecting')

    const { token } = useAuthStore.getState()
    let url = WS_URL
    if (token) {
      url += `?token=${encodeURIComponent(token)}`
    }
    if (this.lastRoomId) {
      url += `${token ? '&' : '?'}roomId=${encodeURIComponent(this.lastRoomId)}`
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
      this.setState('connected')
      this.startHeartbeat()
      this.flushOfflineQueue()
    }

    this.ws.onmessage = (event) => {
      this.clearHeartbeatTimeout()
      try {
        const data = JSON.parse(event.data) as WebSocketMessage
        if (data.type === 'pong' || data.type === 'heartbeat') {
          return
        }
        this.onMessage?.(data)
      } catch {
        console.warn('Failed to parse WebSocket message:', event.data)
      }
    }

    this.ws.onclose = () => {
      this.clearHeartbeatTimers()
      this.setState('disconnected')
      if (!this.intentionalClose) {
        this.scheduleReconnect()
      }
    }

    this.ws.onerror = () => {
      this.clearHeartbeatTimers()
      this.setState('disconnected')
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.warn('Max reconnect attempts reached')
      return
    }

    this.reconnectAttempts++
    this.setState('reconnecting')
    const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), MAX_RECONNECT_DELAY)

    this.reconnectTimer = setTimeout(() => {
      if (!this.intentionalClose) {
        this.connect(this.lastRoomId)
      }
    }, delay)
  }

  disconnect(): void {
    this.intentionalClose = true
    this.clearReconnectTimer()
    this.clearHeartbeatTimers()
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
    this.setState('disconnected')
  }

  send(data: unknown): boolean {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data))
      return true
    }
    this.offlineQueue.push(JSON.stringify(data))
    return false
  }

  get state(): ConnectionState {
    return this.currentState
  }

  get queuedCount(): number {
    return this.offlineQueue.length
  }

  setHandlers(handlers: Pick<WebSocketClientOptions, 'onMessage' | 'onStateChange'>): void {
    if (handlers.onMessage) {
      this.onMessage = handlers.onMessage
    }
    if (handlers.onStateChange) {
      this.onStateChange = handlers.onStateChange
    }
  }

  private setState(next: ConnectionState): void {
    this.currentState = next
    this.onStateChange?.(next)
  }

  private flushOfflineQueue(): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN || this.offlineQueue.length === 0) {
      return
    }
    while (this.offlineQueue.length > 0) {
      const payload = this.offlineQueue.shift()
      if (payload) {
        this.ws.send(payload)
      }
    }
  }

  private startHeartbeat(): void {
    this.clearHeartbeatTimers()
    this.heartbeatTimer = setInterval(() => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
        return
      }
      this.ws.send(JSON.stringify({ type: 'heartbeat', payload: { at: Date.now() } }))
      this.heartbeatTimeoutTimer = setTimeout(() => {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
          this.ws.close()
        }
      }, this.heartbeatTimeout)
    }, this.heartbeatInterval)
  }

  private clearHeartbeatTimeout(): void {
    if (this.heartbeatTimeoutTimer) {
      clearTimeout(this.heartbeatTimeoutTimer)
      this.heartbeatTimeoutTimer = null
    }
  }

  private clearHeartbeatTimers(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer)
      this.heartbeatTimer = null
    }
    this.clearHeartbeatTimeout()
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }
}

export const wsClient = new WebSocketClient()
