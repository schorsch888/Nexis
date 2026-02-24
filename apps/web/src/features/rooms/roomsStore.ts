import { create } from 'zustand'
import type { Room } from '../../shared/api/endpoints/rooms'
import { httpClient } from '../../shared/api/httpClient'

export type { Room }

interface RoomsState {
  rooms: Room[]
  currentRoom: Room | null
  loading: boolean
  error: string | null
  fetchRooms: () => Promise<void>
  fetchRoom: (id: string) => Promise<void>
  createRoom: (name: string, topic?: string) => Promise<Room | null>
  setCurrentRoom: (room: Room | null) => void
}

export const useRoomsStore = create<RoomsState>((set) => ({
  rooms: [],
  currentRoom: null,
  loading: false,
  error: null,

  fetchRooms: async () => {
    set({ loading: true, error: null })
    try {
      const response = await httpClient.get<Room[]>('/rooms')
      set({ rooms: response.data, loading: false })
    } catch {
      set({ error: 'Failed to fetch rooms', loading: false })
    }
  },

  fetchRoom: async (id: string) => {
    set({ loading: true, error: null })
    try {
      const response = await httpClient.get<Room>(`/rooms/${id}`)
      set({ currentRoom: response.data, loading: false })
    } catch {
      set({ error: 'Failed to fetch room', loading: false })
    }
  },

  createRoom: async (name: string, topic?: string) => {
    try {
      const response = await httpClient.post<Room>('/rooms', { name, topic })
      const newRoom = response.data
      set((state) => ({ rooms: [...state.rooms, newRoom] }))
      return newRoom
    } catch {
      set({ error: 'Failed to create room' })
      return null
    }
  },

  setCurrentRoom: (room: Room | null) => set({ currentRoom: room }),
}))
