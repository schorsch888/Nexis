import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'

interface Room {
  id: string
  name: string
  topic?: string
}

export function HomePage() {
  const [rooms, setRooms] = useState<Room[]>([])
  const [newRoomName, setNewRoomName] = useState('')
  const [loading, setLoading] = useState(false)
  const navigate = useNavigate()

  useEffect(() => {
    // TODO: Fetch rooms from API
    setRooms([])
  }, [])

  const createRoom = async () => {
    if (!newRoomName.trim()) return
    
    setLoading(true)
    try {
      const response = await fetch('/api/v1/rooms', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: newRoomName })
      })
      
      if (response.ok) {
        const room = await response.json()
        navigate(`/rooms/${room.id}`)
      }
    } catch (error) {
      console.error('Failed to create room:', error)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="container">
      <h2>Welcome to Nexis</h2>
      <p>AI-Native Team Communication Platform</p>
      
      <div style={{ margin: '2rem 0' }}>
        <h3>Create a Room</h3>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <input
            type="text"
            placeholder="Room name..."
            value={newRoomName}
            onChange={(e) => setNewRoomName(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && createRoom()}
          />
          <button onClick={createRoom} disabled={loading}>
            {loading ? 'Creating...' : 'Create'}
          </button>
        </div>
      </div>

      <div className="room-list">
        <h3>Rooms</h3>
        {rooms.length === 0 ? (
          <p>No rooms yet. Create one to get started!</p>
        ) : (
          rooms.map((room) => (
            <div key={room.id} className="room-card">
              <h4>{room.name}</h4>
              {room.topic && <p>{room.topic}</p>}
            </div>
          ))
        )}
      </div>
    </div>
  )
}
