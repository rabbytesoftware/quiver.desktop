import { useRef, useEffect } from 'react'
import { useUIStore } from '@/store/ui'
import { useArrowStore } from '@/lib/core-store'

export function NavBar() {
  const navMode = useUIStore((s) => s.navMode)
  const selectedNamespace = useUIStore((s) => s.selectedNamespace)
  const setNavMode = useUIStore((s) => s.setNavMode)
  const goHome = useUIStore((s) => s.goHome)
  const arrows = useArrowStore((s) => s.arrows)

  const inputRef = useRef<HTMLInputElement>(null)
  const isSearch = navMode === 'search'

  const contextLabel =
    navMode === 'arrow' && selectedNamespace
      ? (arrows.get(selectedNamespace)?.name ?? selectedNamespace)
      : 'Home'

  useEffect(() => {
    if (isSearch) inputRef.current?.focus()
  }, [isSearch])

  const activateSearch = () => {
    setNavMode('search')
  }

  const deactivateSearch = () => {
    if (inputRef.current) inputRef.current.value = ''
    setNavMode(selectedNamespace ? 'arrow' : 'home')
  }

  return (
    <div className="flex flex-shrink-0 items-center border-y border-white/[0.06]">
      {/* Home button */}
      <button
        aria-label="Home"
        className={`flex h-[30px] w-7 flex-shrink-0 cursor-pointer items-center justify-center border-r border-white/[0.06] transition-colors ${
          navMode === 'home'
            ? 'bg-white/[0.08] opacity-100'
            : 'opacity-40 hover:bg-white/[0.06] hover:opacity-75'
        }`}
        onMouseDown={(e) => {
          // Prevent blur on the search input from firing before click
          e.preventDefault()
          goHome()
        }}
      >
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
          <path
            d="M2 7L8 2L14 7V14H10V10H6V14H2V7Z"
            stroke="white"
            strokeWidth="1.4"
            strokeLinejoin="round"
          />
        </svg>
      </button>

      {/* Bar */}
      <div
        data-testid="nav-bar"
        className={`flex h-[30px] flex-1 cursor-text items-center gap-1.5 px-2 transition-colors ${
          isSearch ? 'bg-white/95' : ''
        }`}
        onClick={activateSearch}
      >
        {!isSearch && (
          <span className="flex-1 select-none truncate text-[11px] text-white/50">
            {contextLabel}
          </span>
        )}
        <input
          ref={inputRef}
          aria-hidden={!isSearch}
          className={`min-w-0 flex-1 border-none bg-transparent text-[11px] text-black/80 outline-none placeholder-black/35 ${
            isSearch ? 'block' : 'hidden'
          }`}
          placeholder="Search anything"
          onBlur={deactivateSearch}
          onKeyDown={(e) => {
            if (e.key === 'Escape') deactivateSearch()
          }}
        />
        {/* Search icon — decorative */}
        <div className={`flex flex-shrink-0 items-center ${isSearch ? 'opacity-50' : 'opacity-38'}`}>
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
            <circle
              cx="6.5"
              cy="6.5"
              r="4.5"
              stroke={isSearch ? 'black' : 'white'}
              strokeWidth="1.4"
            />
            <path
              d="M10 10L14 14"
              stroke={isSearch ? 'black' : 'white'}
              strokeWidth="1.4"
              strokeLinecap="round"
            />
          </svg>
        </div>
      </div>
    </div>
  )
}
