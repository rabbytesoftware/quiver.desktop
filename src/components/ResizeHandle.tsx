import { useRef } from 'react'
import { useUIStore } from '@/store/ui'

export function ResizeHandle() {
  const setSidebarWidth = useUIStore((s) => s.setSidebarWidth)
  const sidebarWidth = useUIStore((s) => s.sidebarWidth)
  const startX = useRef(0)
  const startWidth = useRef(0)

  const onMouseDown = (e: React.MouseEvent) => {
    startX.current = e.clientX
    startWidth.current = sidebarWidth

    const onMouseMove = (e: MouseEvent) => {
      setSidebarWidth(startWidth.current + (e.clientX - startX.current))
    }

    const onMouseUp = () => {
      window.removeEventListener('mousemove', onMouseMove)
      window.removeEventListener('mouseup', onMouseUp)
    }

    window.addEventListener('mousemove', onMouseMove)
    window.addEventListener('mouseup', onMouseUp)
  }

  return (
    <div
      className="w-[3px] flex-shrink-0 cursor-col-resize bg-sidebar-border transition-colors hover:bg-sidebar-foreground/20"
      onMouseDown={onMouseDown}
    />
  )
}
