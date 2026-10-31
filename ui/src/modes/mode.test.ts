// @vitest-environment jsdom
import { act, renderHook } from '@testing-library/react'
import { beforeEach, expect, test, vi } from 'vitest'
import { buildCircuit } from '../editor/buildCircuit'
import { addPart, emptySchematic } from '../editor/schematic'

beforeEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-mode')
  vi.resetModules()
})

test('defaults_to_engineer', async () => {
  const { useMode } = await import('./mode')
  expect(renderHook(() => useMode()).result.current.mode).toBe('engineer')
})

test('invalid_stored_value_falls_back', async () => {
  localStorage.setItem('nodal.mode', 'banana')
  const { useMode } = await import('./mode')
  expect(renderHook(() => useMode()).result.current.mode).toBe('engineer')
})

test('set_mode_persists_and_sets_data_mode_attribute', async () => {
  const { useMode } = await import('./mode')
  const { result } = renderHook(() => useMode())
  act(() => result.current.setMode('kids'))
  expect(result.current.mode).toBe('kids')
  expect(localStorage.getItem('nodal.mode')).toBe('kids')
  expect(document.documentElement.getAttribute('data-mode')).toBe('kids')
})

test('mode_is_not_in_saved_circuit_json', () => {
  const s = addPart(emptySchematic(), 'Resistor', { x: 0, y: 0 })
  expect(JSON.stringify(buildCircuit(s))).not.toContain('"mode"')
})
