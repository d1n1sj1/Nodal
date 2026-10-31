// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, test, vi } from 'vitest'
import { partDefs } from '../parts/registry'
import Canvas from './Canvas'
import { addPart, emptySchematic } from './schematic'

afterEach(cleanup)

test('clicking_canvas_with_a_picked_part_places_it', () => {
  const onChange = vi.fn()
  render(<Canvas schematic={emptySchematic()} onChange={onChange} mode="engineer" placing={partDefs[0]} />)
  fireEvent.click(screen.getByRole('application'), { clientX: 105, clientY: 58 })
  expect(onChange.mock.calls[0][0].parts).toHaveLength(1)
  expect(onChange.mock.calls[0][0].parts[0].pos).toEqual({ x: 100, y: 60 }) // snapped to grid
})

test('delete_removes_the_selected_part', () => {
  const onChange = vi.fn()
  const s = addPart(emptySchematic(), 'Resistor', { x: 100, y: 100 })
  render(<Canvas schematic={s} onChange={onChange} mode="kids" />)
  const body = screen.getByRole('button', { name: 'Select Resistor p1' }) // kids tile name
  fireEvent.keyDown(body, { key: 'Enter' })
  const svg = screen.getByRole('application')
  svg.focus()
  expect(document.activeElement).toBe(svg)
  fireEvent.keyDown(svg, { key: 'Delete' })
  expect(onChange.mock.calls[0][0].parts).toHaveLength(0)
})

test('clicking_two_pins_adds_a_wire', () => {
  const onChange = vi.fn()
  let s = addPart(addPart(emptySchematic(), 'Resistor', { x: 100, y: 100 }), 'Resistor', { x: 300, y: 100 })
  const { rerender } = render(<Canvas schematic={s} onChange={onChange} mode="engineer" />)
  fireEvent.click(screen.getByRole('button', { name: 'p1 pin 2' }))
  fireEvent.click(screen.getByRole('button', { name: 'p2 pin 1' }))
  s = onChange.mock.calls[0][0]
  rerender(<Canvas schematic={s} onChange={onChange} mode="engineer" />)
  expect(s.wires).toHaveLength(1)
  expect(document.querySelectorAll('line.wire')).toHaveLength(1)
})

test('keyboard_wires_two_pins', () => {
  const onChange = vi.fn()
  const s = addPart(addPart(emptySchematic(), 'Resistor', { x: 100, y: 100 }), 'Resistor', { x: 300, y: 100 })
  render(<Canvas schematic={s} onChange={onChange} mode="engineer" />)
  const a = screen.getByRole('button', { name: 'p1 pin 2' })
  a.focus()
  expect(document.activeElement).toBe(a)
  fireEvent.keyDown(a, { key: 'Enter' })
  fireEvent.keyDown(screen.getByRole('button', { name: 'p2 pin 1' }), { key: ' ' })
  expect(onChange.mock.calls[0][0].wires).toHaveLength(1)
})

test('keyboard_places_armed_part_in_next_free_cell', () => {
  const onChange = vi.fn()
  const s = addPart(emptySchematic(), 'Resistor', { x: 100, y: 100 })
  render(<Canvas schematic={s} onChange={onChange} mode="engineer" placing={partDefs[0]} />)
  fireEvent.keyDown(screen.getByRole('application'), { key: 'Enter' })
  expect(onChange.mock.calls[0][0].parts[1].pos).toEqual({ x: 240, y: 100 })
})

test('keyboard_placement_avoids_off_grid_overlap', () => {
  const onChange = vi.fn()
  const s = addPart(emptySchematic(), 'Resistor', { x: 130, y: 90 }) // loaded off-grid, overlaps cell (100,100)
  render(<Canvas schematic={s} onChange={onChange} mode="engineer" placing={partDefs[0]} />)
  fireEvent.keyDown(screen.getByRole('application'), { key: ' ' })
  const { x, y } = onChange.mock.calls[0][0].parts[1].pos
  expect(Math.abs(x - 130) >= 68 || Math.abs(y - 90) >= 44).toBe(true)
})

test('space_on_unarmed_svg_is_default_prevented', () => {
  render(<Canvas schematic={emptySchematic()} onChange={() => {}} mode="engineer" />)
  expect(fireEvent.keyDown(screen.getByRole('application'), { key: ' ' })).toBe(false)
})

test('warned_part_gets_damaged_marker_and_label', () => {
  const s = addPart(emptySchematic(), 'Resistor', { x: 100, y: 100 })
  render(<Canvas schematic={s} onChange={vi.fn()} mode="engineer" damaged={new Set(['p1'])} />)
  expect(screen.getByRole('group', { name: /p1 — warning/ }).getAttribute('class')).toContain('damaged')
})
