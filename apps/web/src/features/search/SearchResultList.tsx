import { Link } from 'react-router-dom'
import type { SearchResult } from './searchStore'
import styles from './SearchResultList.module.css'

interface SearchResultListProps {
  results: SearchResult[]
  loading: boolean
}

export function SearchResultList({ results, loading }: SearchResultListProps) {
  if (loading) {
    return <div className={styles.loading}>Searching...</div>
  }

  if (results.length === 0) {
    return <div className={styles.empty}>No results found</div>
  }

  return (
    <div className={styles.list}>
      {results.map((result) => (
        <div key={`${result.type}-${result.id}`} className={styles.item}>
          {result.type === 'message' && result.roomId ? (
            <Link to={`/app/rooms/${result.roomId}`} className={styles.link}>
              <span className={styles.type}>Message</span>
              <span className={styles.title}>{result.title}</span>
              {result.snippet && <span className={styles.snippet}>{result.snippet}</span>}
            </Link>
          ) : result.type === 'room' ? (
            <Link to={`/app/rooms/${result.id}`} className={styles.link}>
              <span className={styles.type}>Room</span>
              <span className={styles.title}>{result.title}</span>
            </Link>
          ) : (
            <div className={styles.link}>
              <span className={styles.type}>Member</span>
              <span className={styles.title}>{result.title}</span>
            </div>
          )}
        </div>
      ))}
    </div>
  )
}
