import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Navigate, RouterProvider, createBrowserRouter } from 'react-router'

// Served with the app rather than from Google, so it opens with no signal.
import '@fontsource-variable/manrope'
import '@fontsource/unbounded/latin-600.css'
import '@fontsource/unbounded/cyrillic-600.css'

import { MiniApp } from './app/MiniApp'
import { CoachArea } from './coach/CoachArea'
import { UpdateNote } from './shared/UpdateNote'
import { registerServiceWorker } from './shared/appUpdate'
import { placeCaretOnFocus } from './shared/fieldFocus'
import './styles.css'

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
})

const router = createBrowserRouter([
  { path: '/', element: <Navigate to="/coach" replace /> },
  // Opened from the bot inside Telegram: a client's workouts, or Dasha's workspace.
  { path: '/app/*', element: <MiniApp /> },
  // Dasha's workspace in a normal browser, signed in through the bot.
  { path: '/coach/*', element: <CoachArea /> },
])

placeCaretOnFocus()
registerServiceWorker()

const root = document.getElementById('root')
if (!root) throw new Error('index.html has no #root element')

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <UpdateNote />
    </QueryClientProvider>
  </StrictMode>,
)
