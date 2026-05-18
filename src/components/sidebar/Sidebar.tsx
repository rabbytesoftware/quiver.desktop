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
      {/* Top nav — drag region; pl-[76px] reserves space for the native traffic lights */}
      <div
        className="flex flex-shrink-0 items-center py-2.5 pr-2.5 pl-[76px]"
        data-tauri-drag-region
      >
        {/* Server icon — decorative placeholder */}
        <div className="ml-2 flex items-center text-sidebar-foreground opacity-35">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
            <rect x="1" y="2" width="14" height="4" rx="1" stroke="currentColor" strokeWidth="1.3" />
            <rect x="1" y="9" width="14" height="4" rx="1" stroke="currentColor" strokeWidth="1.3" />
            <circle cx="12.5" cy="4" r="1" fill="currentColor" />
            <circle cx="12.5" cy="11" r="1" fill="currentColor" />
          </svg>
        </div>

        {/* Options icon — decorative, pushed right */}
        <div className="ml-auto flex items-center text-sidebar-foreground opacity-35">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="3" r="1.2" fill="currentColor" />
            <circle cx="8" cy="8" r="1.2" fill="currentColor" />
            <circle cx="8" cy="13" r="1.2" fill="currentColor" />
          </svg>
        </div>
      </div>

      <NavBar />
      <ArrowList />
    </div>
  )
}
