// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, expect, test } from 'vitest'
import type { AcOutput } from '../sim/api'
import Bode, { toDb } from './Bode'

const out: AcOutput = {
  freqs: [10, 100, 1000],
  magnitude: [[0, 1, 1], [0, 1, 0.5], [0, 1, 0.1]],
  phase_deg: [[0, 0, -5], [0, 0, -45], [0, 0, -84]],
}

afterEach(cleanup)

test('toDb converts gain and floors zero', () => {
  expect(toDb(1)).toBeCloseTo(0)
  expect(toDb(0.1)).toBeCloseTo(-20)
  expect(Number.isFinite(toDb(0))).toBe(true)
})

test('bode draws a gain and a phase plot with decade ticks', () => {
  const { container } = render(<Bode output={out} node={2} />)
  expect(screen.getByRole('img', { name: 'Gain' })).toBeTruthy()
  expect(screen.getByRole('img', { name: 'Phase' })).toBeTruthy()
  expect(container.querySelectorAll('polyline')).toHaveLength(2)
  expect(container.querySelectorAll('.bode-grid')).toHaveLength(6) // 10, 100, 1k on each plot
  expect(container.textContent).toContain('1.00 kHz')
  expect(container.textContent).not.toContain('-0')
})

test('a single point draws nothing', () => {
  const { container } = render(<Bode output={{ freqs: [10], magnitude: [[0, 1]], phase_deg: [[0, 0]] }} node={1} />)
  expect(container.querySelector('svg')).toBeNull()
})
