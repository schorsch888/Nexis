import { httpClient } from '../httpClient'

export interface Message {
  id: string
  roomId: string
  sender: string
  text: string
  timestamp: string
}

export const messagesApi = {
  list: (roomId: string) => httpClient.get<Message[]>(`/rooms/${roomId}/messages`),
  send: (roomId: string, sender: string, text: string) =>
    httpClient.post<Message>('/messages', { roomId, sender, text }),
}
