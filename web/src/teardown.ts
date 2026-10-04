/**
 * Runs every step of a teardown in order, also when one fails, so a part
 * that would not let go cannot keep the parts after it alive. Throws the
 * first fault once every step ran.
 */
export function tearDown(steps: Array<() => void>): void {
  const faults: unknown[] = [];
  for (const step of steps) {
    try {
      step();
    } catch (fault) {
      faults.push(fault);
    }
  }
  if (faults.length > 0) throw faults[0];
}
