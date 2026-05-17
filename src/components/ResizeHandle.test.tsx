import { render, fireEvent } from '@testing-library/react'
import { describe, it, expect, beforeEach } from 'vitest'
import { ResizeHandle } from './ResizeHandle'
import { useUIStore } from '@/store/ui'

beforeEach(() => {
  useUIStore.setState({ sidebarWidth: 200 })
})

describe('ResizeHandle', () => {
  it('increases sidebarWidth when dragged right', () => {
    const { container } = render(<ResizeHandle />)
    const handle = container.firstChild as HTMLElement

    fireEvent.mouseDown(handle, { clientX: 200 })
    fireEvent.mouseMove(window, { clientX: 250 })
    fireEvent.mouseUp(window)

    expect(useUIStore.getState().sidebarWidth).toBe(250)
  })

  it('decreases sidebarWidth when dragged left', () => {
    const { container } = render(<ResizeHandle />)
    const handle = container.firstChild as HTMLElement

    fireEvent.mouseDown(handle, { clientX: 200 })
    fireEvent.mouseMove(window, { clientX: 150 })
    fireEvent.mouseUp(window)

    expect(useUIStore.getState().sidebarWidth).toBe(150)
  })

  it('does not go below minimum 120', () => {
    const { container } = render(<ResizeHandle />)
    const handle = container.firstChild as HTMLElement

    fireEvent.mouseDown(handle, { clientX: 200 })
    fireEvent.mouseMove(window, { clientX: 0 })
    fireEvent.mouseUp(window)

    expect(useUIStore.getState().sidebarWidth).toBe(120)
  })

  it('does not exceed maximum 320', () => {
    const { container } = render(<ResizeHandle />)
    const handle = container.firstChild as HTMLElement

    fireEvent.mouseDown(handle, { clientX: 200 })
    fireEvent.mouseMove(window, { clientX: 600 })
    fireEvent.mouseUp(window)

    expect(useUIStore.getState().sidebarWidth).toBe(320)
  })
})
