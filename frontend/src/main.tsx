// First, so that in the landing page's demo no module reaches the real storage.
import './demo/isolate'

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import {
  Outlet,
  type RouteObject,
  RouterProvider,
  createBrowserRouter,
  createMemoryRouter,
  redirect,
} from 'react-router'

// Served with the app rather than from Google, so it opens with no signal.
import '@fontsource-variable/manrope'
import '@fontsource/unbounded/latin-600.css'
import '@fontsource/unbounded/cyrillic-600.css'

import { InstallPage } from './app/InstallPage'
import { MiniApp } from './app/MiniApp'
import { MovedPage } from './app/MovedPage'
import { StandaloneApp } from './app/StandaloneApp'
import { leaveOldAddress } from './app/moved'
import { telegramWebApp } from './app/telegram'
import { LazyDemoApp } from './demo/LazyDemoApp'
import { demo } from './demo/mode'
import { DialogHost } from './shared/DialogHost'
import { MotionProvider } from './shared/MotionProvider'
import { ScreenTransitions } from './shared/ScreenTransitions'
import { UpdateNote } from './shared/UpdateNote'
import { registerServiceWorker } from './shared/appUpdate'
import { placeCaretOnFocus } from './shared/fieldFocus'
import { trackHistory } from './shared/history'
import { listenForInstallPrompt } from './shared/installPrompt'
import { listenForNotifications } from './shared/push'
import { playScreenTransitions } from './shared/screenMotion'
import './styles.css'

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
})

// A client's workouts or Dasha's workspace, whoever signs in. Inside Telegram
// Telegram vouches for them; installed on a phone or in a browser, the bot does.
const routes: RouteObject[] = [
  {
    element: (
      <>
        <ScreenTransitions />
        <Outlet />
        <DialogHost />
      </>
    ),
    children: [
      { path: '/', loader: () => redirect('/app') },
      {
        path: '/app/*',
        // The landing page's demo (src/demo), the Mini App, or the app outside Telegram.
        element: demo ? (
          <LazyDemoApp role={demo.role} />
        ) : telegramWebApp() ? (
          <MiniApp />
        ) : (
          <StandaloneApp />
        ),
      },
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
    ],
  },
]

// The demo's two frames share the landing page's history, so each keeps its
// own in memory: going back in one never moves the other, or the page.
const router = demo
  ? createMemoryRouter(routes, { initialEntries: ['/app'] })
  : createBrowserRouter(routes)

playScreenTransitions(router, queryClient)
trackHistory(router)

placeCaretOnFocus()
// iPhones only show a pressed button (`:active`) on pages that listen for touches.
document.addEventListener('touchstart', () => undefined, { passive: true })
// The demo is a picture of the app inside the landing page: nothing to
// install, update or notify.
if (!demo) {
  registerServiceWorker()
  listenForInstallPrompt()
  listenForNotifications((url) => void router.navigate(url))
}

const root = document.getElementById('root')
if (!root) throw new Error('index.html has no #root element')

// The old address hands over to the new one; an app installed from it says so.
const address = demo ? 'here' : leaveOldAddress()
if (address === 'installed') {
  createRoot(root).render(
    <StrictMode>
      <MovedPage />
    </StrictMode>,
  )
} else if (address === 'here') {
  createRoot(root).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <MotionProvider>
          <RouterProvider router={router} />
          <UpdateNote />
        </MotionProvider>
      </QueryClientProvider>
    </StrictMode>,
  )
}
