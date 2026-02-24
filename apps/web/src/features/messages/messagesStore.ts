import { create } from 'zustand'
import type { Message } from '../../shared/api/endpoints/messages'
import { httpClient } from '../../shared/api/httpClient'

export type { Message }

export type ConnectionState = 'connected' | 'connecting' | 'disconnected' | 'reconnecting'

interface MessagesState {
  messages: Message[]
  loading: boolean
  error: string | null
  connectionState: ConnectionState
  setMessages: (messages: Message[]) => void
  addMessage: (message: Message) => void
  fetchMessages: (roomId: string) => Promise<void>
  sendMessage: (roomId: string, text: string) => Promise<void>
  connect: (roomId: string) => void
  disconnect: () => void
  setConnectionState: (state: ConnectionState) => void
}

export const useMessagesStore = create<MessagesState>((set) => ({
  messages: [],
  loading: false,
  error: null,
  connectionState: 'disconnected',

  setMessages: (messages) => set({ messages }),
  addMessage: (message) => set((state) => ({ messages: [...state.messages, message] })),
  setConnectionState: (state) => set({ connectionState: state }),

  fetchMessages: async (roomId: string) => {
    set({ loading: true, error: null })
    try {
      const response = await httpClient.get<Message[]>(`/rooms/${roomId}/messages`)
      set({ messages: response.data, loading: false })
    } catch {
      set({ error: 'Failed to fetch messages', loading: false })
    }
  },

  sendMessage: async (roomId: string, text: string) => {
    try {
      const { useAuthStore } = await import('../auth/authStore')
      const sender = useAuthStore.getState().memberId || 'nexis:human:unknown'
      const response = await httpClient.post<Message>('/messages', {
        roomId,
        sender,
        text,
      })
      set((state) => ({ messages: [...state.messages, response.data] }))
    } catch {
      set({ error: 'Failed to send message' })
    }
  },

  connect: (_roomId: string) => {
    set({ connectionState: 'connecting' })
  },

  disconnect: () => {
    set({ connectionState: 'disconnected' })
  },
}))
