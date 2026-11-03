// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, test, vi } from 'vitest'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

import { addPart, addWire, emptySchematic, type Schematic } from '../editor/schematic'
import { kidsPalette } from '../parts/registry'
import AcPanel from './AcPanel'

const at = { x: 0, y: 0 }
function rc(): Schematic {
  let s = ['VSource', 'Resistor', 'Capacitor', 'Ground'].reduce((acc, k) => addPart(acc, k, at), emptySchematic())
  s = addWire(s, { part: 'p1', pin: 0 }, { part: 'p2', pin: 0 })
  s = addWire(s, { part: 'p2', pin: 1 }, { part: 'p3', pin: 0 })
  s = addWire(s, { part: 'p3', pin: 1 }, { part: 'p4', pin: 0 })
  return addWire(s, { part: 'p1', pin: 1 }, { part: 'p4', pin: 0 })
}

beforeEach(() => { invoke.mockReset() }) // a returned function would run as teardown

afterEach(cleanup)

test('sweep sends the chosen source and parsed frequencies, then draws the plots', async () => {
  invoke.mockResolvedValue({ freqs: [10, 100], magnitude: [[0, 1, 1], [0, 1, 0.5]], phase_deg: [[0, 0, -5], [0, 0, -45]] })
  render(<AcPanel schematic={rc()} />)
  fireEvent.change(screen.getByLabelText('Stop frequency'), { target: { value: '1k' } })
  fireEvent.click(screen.getByRole('button', { name: 'Sweep' }))
  await waitFor(() => expect(screen.getByRole('img', { name: 'Gain' })).toBeTruthy())
  const [cmd, args] = invoke.mock.calls[0]
  expect(cmd).toBe('run_ac')
  expect(args).toMatchObject({ source: 'p1', fStart: 10, fStop: 1000, pointsPerDecade: 20 })
})

test('bad frequencies and a missing source are explained without calling the core', async () => {
  render(<AcPanel schematic={rc()} />)
  fireEvent.change(screen.getByLabelText('Start frequency'), { target: { value: '500k' } })
  fireEvent.click(screen.getByRole('button', { name: 'Sweep' }))
  expect((await screen.findByRole('alert')).textContent).toMatch(/start below stop/)
  expect(invoke).not.toHaveBeenCalled()
  render(<AcPanel schematic={emptySchematic()} />)
  fireEvent.click(screen.getAllByRole('button', { name: 'Sweep' })[1])
  await waitFor(() => expect(screen.getAllByRole('alert')[1].textContent).toMatch(/voltage source/))
})

test('a core error is shown and clears the old plot', async () => {
  invoke.mockImplementation(() => Promise.reject(new Error('Invalid input: nope')))
  render(<AcPanel schematic={rc()} />)
  fireEvent.click(screen.getByRole('button', { name: 'Sweep' }))
  await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('nope'))
  expect(invoke).toHaveBeenCalledTimes(1)
})

test('kids sweep hides the numbers, picks the Wave tile and draws loudness', async () => {
  invoke.mockResolvedValue({ freqs: [10, 100], magnitude: [[0, 1, 1], [0, 1, 0.5]], phase_deg: [[0, 0, -5], [0, 0, -45]] })
  const kid = (id: string) => kidsPalette().find((d) => d.id === `kids-${id}`)!
  let s = addPart(addPart(addPart(emptySchematic(), kid('battery'), at), kid('wave'), at), kid('resistor'), at) // p1 battery, p2 wave, p3 resistor
  s = addWire(s, { part: 'p2', pin: 0 }, { part: 'p3', pin: 0 })
  s = addWire(s, { part: 'p3', pin: 1 }, { part: 'p2', pin: 1 })
  render(<AcPanel schematic={s} mode="kids" />)
  expect(screen.queryByLabelText('Start frequency')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: 'Sweep pitches' }))
  await waitFor(() => expect(screen.getByRole('img', { name: 'Loudness' })).toBeTruthy())
  expect(invoke.mock.calls[0][1]).toMatchObject({ source: 'p2', fStart: 10, fStop: 100000, pointsPerDecade: 20 })
  expect(screen.queryByRole('img', { name: 'Phase' })).toBeNull()
})
