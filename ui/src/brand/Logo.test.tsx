// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, expect, test } from 'vitest'
import Logo from './Logo'

afterEach(cleanup)

test('logo_renders_name_and_svg', () => {
  const { container } = render(<Logo />)
  expect(screen.getByText('Nodal')).toBeTruthy()
  expect(container.querySelector('svg')).not.toBeNull()
})
