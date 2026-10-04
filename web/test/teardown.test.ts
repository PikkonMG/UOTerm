import { describe, expect, it, vi } from 'vitest';
import { tearDown } from '../src/teardown';

describe('tearDown', () => {
  it('runs_every_step_when_one_throws_then_throws_the_first_fault', () => {
    const fault = new Error('the renderer would not go');
    const disposeRenderer = vi.fn(() => {
      throw fault;
    });
    const freeView = vi.fn();
    expect(() => tearDown([vi.fn(), disposeRenderer, freeView])).toThrow(fault);
    expect(freeView).toHaveBeenCalledOnce();
  });

  it('runs_each_step_in_order', () => {
    const order: number[] = [];
    tearDown([() => order.push(1), () => order.push(2), () => order.push(3)]);
    expect(order).toEqual([1, 2, 3]);
  });
});
