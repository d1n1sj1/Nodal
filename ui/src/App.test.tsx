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
