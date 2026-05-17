import { render, screen } from '@testing-library/react'
import { describe, it, expect, beforeEach } from 'vitest'
import { ArrowList } from './ArrowList'
import { useArrowStore } from '@/lib/core-store'
import type { ArrowEntry } from '@/domain/arrow'

const makeArrow = (name: string): ArrowEntry => ({
  namespace: `github.com/test/${name}`,
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
  useArrowStore.getState().resetArrows()
})

describe('ArrowList', () => {
  it('renders nothing when store is empty', () => {
    const { container } = render(<ArrowList />)
    expect(container.firstChild?.childNodes.length).toBe(0)
  })

  it('renders one ArrowItem per arrow in the store', () => {
    useArrowStore.getState().upsertArrow(makeArrow('alpha'))
    useArrowStore.getState().upsertArrow(makeArrow('beta'))
    render(<ArrowList />)
    expect(screen.getByText('alpha')).toBeInTheDocument()
    expect(screen.getByText('beta')).toBeInTheDocument()
  })
})
