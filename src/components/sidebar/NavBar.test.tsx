import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, beforeEach } from 'vitest'
import { NavBar } from './NavBar'
import { useUIStore } from '@/store/ui'
import { useArrowStore } from '@/lib/core-store'
import type { ArrowEntry } from '@/domain/arrow'

const makeArrow = (namespace: string, name: string): ArrowEntry => ({
  namespace,
  name,
  description: '',
  tags: [],
  icon: null,
  banner: null,
  version: '1.0.0',
  state: 'ready',
  active_run: null,
  last_return: null,
})

beforeEach(() => {
  useUIStore.setState({ navMode: 'home', selectedNamespace: null })
  useArrowStore.getState().resetArrows()
})

describe('NavBar', () => {
  it('shows "Home" label in home mode', () => {
    render(<NavBar />)
    expect(screen.getByText('Home')).toBeInTheDocument()
  })

  it('switches to search mode when bar is clicked', async () => {
    render(<NavBar />)
    await userEvent.click(screen.getByTestId('nav-bar'))
    expect(useUIStore.getState().navMode).toBe('search')
  })

  it('shows the selected arrow namespace in arrow mode', () => {
    useArrowStore.getState().upsertArrow(makeArrow('github.com/char2cs/quiver', 'quiver'))
    useUIStore.setState({ navMode: 'arrow', selectedNamespace: 'github.com/char2cs/quiver' })
    render(<NavBar />)
    expect(screen.getByText('quiver')).toBeInTheDocument()
  })

  it('calls goHome when home button is clicked', async () => {
    useUIStore.setState({ navMode: 'arrow', selectedNamespace: 'github.com/char2cs/quiver' })
    render(<NavBar />)
    await userEvent.click(screen.getByRole('button', { name: /home/i }))
    expect(useUIStore.getState().navMode).toBe('home')
    expect(useUIStore.getState().selectedNamespace).toBeNull()
  })

  it('exits search mode on Escape', async () => {
    render(<NavBar />)
    await userEvent.click(screen.getByTestId('nav-bar'))
    await userEvent.keyboard('{Escape}')
    expect(useUIStore.getState().navMode).not.toBe('search')
  })
})
