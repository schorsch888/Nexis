import { create } from 'zustand'
import type { SearchResult } from '../../shared/api/endpoints/search'
import { httpClient } from '../../shared/api/httpClient'

export type { SearchResult }

interface SearchState {
  query: string
  results: SearchResult[]
  loading: boolean
  error: string | null
  search: (query: string, limit?: number) => Promise<void>
}

export const useSearchStore = create<SearchState>((set) => ({
  query: '',
  results: [],
  loading: false,
  error: null,

  search: async (query: string, limit = 20) => {
    set({ loading: true, error: null, query })
    try {
      const response = await httpClient.post<SearchResult[]>('/search', {
        query,
        limit,
      })
      set({ results: response.data, loading: false })
    } catch {
      set({ error: 'Search failed', loading: false })
    }
  },
}))
