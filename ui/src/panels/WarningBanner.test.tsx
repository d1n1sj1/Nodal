// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, expect, test } from 'vitest'
import WarningBanner from './WarningBanner'

afterEach(cleanup)
const w = (id: string) => ({ part_id: id, kind: 'OverCurrent' as const, peak: 0.047, limit: 0.02 })

test('banner_lists_each_warning', () => {
  render(<WarningBanner warnings={[w('p1'), w('p2')]} mode="engineer" />)
  expect(screen.getAllByRole('listitem')).toHaveLength(2)
  expect(screen.getByRole('alert')).toBeTruthy()
})

test('no_warnings_renders_nothing', () => {
  const { container, rerender } = render(<WarningBanner warnings={[]} mode="kids" />)
  expect(container.innerHTML).toBe('')
  rerender(<WarningBanner mode="kids" />)
  expect(container.innerHTML).toBe('')
})
