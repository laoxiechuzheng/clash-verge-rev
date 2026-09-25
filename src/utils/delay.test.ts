import { describe, expect, test } from 'vitest'

import {
  classifyDelay,
  compareByDelay,
  compareDelayPresentation,
  DEFAULT_DELAY_TIMEOUT,
  presentDelay,
} from './delay'

describe('delay semantics', () => {
  test('classifies measurements and core sentinel values', () => {
    expect([
      classifyDelay(120),
      classifyDelay(-2),
      classifyDelay(-1),
      classifyDelay(0),
      classifyDelay(DEFAULT_DELAY_TIMEOUT),
      classifyDelay(1e5 + 1),
      classifyDelay(Number.NaN),
    ]).toEqual([
      'measured',
      'testing',
      'untested',
      'timeout',
      'timeout',
      'error',
      'untested',
    ])
    expect(classifyDelay(3000, 5000)).toBe('measured')
    expect(classifyDelay(3000, 2000)).toBe('timeout')
  })

  test('sorts measurements first and sentinel states by meaning', () => {
    const delays = [-1, 0, -2, 1e5 + 1, 300, 100]

    expect(delays.sort(compareByDelay)).toEqual([100, 300, 0, 1e5 + 1, -2, -1])
  })

  test('keeps the comparator symmetric', () => {
    const direction = (value: number) => Math.sign(value)
    const samples = [150, 0, -1, -2, 1e5 + 1]

    for (const a of samples) {
      for (const b of samples) {
        expect(
          direction(compareByDelay(a, b)) + direction(compareByDelay(b, a)),
        ).toBe(0)
      }
    }
  })

  test('rounds display percentages without changing sentinel states', () => {
    expect([
      presentDelay(1, 40).display,
      presentDelay(2, 40).display,
      presentDelay(3, 40).display,
      presentDelay(4, 40).display,
      presentDelay(101, 40).display,
      presentDelay(102, 40).display,
      presentDelay(150, 200).display,
    ]).toEqual([1, 1, 1, 2, 40, 41, 300])

    for (const delay of [-2, -1, 0, DEFAULT_DELAY_TIMEOUT, 1e5 + 1, NaN]) {
      expect(presentDelay(delay, 40).display).toBe(delay)
      expect(presentDelay(delay, 200).display).toBe(delay)
    }
  })

  test('sorts measured presentations by displayed delay and preserves raw precedence', () => {
    const values = [
      presentDelay(60, 40),
      presentDelay(30, 200),
      presentDelay(0, 40),
    ]
    expect(
      values.sort(compareDelayPresentation).map(({ display }) => display),
    ).toEqual([24, 60, 0])
  })
})
