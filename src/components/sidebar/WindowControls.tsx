import { getCurrentWindow } from '@tauri-apps/api/window'
import { useState } from 'react'

export function WindowControls() {
  const [hovered, setHovered] = useState(false)

  const win = getCurrentWindow()
  const close = () => win.close()
  const minimize = () => win.minimize()
  const fullscreen = async () => {
    const isFull = await win.isFullscreen()
    win.setFullscreen(!isFull)
  }

  return (
    <div
      className="flex items-center gap-[6px] px-[10px]"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <button
        aria-label="Close"
        onClick={close}
        className="flex h-3 w-3 cursor-default items-center justify-center rounded-full bg-[#FF5F57] hover:bg-[#FF5F57] active:bg-[#BF4743]"
      >
        {hovered && (
          <svg width="6" height="6" viewBox="0 0 10 10" fill="none" className="opacity-70">
            <path d="M1 1L9 9M9 1L1 9" stroke="black" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        )}
      </button>

      <button
        aria-label="Minimize"
        onClick={minimize}
        className="flex h-3 w-3 cursor-default items-center justify-center rounded-full bg-[#FFBD2E] hover:bg-[#FFBD2E] active:bg-[#BF9122]"
      >
        {hovered && (
          <svg width="6" height="2" viewBox="0 0 8 2" fill="none" className="opacity-70">
            <path d="M1 1H7" stroke="black" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        )}
      </button>

      <button
        aria-label="Fullscreen"
        onClick={fullscreen}
        className="flex h-3 w-3 cursor-default items-center justify-center rounded-full bg-[#28C840] hover:bg-[#28C840] active:bg-[#1E9930]"
      >
        {hovered && (
          <svg width="6" height="6" viewBox="0 0 10 10" fill="none" className="opacity-70">
            <path d="M1 9L9 1M1 1L1 5M1 1L5 1" stroke="black" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M9 1L9 5M9 9L5 9" stroke="black" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        )}
      </button>
    </div>
  )
}
