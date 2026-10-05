import { afterEach, describe, expect, it, vi } from 'vitest';
import { drawFrame, followSize, startFrames, type DrawnFrame } from '../src/frame_loop';
import { setPointScale } from '../src/points';

/** The animation frames asked for, run by hand. */
function fakeFrames() {
  const waiting = new Map<number, FrameRequestCallback>();
  let next = 0;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    next += 1;
    waiting.set(next, callback);
    return next;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => waiting.delete(id));
  return {
    waiting,
    runAt(ms: number) {
      const callbacks = [...waiting.values()];
      waiting.clear();
      for (const callback of callbacks) callback(ms);
    },
  };
}

afterEach(() => vi.unstubAllGlobals());

describe('startFrames', () => {
  it('runs_a_frame_when_its_interval_has_passed', () => {
    const frames = fakeFrames();
    const frame = vi.fn();
    startFrames({ intervalMs: () => 100, frame, fault: vi.fn() });
    frames.runAt(0);
    frames.runAt(50);
    frames.runAt(99);
    expect(frame).toHaveBeenCalledTimes(2);
  });

  it('stops_and_reports_a_frame_that_fails', () => {
    const frames = fakeFrames();
    const boom = new Error('unreachable');
    const fault = vi.fn();
    startFrames({
      intervalMs: () => 0,
      frame: () => {
        throw boom;
      },
      fault,
    });
    frames.runAt(0);
    expect(fault).toHaveBeenCalledWith(boom);
    expect(frames.waiting.size).toBe(0);
  });

  it('runs_no_frame_once_stopped', () => {
    const frames = fakeFrames();
    const frame = vi.fn();
    const loop = startFrames({ intervalMs: () => 0, frame, fault: vi.fn() });
    loop.stop();
    frames.runAt(0);
    expect(frame).not.toHaveBeenCalled();
  });
});

describe('drawFrame', () => {
  it('frees_the_buffers_of_a_frame_that_fails_to_draw', () => {
    const buffers = { free: vi.fn() } as unknown as DrawnFrame;
    const view = { tick: vi.fn().mockReturnValue(buffers) };
    const renderer = {
      draw: () => {
        throw new Error('lost');
      },
    };
    expect(() => drawFrame(view, renderer, 1, { width: 10, height: 10 }, null)).toThrow('lost');
    expect(buffers.free).toHaveBeenCalled();
  });

  it('gives_back_the_words_over_the_world_of_the_frame_before_it_frees_it', () => {
    const plate = { name: 'an orc' };
    const buffers = { free: vi.fn(), plates: () => [plate], floats: () => [] } as unknown as DrawnFrame;
    const view = { tick: vi.fn().mockReturnValue(buffers) };
    expect(drawFrame(view, { draw: vi.fn() }, 2, { width: 30, height: 20 }, null)).toEqual({ plates: [plate], floats: [] });
  });

  it('gives_the_view_the_size_and_the_mouse', () => {
    const buffers = { free: vi.fn(), plates: () => [], floats: () => [] } as unknown as DrawnFrame;
    const view = { tick: vi.fn().mockReturnValue(buffers) };
    drawFrame(view, { draw: vi.fn() }, 2, { width: 30, height: 20 }, { x: 4, y: 5 });
    expect(view.tick).toHaveBeenCalledWith(2, 30, 20, 4, 5, true);
    expect(buffers.free).toHaveBeenCalled();
  });
});

describe('followSize', () => {
  it('sizes_the_world_again_when_the_canvas_or_the_screen_changes_and_not_else', () => {
    const canvas = { clientWidth: 1024, clientHeight: 769 };
    vi.stubGlobal('devicePixelRatio', 1.4);
    const apply = vi.fn();
    const follow = followSize(canvas, apply);
    expect(follow()).toEqual({ width: 1024, height: 769, ratio: 1.4 });
    follow();
    expect(apply).toHaveBeenCalledTimes(1);
    canvas.clientWidth = 1280;
    expect(follow().width).toBe(1280);
    vi.stubGlobal('devicePixelRatio', 2);
    expect(follow().ratio).toBe(2);
    expect(apply.mock.calls).toEqual([
      [{ width: 1024, height: 769, ratio: 1.4 }],
      [{ width: 1280, height: 769, ratio: 1.4 }],
      [{ width: 1280, height: 769, ratio: 2 }],
    ]);
  });

  it('counts_the_view_in_the_points_the_ui_scale_grows', () => {
    const canvas = { clientWidth: 1000, clientHeight: 800 };
    vi.stubGlobal('devicePixelRatio', 1);
    const apply = vi.fn();
    const follow = followSize(canvas, apply);
    follow();
    setPointScale(2);
    expect(follow()).toEqual({ width: 500, height: 400, ratio: 2 });
    expect(apply).toHaveBeenCalledTimes(2);
    setPointScale(1);
  });
});
