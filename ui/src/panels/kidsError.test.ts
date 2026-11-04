import { expect, test } from 'vitest'
import { kidsError } from './kidsError'

test('core_errors_become_plain_words_in_kids_mode', () => {
  expect(kidsError('Node 3 is not connected to ground.')).toMatch(/Close the loop/)
  expect(kidsError('The circuit cannot be solved: voltage sources in a loop')).toMatch(/cannot work/)
  expect(kidsError('The solver did not converge after 100 iterations.')).toMatch(/confused/)
  expect(kidsError('Invalid input: transient result would be too large; use a larger step')).toMatch(/too long/)
  expect(kidsError('something new')).toBe('something new')
})
