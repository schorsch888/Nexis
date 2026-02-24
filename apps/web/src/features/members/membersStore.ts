import { create } from 'zustand'
import type { Member } from '../../shared/api/endpoints/members'
import { httpClient } from '../../shared/api/httpClient'

export type { Member }

interface MembersState {
  members: Member[]
  loading: boolean
  error: string | null
  fetchMembers: () => Promise<void>
  inviteMember: (email: string, role?: string) => Promise<boolean>
}

export const useMembersStore = create<MembersState>((set) => ({
  members: [],
  loading: false,
  error: null,

  fetchMembers: async () => {
    set({ loading: true, error: null })
    try {
      const response = await httpClient.get<Member[]>('/members')
      set({ members: response.data, loading: false })
    } catch {
      set({ error: 'Failed to fetch members', loading: false })
    }
  },

  inviteMember: async (email: string, role?: string) => {
    try {
      await httpClient.post('/members/invite', { email, role })
      set((state) => ({ members: [...state.members, { id: '', email, role: role || 'member' }] }))
      return true
    } catch {
      set({ error: 'Failed to invite member' })
      return false
    }
  },
}))
