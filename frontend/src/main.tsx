import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider, createBrowserRouter, redirect } from 'react-router'

// Served with the app rather than from Google, so it opens with no signal.
import '@fontsource-variable/manrope'
import '@fontsource/unbounded/latin-600.css'
import '@fontsource/unbounded/cyrillic-600.css'

import { InstallPage } from './app/InstallPage'
import { MiniApp } from './app/MiniApp'
import { StandaloneApp } from './app/StandaloneApp'
import { telegramWebApp } from './app/telegram'
import { UpdateNote } from './shared/UpdateNote'
import { registerServiceWorker } from './shared/appUpdate'
import { placeCaretOnFocus } from './shared/fieldFocus'
import { listenForInstallPrompt } from './shared/installPrompt'
import { listenForNotifications } from './shared/push'
import './styles.css'

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
})

// A client's workouts or Dasha's workspace, whoever signs in. Inside Telegram
// Telegram vouches for them; installed on a phone or in a browser, the bot does.
const router = createBrowserRouter([
  { path: '/', loader: () => redirect('/app') },
  { path: '/app/*', element: telegramWebApp() ? <MiniApp /> : <StandaloneApp /> },
  // How to put the app on a phone's home screen.
  { path: '/install', element: <InstallPage /> },
  // Dasha's old browser address: `/coach/…` is `/app/…` now.
  {
    path: '/coach/*',
    loader: ({ request }) => {
      const url = new URL(request.url)
      return redirect(url.pathname.replace(/^\/coach/, '/app') + url.search)
    },
  },
])

placeCaretOnFocus()
registerServiceWorker()
listenForInstallPrompt()
listenForNotifications((url) => void router.navigate(url))

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
