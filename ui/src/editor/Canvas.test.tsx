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
  expect(screen.getByRole('button', { name: 'Resistor' })).toBeTruthy() // kids tile name
  fireEvent.mouseDown(screen.getByRole('button', { name: 'Resistor' }))
  fireEvent.keyDown(screen.getByRole('application'), { key: 'Delete' })
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
