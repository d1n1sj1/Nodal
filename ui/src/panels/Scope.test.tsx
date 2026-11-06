// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, expect, test } from 'vitest'
import Scope from './Scope'

afterEach(cleanup)

const run = (n: number, spikeAt: number) => ({
  times: Array.from({ length: n }, (_, i) => i * 1e-6),
  node_voltages: Array.from({ length: n }, (_, i) => [0, i === spikeAt ? 5 : 0]),
  part_currents: {},
  warnings: [],
})

test('a million-step run draws a bounded line that still shows a one-sample spike', () => {
  render(<Scope output={run(200_000, 123_457)} selectedNode={1} />)
  const pts = screen.getByRole('img', { name: 'Scope' }).querySelector('polyline')!.getAttribute('points')!.split(' ')
  expect(pts.length).toBeLessThanOrEqual(2000)
  const ys = pts.map((p) => Number(p.split(',')[1]))
  expect(Math.min(...ys)).toBeLessThan(Math.max(...ys) - 100) // the 5 V sample was kept
})

test('a short run draws every sample', () => {
  render(<Scope output={run(50, -1)} selectedNode={1} />)
  expect(screen.getByRole('img', { name: 'Scope' }).querySelector('polyline')!.getAttribute('points')!.split(' ')).toHaveLength(50)
})
