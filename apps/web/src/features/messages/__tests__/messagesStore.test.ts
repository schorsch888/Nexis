import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useMessagesStore } from '../messagesStore'
import { useAuthStore, resetAuthStore } from '../../auth/authStore'
import { httpClient } from '../../../shared/api/httpClient'

vi.mock('../../../shared/api/httpClient', () => ({
  httpClient: {
    get: vi.fn(),
    post: vi.fn(),
  },
}))

describe('messagesStore', () => {
  beforeEach(() => {
    resetAuthStore()
    useAuthStore.getState().login({
      token: 'token',
      memberId: 'nexis:human:tester',
      tenantId: 'tenant-1',
    })

    useMessagesStore.getState().disconnect()
    useMessagesStore.setState({
      messages: [],
      loading: false,
      error: null,
      unreadCount: 0,
      connectionState: 'disconnected',
    })

    vi.mocked(httpClient.post).mockReset()
    vi.mocked(httpClient.get).mockReset()
  })

  it('queues outgoing messages while offline and flushes them when recovered', async () => {
    vi.mocked(httpClient.post).mockResolvedValueOnce({
      data: {
        id: 'server-1',
        roomId: 'room-1',
        sender: 'nexis:human:tester',
        text: 'hello',
        timestamp: new Date().toISOString(),
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    } as any)

    await useMessagesStore.getState().sendMessage('room-1', 'hello')

    const queued = useMessagesStore.getState().messages[0]
    expect(queued.deliveryStatus).toBe('sent')

    await useMessagesStore.getState().flushOfflineQueue()

    const delivered = useMessagesStore.getState().messages[0]
    expect(delivered.deliveryStatus).toBe('delivered')
    expect(delivered.id).toBe('server-1')
  })

  it('marks message as failed when sending fails online', async () => {
    useMessagesStore.getState().setConnectionState('connected')
    vi.mocked(httpClient.post).mockRejectedValueOnce(new Error('network'))

    await useMessagesStore.getState().sendMessage('room-1', 'oops')

    const failed = useMessagesStore.getState().messages[0]
    expect(failed.deliveryStatus).toBe('failed')
  })

  it('tracks unread count and marks messages as read', () => {
    useMessagesStore.getState().handleRealtimeEvent({
      type: 'message',
      payload: {
        id: 'msg-1',
        roomId: 'room-1',
        sender: 'nexis:human:peer',
        text: 'ping',
        timestamp: new Date().toISOString(),
      },
    })

    expect(useMessagesStore.getState().unreadCount).toBe(1)

    useMessagesStore.getState().markAllRead()
    expect(useMessagesStore.getState().unreadCount).toBe(0)
    expect(useMessagesStore.getState().messages[0].deliveryStatus).toBe('read')
  })
})
