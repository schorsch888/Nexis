import { useState, useEffect, useRef } from 'react'
import { useParams } from 'react-router-dom'

interface Message {
  id: string
  sender: string
  text: string
  timestamp: string
}

interface Room {
  id: string
  name: string
  topic?: string
  messages: Message[]
}

export function RoomPage() {
  const { roomId } = useParams<{ roomId: string }>()
  const [room, setRoom] = useState<Room | null>(null)
  const [messageText, setMessageText] = useState('')
  const [connected, setConnected] = useState(false)
  const wsRef = useRef<WebSocket | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    // Fetch room info
    fetchRoom()
    
    // Connect to WebSocket
    const ws = new WebSocket(`ws://${window.location.host}/ws`)
    
    ws.onopen = () => {
      setConnected(true)
      console.log('WebSocket connected')
    }
    
    ws.onmessage = (event) => {
      console.log('Received:', event.data)
      // Handle incoming messages
    }
    
    ws.onclose = () => {
      setConnected(false)
      console.log('WebSocket disconnected')
    }
    
    wsRef.current = ws
    
    return () => {
      ws.close()
    }
  }, [roomId])

  useEffect(() => {
    // Scroll to bottom when new messages arrive
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [room?.messages])

  const fetchRoom = async () => {
    try {
      const response = await fetch(`/api/v1/rooms/${roomId}`)
      if (response.ok) {
        const data = await response.json()
        setRoom(data)
      }
    } catch (error) {
      console.error('Failed to fetch room:', error)
    }
  }

  const sendMessage = async () => {
    if (!messageText.trim() || !roomId) return
    
    try {
      await fetch('/api/v1/messages', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          roomId,
          sender: 'nexis:human:user@example.com',
          text: messageText
        })
      })
      
      setMessageText('')
      fetchRoom() // Refresh room to get new messages
    } catch (error) {
      console.error('Failed to send message:', error)
    }
  }

  const isAIMessage = (sender: string) => sender.startsWith('nexis:ai:')

  if (!room) {
    return <div className="container">Loading...</div>
  }

  return (
    <div className="chat-container">
      <div style={{ padding: '1rem', borderBottom: '1px solid #ddd' }}>
        <h2 style={{ margin: 0 }}>{room.name}</h2>
        {room.topic && <p style={{ margin: 0, color: '#666' }}>{room.topic}</p>}
        <span style={{ fontSize: '0.8rem', color: connected ? 'green' : 'red' }}>
          {connected ? '● Connected' : '○ Disconnected'}
        </span>
      </div>
      
      <div className="message-list">
        {room.messages.map((msg) => (
          <div 
            key={msg.id} 
            className={`message ${isAIMessage(msg.sender) ? 'ai' : ''}`}
          >
            <div style={{ fontSize: '0.8rem', color: '#666', marginBottom: '0.25rem' }}>
              {msg.sender}
            </div>
            <div>{msg.text}</div>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>
      
      <div className="message-input">
        <input
          type="text"
          placeholder="Type a message..."
          value={messageText}
          onChange={(e) => setMessageText(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && sendMessage()}
        />
        <button onClick={sendMessage}>Send</button>
      </div>
    </div>
  )
}
