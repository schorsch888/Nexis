import { httpClient } from '../httpClient'

export interface SearchResult {
  id: string
  type: 'message' | 'room' | 'member'
  title: string
  snippet?: string
  roomId?: string
}

export const searchApi = {
  search: (query: string, limit = 20) =>
    httpClient.post<SearchResult[]>('/search', { query, limit }),
}
