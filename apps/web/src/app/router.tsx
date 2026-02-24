import { createBrowserRouter, Navigate } from 'react-router-dom'
import { ProtectedRoute } from './routes/ProtectedRoute'
import { AppShell } from './layout/AppShell'
import { LoginPage } from '../features/auth/LoginPage'
import { RoomListPage } from '../features/rooms/RoomListPage'
import { RoomDetailPage } from '../features/messages/RoomDetailPage'
import { MembersPage } from '../features/members/MembersPage'
import { SearchPage } from '../features/search/SearchPage'

export const router = createBrowserRouter([
  {
    path: '/login',
    element: <LoginPage />,
  },
  {
    path: '/app',
    element: (
      <ProtectedRoute>
        <AppShell />
      </ProtectedRoute>
    ),
    children: [
      {
        index: true,
        element: <Navigate to="/app/rooms" replace />,
      },
      {
        path: 'rooms',
        element: <RoomListPage />,
      },
      {
        path: 'rooms/:roomId',
        element: <RoomDetailPage />,
      },
      {
        path: 'members',
        element: <MembersPage />,
      },
      {
        path: 'search',
        element: <SearchPage />,
      },
    ],
  },
  {
    path: '/',
    element: <Navigate to="/app/rooms" replace />,
  },
])
