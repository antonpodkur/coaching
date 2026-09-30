import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Navigate, RouterProvider, createBrowserRouter } from 'react-router'

import { MiniApp } from './app/MiniApp'
import { CoachArea } from './coach/CoachArea'
import './styles.css'

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
})

const router = createBrowserRouter([
  { path: '/', element: <Navigate to="/coach" replace /> },
  // Opened by the bot's button inside Telegram.
  { path: '/app/*', element: <MiniApp /> },
  // Dasha's workspace in a normal browser.
  { path: '/coach/*', element: <CoachArea /> },
])

const root = document.getElementById('root')
if (!root) throw new Error('index.html has no #root element')

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
)
