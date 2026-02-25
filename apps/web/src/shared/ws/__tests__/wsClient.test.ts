import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { WebSocketClient } from '../wsClient'
import type { ConnectionState } from '../types'

class MockWebSocket {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  static instances: MockWebSocket[] = []

  readonly url: string
  readyState = MockWebSocket.CONNECTING
  sent: string[] = []

  onopen: (() => void) | null = null
  onmessage: ((event: { data: string }) => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null

  constructor(url: string) {
    this.url = url
    MockWebSocket.instances.push(this)
  }

  send(data: string): void {
    this.sent.push(data)
  }

  close(): void {
    this.readyState = MockWebSocket.CLOSED
    this.onclose?.()
  }

  triggerOpen(): void {
    this.readyState = MockWebSocket.OPEN
    this.onopen?.()
  }

  triggerMessage(data: string): void {
    this.onmessage?.({ data })
  }

  triggerClose(): void {
    this.readyState = MockWebSocket.CLOSED
    this.onclose?.()
  }
}

describe('WebSocketClient', () => {
  const originalWebSocket = globalThis.WebSocket

  beforeEach(() => {
    vi.useFakeTimers()
    MockWebSocket.instances = []
    Object.defineProperty(globalThis, 'WebSocket', {
      value: MockWebSocket,
      configurable: true,
      writable: true,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
    Object.defineProperty(globalThis, 'WebSocket', {
      value: originalWebSocket,
      configurable: true,
      writable: true,
    })
  })

  it('tracks connection state and reconnects with exponential backoff', () => {
    const states: ConnectionState[] = []
    const client = new WebSocketClient({
      onStateChange: (state) => states.push(state),
      reconnectDelay: 100,
      maxReconnectAttempts: 2,
    })

    client.connect('room-1')
    expect(states).toEqual(['connecting'])

    const firstSocket = MockWebSocket.instances[0]
    firstSocket.triggerOpen()
    expect(states.at(-1)).toBe('connected')

    firstSocket.triggerClose()
    expect(states.at(-2)).toBe('disconnected')
    expect(states.at(-1)).toBe('reconnecting')

    vi.advanceTimersByTime(100)
    expect(MockWebSocket.instances).toHaveLength(2)
  })

  it('sends heartbeats and reconnects when heartbeat times out', () => {
    const states: ConnectionState[] = []
    const client = new WebSocketClient({
      onStateChange: (state) => states.push(state),
      heartbeatInterval: 1000,
      heartbeatTimeout: 500,
      reconnectDelay: 50,
    })

    client.connect()
    const socket = MockWebSocket.instances[0]
    socket.triggerOpen()

    vi.advanceTimersByTime(1000)
    expect(socket.sent).toHaveLength(1)

    vi.advanceTimersByTime(500)
    expect(states).toContain('reconnecting')
  })

  it('queues outbound messages while disconnected and flushes on connect', () => {
    const client = new WebSocketClient()
    client.connect('room-2')

    const sentNow = client.send({ type: 'message', payload: { text: 'hello' } })
    expect(sentNow).toBe(false)
    expect(client.queuedCount).toBe(1)

    const socket = MockWebSocket.instances[0]
    socket.triggerOpen()

    expect(client.queuedCount).toBe(0)
    expect(socket.sent).toHaveLength(1)
  })
})
