// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, test } from 'vitest'
import { buildCircuit } from '../editor/buildCircuit'
import { addPart, emptySchematic, updateValues, type Schematic } from '../editor/schematic'
import type { Mode } from '../modes/mode'
import { kidsPalette, partDefs, type PartDef } from '../parts/registry'
import Properties from './Properties'

afterEach(cleanup)

/** Renders the panel for p1 and returns a getter for the edited schematic. */
function setup(what: string | PartDef, mode: Mode) {
  let s: Schematic = addPart(emptySchematic(), what, { x: 0, y: 0 })
  render(<Properties part={s.parts[0]} mode={mode} onChange={(v) => { s = updateValues(s, 'p1', v) }} />)
  return () => buildCircuit(s).parts[0] as unknown as Record<string, unknown>
}
const kid = (id: string) => kidsPalette().find((d) => d.id === `kids-${id}`)!

test('changing_ohms_changes_build_circuit_output', () => {
  const built = setup('Resistor', 'engineer')
  fireEvent.change(screen.getByLabelText('Resistance (Ω)'), { target: { value: '2.2k' } })
  expect(built().ohms).toBe(2200)
  expect(screen.queryByRole('alert')).toBeNull()
})

test.each(['abc', '0', '-5', '1e999'])('bad_value_%s_shows_message_and_keeps_old_value', (text) => {
  const built = setup('Resistor', 'engineer')
  fireEvent.change(screen.getByLabelText('Resistance (Ω)'), { target: { value: text } })
  expect(screen.getByRole('alert').textContent).toMatch(/above zero/)
  expect(built().ohms).toBe(1000)
})

test('volts_may_be_negative_or_zero', () => {
  const built = setup('VSource', 'engineer')
  fireEvent.change(screen.getByLabelText('Voltage (V)'), { target: { value: '-3' } })
  expect(built().volts).toBe(-3)
  fireEvent.change(screen.getByLabelText('Voltage (V)'), { target: { value: '0' } })
  expect(built().volts).toBe(0)
  expect(screen.queryByRole('alert')).toBeNull()
})

test('kids_battery_has_friendly_label', () => {
  const built = setup(kid('battery'), 'kids')
  fireEvent.change(screen.getByLabelText('Battery volts'), { target: { value: '9' } })
  expect(built().volts).toBe(9)
  fireEvent.change(screen.getByLabelText('Battery volts'), { target: { value: 'lots' } })
  expect(screen.getByRole('alert').textContent).toMatch(/number/)
})

test('kids_switch_is_toggle_only_and_open_gives_closed_false', () => {
  const built = setup(kid('switch'), 'kids')
  expect(screen.queryByRole('textbox')).toBeNull()
  const open = screen.getByRole('button', { name: 'Open' })
  expect(screen.getByRole('button', { name: 'Closed' }).getAttribute('aria-pressed')).toBe('true')
  fireEvent.click(open)
  expect(built().closed).toBe(false)
})

test('duty_cycle_shows_only_for_square_and_must_be_below_one', () => {
  const signal = partDefs.find((d) => d.id === 'signal')!
  setup(signal, 'engineer')
  expect(screen.queryByLabelText(/Duty/)).toBeNull()
  cleanup()
  const built = setup({ ...signal, defaults: { ...signal.defaults, square: 1 } }, 'engineer')
  fireEvent.change(screen.getByLabelText(/Duty/), { target: { value: '0.25' } })
  expect(built().wave).toMatchObject({ shape: 'Square', duty: 0.25 })
  fireEvent.change(screen.getByLabelText(/Duty/), { target: { value: '1' } })
  expect(screen.getByRole('alert')).toBeTruthy()
})
