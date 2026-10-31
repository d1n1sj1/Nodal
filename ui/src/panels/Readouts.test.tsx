// @vitest-environment jsdom
import { cleanup, render } from '@testing-library/react'
import { afterEach, expect, test } from 'vitest'
import { formatSI } from './formatSI'
import Scope from './Scope'
import type { SimOutput } from '../sim/api'

afterEach(cleanup)

test('formatSI_examples', () => {
  expect(formatSI(3.2, 'V')).toBe('3.20 V')
  expect(formatSI(0.0018, 'A')).toBe('1.80 mA')
  expect(formatSI(4.2e-3, 's')).toBe('4.20 ms')
})

test('formatSI_edge_cases', () => {
  expect(formatSI(0, 'V')).toBe('0.00 V')
  expect(formatSI(-0.0018, 'A')).toBe('-1.80 mA')
  expect(formatSI(2.5e-6, 's')).toBe('2.50 µs')
  expect(formatSI(4700, 'Ω')).toBe('4.70 kΩ')
  expect(formatSI(999.6, 'V')).toBe('1.00 kV')
  expect(formatSI(1e-15, 'A')).toBe('0.00 pA')
  expect(formatSI(1e15, 'V')).toBe('1000000 GV')
  expect(formatSI(NaN, 'V')).toBe('—')
  expect(formatSI(Infinity, 'V')).toBe('—')
})

test('formatSI_rounding_boundaries', () => {
  expect(formatSI(99.96, 'V')).toBe('100 V')
  expect(formatSI(9.996, 'V')).toBe('10.0 V')
  expect(formatSI(-1e-15, 'A')).toBe('0.00 pA')
})

test('scope_handles_300k_points', () => {
  const { container } = render(<Scope output={mk(300000, (i) => i % 7)} selectedNode={1} />)
  expect(container.querySelector('polyline')).toBeTruthy()
})

const mk = (n: number, v: (i: number) => number): SimOutput => ({
  times: Array.from({ length: n }, (_, i) => i * 1e-3),
  node_voltages: Array.from({ length: n }, (_, i) => [0, v(i)]),
  part_currents: {},
  warnings: [],
})

test.each([
  ['flat', mk(5, () => 3)],
  ['empty', mk(0, () => 0)],
  ['single', mk(1, () => 2)],
])('scope_renders_without_NaN_%s', (_n, out) => {
  const { container } = render(<Scope output={out} selectedNode={1} />)
  expect(container.querySelector('svg')).toBeTruthy()
  expect(container.innerHTML).not.toMatch(/NaN|Infinity/)
})
