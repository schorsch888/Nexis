import { httpClient } from '../httpClient'

export interface Room {
  id: string
  name: string
  topic?: string
  createdAt?: string
}

export const roomsApi = {
  list: () => httpClient.get<Room[]>('/rooms'),
  get: (id: string) => httpClient.get<Room>(`/rooms/${id}`),
  create: (name: string, topic?: string) => httpClient.post<Room>('/rooms', { name, topic }),
  delete: (id: string) => httpClient.delete(`/rooms/${id}`),
  invite: (roomId: string, email: string) => 
    httpClient.post(`/rooms/${roomId}/invite`, { email }),
}
