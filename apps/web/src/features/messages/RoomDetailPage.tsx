import { useState, useEffect, useRef } from 'react'
import { useParams } from 'react-router-dom'
import { useMessagesStore } from './messagesStore'
import { useRoomsStore } from '../rooms/roomsStore'
import { ConnectionBadge } from './ConnectionBadge'
import styles from './RoomDetailPage.module.css'

export function RoomDetailPage() {
  const { roomId } = useParams<{ roomId: string }>()
  const { currentRoom, fetchRoom } = useRoomsStore()
  const { messages, loading, error, sendMessage, fetchMessages, connect, disconnect, connectionState } = useMessagesStore()
  const [messageText, setMessageText] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (roomId) {
      fetchRoom(roomId)
      fetchMessages(roomId)
      connect(roomId)
    }
    return () => disconnect()
  }, [roomId, fetchRoom, fetchMessages, connect, disconnect])

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  const handleSend = async () => {
    if (!messageText.trim() || !roomId) return
    await sendMessage(roomId, messageText.trim())
    setMessageText('')
  }

  if (!currentRoom && loading) {
    return <div className={styles.container}>Loading...</div>
  }

  if (!currentRoom) {
    return <div className={styles.container}>Room not found</div>
  }

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <div>
          <h2>{currentRoom.name}</h2>
          {currentRoom.topic && <p className={styles.topic}>{currentRoom.topic}</p>}
        </div>
        <ConnectionBadge state={connectionState} />
      </div>

      {error && <div className={styles.error}>{error}</div>}

      <div className={styles.messages}>
        {messages.length === 0 ? (
          <div className={styles.empty}>No messages yet. Start the conversation!</div>
        ) : (
          messages.map((msg) => (
            <div key={msg.id} className={`${styles.message} ${msg.sender.startsWith('nexis:ai:') ? styles.ai : ''}`}>
              <div className={styles.sender}>{msg.sender}</div>
              <div className={styles.text}>{msg.text}</div>
              <div className={styles.time}>{new Date(msg.timestamp).toLocaleTimeString()}</div>
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className={styles.input}>
        <input
          type="text"
          value={messageText}
          onChange={(e) => setMessageText(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleSend()}
          placeholder="Type a message..."
        />
        <button onClick={handleSend}>Send</button>
      </div>
    </div>
  )
}

export default RoomDetailPage
