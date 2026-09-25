/** Normalizes latency values and the core's non-measurement sentinels. */

export const DEFAULT_DELAY_TIMEOUT = 10000

const TESTING = -2

const IMPLAUSIBLE_DELAY = 1e5

export type DelayState =
  | 'testing'
  | 'untested'
  | 'error'
  | 'timeout'
  | 'measured'

export interface DelayPresentation {
  raw: number
  display: number
  percent: number
}

/** Applies a display-only multiplier while preserving the raw state used for classification. */
export const presentDelay = (
  delay: number,
  percent = 100,
  timeout: number = DEFAULT_DELAY_TIMEOUT,
): DelayPresentation => {
  const displayPercent = Number.isFinite(percent) && percent > 0 ? percent : 100
  const display =
    classifyDelay(delay, timeout) === 'measured'
      ? Math.max(1, Math.round((delay * displayPercent) / 100))
      : delay
  return { raw: delay, display, percent: displayPercent }
}

export const classifyDelay = (
  delay: number,
  timeout: number = DEFAULT_DELAY_TIMEOUT,
): DelayState => {
  if (!Number.isFinite(delay)) return 'untested'
  if (delay === TESTING) return 'testing'
  if (delay < 0) return 'untested'
  if (delay > IMPLAUSIBLE_DELAY) return 'error'
  if (delay === 0 || delay >= timeout) return 'timeout'
  return 'measured'
}

/** Rank separately so sentinel magnitudes cannot outrank real measurements. */
const rankOf = (state: DelayState): number => {
  switch (state) {
    case 'measured':
      return 0
    case 'timeout':
      return 1
    case 'error':
      return 2
    case 'testing':
      return 3
    case 'untested':
      return 4
  }
}

export const compareByDelay = (
  a: number,
  b: number,
  timeout: number = DEFAULT_DELAY_TIMEOUT,
): number => {
  const [aState, bState] = [
    classifyDelay(a, timeout),
    classifyDelay(b, timeout),
  ]
  const rankDifference = rankOf(aState) - rankOf(bState)
  if (rankDifference !== 0) return rankDifference

  if (aState !== 'measured') return 0
  return a - b
}

/** Sorts by the raw state, then by the value users actually see. */
export const compareDelayPresentation = (
  a: DelayPresentation,
  b: DelayPresentation,
  timeout: number = DEFAULT_DELAY_TIMEOUT,
): number => {
  const [aState, bState] = [
    classifyDelay(a.raw, timeout),
    classifyDelay(b.raw, timeout),
  ]
  const rankDifference = rankOf(aState) - rankOf(bState)
  if (rankDifference !== 0) return rankDifference
  if (aState !== 'measured') return 0
  return a.display - b.display
}
