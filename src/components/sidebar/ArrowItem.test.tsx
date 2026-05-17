import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, beforeEach } from 'vitest'
import { ArrowItem } from './ArrowItem'
import { useUIStore } from '@/store/ui'
import type { ArrowEntry } from '@/domain/arrow'

const mockArrow: ArrowEntry = {
  namespace: 'github.com/char2cs/quiver',
  name: 'quiver',
  description: '',
  tags: [],
  icon: null,
  banner: null,
  version: '1.0.0',
  state: 'ready',
  active_run: null,
  last_return: null,
}

beforeEach(() => {
  useUIStore.setState({ selectedNamespace: null, navMode: 'home' })
})

describe('ArrowItem', () => {
  it('renders the arrow name', () => {
    render(<ArrowItem arrow={mockArrow} />)
    expect(screen.getByText('quiver')).toBeInTheDocument()
  })

  it('calls selectArrow with the namespace on click', async () => {
    render(<ArrowItem arrow={mockArrow} />)
    await userEvent.click(screen.getByText('quiver'))
    expect(useUIStore.getState().selectedNamespace).toBe('github.com/char2cs/quiver')
  })

  it('shows namespace when selected', () => {
    useUIStore.setState({ selectedNamespace: 'github.com/char2cs/quiver', navMode: 'arrow' })
    render(<ArrowItem arrow={mockArrow} />)
    expect(screen.getByText('github.com/char2cs/quiver')).toBeInTheDocument()
  })

  it('hides namespace when not selected and not hovered', () => {
    render(<ArrowItem arrow={mockArrow} />)
    expect(screen.getByText('github.com/char2cs/quiver').className).toContain('hidden')
  })
})
