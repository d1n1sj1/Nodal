// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, test, vi } from 'vitest'
import { kidsPalette, partDefs } from '../parts/registry'
import Palette from './Palette'

afterEach(cleanup)

test('engineer_shows_icon_and_name_for_each_part', () => {
  render(<Palette mode="engineer" onPick={() => {}} />)
  for (const d of partDefs) {
    const b = screen.getByRole('button', { name: d.name })
    expect(b.querySelector('svg')).toBeTruthy()
  }
})

test('kids_shows_tiles_with_names_and_pick_fires', () => {
  const onPick = vi.fn()
  render(<Palette mode="kids" onPick={onPick} />)
  for (const d of kidsPalette()) {
    expect(screen.getByRole('button', { name: d.name }).querySelector('svg')).toBeTruthy()
  }
  fireEvent.click(screen.getByRole('button', { name: 'Battery' }))
  expect(onPick.mock.calls[0][0].id).toBe('kids-battery')
})
