import { describe, expect, it } from 'vitest'
import { parseSI } from './formatSI'

describe('parseSI', () => {
  it('reads SI suffixes, including Greek mu and uppercase K', () => {
    expect(parseSI('10m')).toBe(0.01)
    expect(parseSI('5u')).toBe(5e-6)
    expect(parseSI('5μ')).toBe(5e-6)
    expect(parseSI('5µ')).toBe(5e-6)
    expect(parseSI('2K')).toBe(2000)
    expect(parseSI('3M')).toBe(3e6)
  })
  it('rejects junk', () => {
    expect(parseSI('abc')).toBeNaN()
  })
})
