// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, test, vi } from 'vitest'
import RunBar from './RunBar'
import { addPart, emptySchematic } from '../editor/schematic'

// Plain wrapper, not a rejecting vi.fn: vitest 5 fails the test on a mock that returns a rejected promise.
const impl = vi.hoisted(() => ({ fn: (..._a: unknown[]): Promise<unknown> => Promise.resolve() }))
const runSimulation = vi.hoisted(() => vi.fn((...a: unknown[]) => impl.fn(...a)))
vi.mock('../sim/api', () => ({ runSimulation }))

beforeEach(() => { runSimulation.mockClear(); impl.fn = () => Promise.resolve() })
afterEach(cleanup)

const withPart = () => addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 })

test('run_on_empty_schematic_shows_nothing_to_simulate_message_and_does_not_call_backend', () => {
  render(<RunBar schematic={emptySchematic()} onResult={vi.fn()} mode="engineer" />)
  fireEvent.click(screen.getByRole('button', { name: 'Run' }))
  expect(screen.getByRole('alert').textContent).toMatch(/nothing to simulate/i)
  expect(runSimulation).not.toHaveBeenCalled()
})

test('run_with_error_shows_message_text', async () => {
  impl.fn = async () => { throw 'floating node at p1' }
  render(<RunBar schematic={withPart()} onResult={vi.fn()} mode="engineer" />)
  fireEvent.click(screen.getByRole('button', { name: 'Run' }))
  expect((await screen.findByRole('alert')).textContent).toContain('floating node at p1')
})

test('run_passes_defaults_and_result', async () => {
  const out = { times: [], node_voltages: [], part_currents: {}, warnings: [] }
  impl.fn = async () => out
  const onResult = vi.fn()
  render(<RunBar schematic={withPart()} onResult={onResult} mode="engineer" />)
  fireEvent.click(screen.getByRole('button', { name: 'Run' }))
  await waitFor(() => expect(onResult).toHaveBeenCalledWith(out))
  expect(runSimulation.mock.calls[0].slice(1)).toEqual([0.01, 1e-5])
})

test('invalid_input_shows_message_and_does_not_call', () => {
  render(<RunBar schematic={withPart()} onResult={vi.fn()} mode="engineer" />)
  fireEvent.change(screen.getByLabelText('Stop time'), { target: { value: 'abc' } })
  fireEvent.click(screen.getByRole('button', { name: 'Run' }))
  expect(screen.getByRole('alert')).toBeTruthy()
  expect(runSimulation).not.toHaveBeenCalled()
})

test('si_suffix_input_parsed', async () => {
  impl.fn = async () => ({ times: [], node_voltages: [], part_currents: {}, warnings: [] })
  render(<RunBar schematic={withPart()} onResult={vi.fn()} mode="engineer" />)
  fireEvent.change(screen.getByLabelText('Stop time'), { target: { value: '2m' } })
  fireEvent.change(screen.getByLabelText('Step'), { target: { value: '5u' } })
  fireEvent.click(screen.getByRole('button', { name: 'Run' }))
  await waitFor(() => expect(runSimulation).toHaveBeenCalled())
  expect(runSimulation.mock.calls[0].slice(1)).toEqual([0.002, 5e-6])
})

test('kids_label_is_play', () => {
  render(<RunBar schematic={emptySchematic()} onResult={vi.fn()} mode="kids" />)
  expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy()
})

test.each([['1e999', '10u'], ['10m', '1e999'], ['1', '1p']])('bad_bounds_%s_%s_show_message_and_do_not_call', (stop, step) => {
  render(<RunBar schematic={withPart()} onResult={vi.fn()} mode="engineer" />)
  fireEvent.change(screen.getByLabelText('Stop time'), { target: { value: stop } })
  fireEvent.change(screen.getByLabelText('Step'), { target: { value: step } })
  fireEvent.click(screen.getByRole('button', { name: 'Run' }))
  expect(screen.getByRole('alert')).toBeTruthy()
  expect(runSimulation).not.toHaveBeenCalled()
})
