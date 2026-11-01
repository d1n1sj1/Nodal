// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, test } from 'vitest'
import App from './App'

afterEach(() => {
  cleanup()
  localStorage.clear()
})

test('kids_header_shows_kids_tag_engineer_does_not', () => {
  render(<App />)
  expect(screen.queryByText('Kids', { selector: '.kids-tag' })).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: 'Kids' }))
  expect(screen.getByText('Kids', { selector: '.kids-tag' })).toBeTruthy()
  expect(document.querySelector('.app')?.getAttribute('data-mode')).toBe('kids')
})

test('selecting_a_part_shows_its_properties', () => {
  render(<App />)
  fireEvent.click(screen.getByRole('button', { name: 'Engineer' })) // mode lives in module state across tests
  fireEvent.click(screen.getByRole('button', { name: 'Resistor' }))
  fireEvent.click(screen.getByRole('application'))
  expect(screen.queryByLabelText('Resistance (Ω)')).toBeNull()
  fireEvent.keyDown(screen.getByRole('button', { name: 'Select Resistor p1' }), { key: 'Enter' })
  expect((screen.getByLabelText('Resistance (Ω)') as HTMLInputElement).value).toBe('1.00 k')
})

test('undo_button_removes_the_last_placed_part', () => {
  render(<App />)
  fireEvent.click(screen.getByRole('button', { name: 'Engineer' }))
  expect((screen.getByRole('button', { name: 'Undo' }) as HTMLButtonElement).disabled).toBe(true)
  fireEvent.click(screen.getByRole('button', { name: 'Resistor' }))
  fireEvent.click(screen.getByRole('application'))
  expect(screen.getByRole('button', { name: 'Select Resistor p1' })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Undo' }))
  expect(screen.queryByRole('button', { name: 'Select Resistor p1' })).toBeNull()
  fireEvent.keyDown(document.body, { key: 'y', ctrlKey: true })
  expect(screen.getByRole('button', { name: 'Select Resistor p1' })).toBeTruthy()
})
