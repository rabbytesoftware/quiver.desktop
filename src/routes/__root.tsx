// src/routes/__root.tsx
import { getCurrentWindow } from '@tauri-apps/api/window'
import { createRootRoute, Outlet } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { useEffect } from 'react'
import { Sidebar } from '@/components/sidebar/Sidebar'
import { ResizeHandle } from '@/components/ResizeHandle'
import { useArrowStore } from '@/lib/core-store'
import type { ArrowEntry } from '@/domain/arrow'
import '../index.css'

export const Route = createRootRoute({
  component: RootLayout,
})

const MOCK_ARROWS: ArrowEntry[] = [
  { namespace: 'github.com/char2cs/quiver', name: 'quiver', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/char2cs/quiver-ui', name: 'quiver-ui', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/char2cs/quiver.experiments', name: 'experiments', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/mateo/dotfiles', name: 'dotfiles', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/mateo/dev-tools', name: 'dev-tools', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/someone/nvim-config', name: 'nvim-config', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/someone/zsh-extras', name: 'zsh-extras', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/nerd/fonts', name: 'fonts', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/utils/cli-helpers', name: 'cli-helpers', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
  { namespace: 'github.com/utils/scripts', name: 'scripts', description: '', tags: [], icon: null, banner: null, version: '1.0.0', state: 'ready', active_run: null, last_return: null },
]

function RootLayout() {
  const upsertArrow = useArrowStore((s) => s.upsertArrow)

  useEffect(() => {
    MOCK_ARROWS.forEach(upsertArrow)
  }, [upsertArrow])

  useEffect(() => {
    const appWindow = getCurrentWindow()
    const onMouseDown = (e: MouseEvent) => {
      const target = e.target as HTMLElement
      if (target.closest('[data-tauri-drag-region]')) {
        appWindow.startDragging()
      }
    }
    document.addEventListener('mousedown', onMouseDown)
    return () => document.removeEventListener('mousedown', onMouseDown)
  }, [])

  return (
    <div className="flex h-full">
      <Sidebar />
      <ResizeHandle />
      <main className="flex-1 overflow-hidden">
        <Outlet />
      </main>
      <TanStackRouterDevtools position="bottom-right" />
    </div>
  )
}
