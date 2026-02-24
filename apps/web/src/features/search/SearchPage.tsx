import { useState } from 'react'
import { useSearchStore } from './searchStore'
import { SearchBar } from './SearchBar'
import { SearchResultList } from './SearchResultList'
import styles from './SearchPage.module.css'

export function SearchPage() {
  const { results, loading, error, search } = useSearchStore()
  const [query, setQuery] = useState('')

  const handleSearch = (q: string) => {
    setQuery(q)
    if (q.trim()) {
      search(q.trim())
    }
  }

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h2>Search</h2>
      </div>

      <SearchBar onSearch={handleSearch} />

      {error && <div className={styles.error}>{error}</div>}

      {query.trim() && !error && (
        <SearchResultList results={results} loading={loading} />
      )}
    </div>
  )
}

export default SearchPage
