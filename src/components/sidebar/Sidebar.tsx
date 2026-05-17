import { WindowControls } from 'tauri-controls'
import { useUIStore } from '@/store/ui'
import { NavBar } from './NavBar'
import { ArrowList } from './ArrowList'

export function Sidebar() {
  const sidebarWidth = useUIStore((s) => s.sidebarWidth)

  return (
    <div
      className="flex flex-shrink-0 flex-col overflow-hidden"
      style={{ width: sidebarWidth }}
    >
      {/* Top nav — drag region */}
      <div
        className="flex flex-shrink-0 items-center px-2.5 py-2.5"
        data-tauri-drag-region
      >
        <WindowControls />

        {/* Server icon — decorative placeholder */}
        <div className="ml-2 flex items-center opacity-35">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
            <rect x="1" y="2" width="14" height="4" rx="1" stroke="white" strokeWidth="1.3" />
            <rect x="1" y="9" width="14" height="4" rx="1" stroke="white" strokeWidth="1.3" />
            <circle cx="12.5" cy="4" r="1" fill="white" />
            <circle cx="12.5" cy="11" r="1" fill="white" />
          </svg>
        </div>

        {/* Options icon — decorative, pushed right */}
        <div className="ml-auto flex items-center opacity-35">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="3" r="1.2" fill="white" />
            <circle cx="8" cy="8" r="1.2" fill="white" />
            <circle cx="8" cy="13" r="1.2" fill="white" />
          </svg>
        </div>
      </div>

      <NavBar />
      <ArrowList />
    </div>
  )
}
