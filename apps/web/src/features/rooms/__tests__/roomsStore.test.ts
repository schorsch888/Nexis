import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useRoomsStore } from '../roomsStore'

vi.mock('../../../shared/api/httpClient', () => ({
  httpClient: {
    get: vi.fn(),
    post: vi.fn(),
  },
}))

describe('roomsStore', () => {
  beforeEach(() => {
    useRoomsStore.setState({
      rooms: [],
      currentRoom: null,
      loading: false,
      error: null,
    })
  })

  it('should start with default values', () => {
    const state = useRoomsStore.getState()
    expect(state.rooms).toEqual([])
    expect(state.currentRoom).toBeNull()
    expect(state.loading).toBe(false)
    expect(state.error).toBeNull()
  })

  it('should set current room', () => {
    const { setCurrentRoom } = useRoomsStore.getState()
    const room = { id: '1', name: 'Test Room' }
    
    setCurrentRoom(room)
    
    const state = useRoomsStore.getState()
    expect(state.currentRoom).toEqual(room)
  })
})
