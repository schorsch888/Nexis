import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { useRoomsStore } from './roomsStore'
import styles from './RoomListPage.module.css'

export function RoomListPage() {
  const { rooms, loading, error, fetchRooms, createRoom } = useRoomsStore()
  const [newRoomName, setNewRoomName] = useState('')
  const [showCreate, setShowCreate] = useState(false)

  useEffect(() => {
    fetchRooms()
  }, [fetchRooms])

  const handleCreate = async () => {
    if (!newRoomName.trim()) return
    await createRoom(newRoomName.trim())
    setNewRoomName('')
    setShowCreate(false)
  }

  if (error) {
    return (
      <div className={styles.container}>
        <div className={styles.error}>{error}</div>
        <button onClick={() => fetchRooms()}>Retry</button>
      </div>
    )
  }

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h2>Rooms</h2>
        <button onClick={() => setShowCreate(true)} className={styles.createBtn}>
          Create Room
        </button>
      </div>

      {showCreate && (
        <div className={styles.createForm}>
          <input
            type="text"
            value={newRoomName}
            onChange={(e) => setNewRoomName(e.target.value)}
            placeholder="Room name"
            onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
          />
          <button onClick={handleCreate}>Create</button>
          <button onClick={() => setShowCreate(false)}>Cancel</button>
        </div>
      )}

      {loading ? (
        <div className={styles.loading}>Loading rooms...</div>
      ) : rooms.length === 0 ? (
        <div className={styles.empty}>
          <p>No rooms yet. Create one to get started!</p>
        </div>
      ) : (
        <div className={styles.grid}>
          {rooms.map((room) => (
            <Link key={room.id} to={`/app/rooms/${room.id}`} className={styles.card}>
              <h3>{room.name}</h3>
              {room.topic && <p>{room.topic}</p>}
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}

export default RoomListPage
