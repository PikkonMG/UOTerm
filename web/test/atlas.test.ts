import { describe, expect, it, vi } from 'vitest';
import { AtlasTexture, type AtlasGpu } from '../src/world/atlas';

const SIDE = 64;
const WHITE = 4;

/** A renderer that records what the atlas asks of it. */
function fakeGpu() {
  return {
    copyTextureToTexture: vi.fn(),
    setRenderTarget: vi.fn(),
    getRenderTarget: vi.fn().mockReturnValue(null),
    setClearColor: vi.fn(),
    clear: vi.fn(),
  };
}

const bitmap = { width: 2, height: 2 } as ImageBitmap;

describe('AtlasTexture', () => {
  it('lays_the_white_square_in_the_corner_when_made', () => {
    const gpu = fakeGpu();
    const atlas = new AtlasTexture(gpu as unknown as AtlasGpu, SIDE, WHITE);
    expect(gpu.clear).toHaveBeenCalledTimes(1);
    expect(gpu.copyTextureToTexture).toHaveBeenCalledTimes(1);
    const [white, target, , at] = gpu.copyTextureToTexture.mock.calls[0];
    expect(target).toBe(atlas.texture);
    expect(white.image).toMatchObject({ width: WHITE, height: WHITE });
    expect(at).toMatchObject({ x: 0, y: 0 });
  });

  it('copies_each_picture_to_its_place', () => {
    const gpu = fakeGpu();
    const atlas = new AtlasTexture(gpu as unknown as AtlasGpu, SIDE, WHITE);
    gpu.copyTextureToTexture.mockClear();
    atlas.upload([{ key: '1', x: 4, y: 8, width: 2, height: 2 }], () => bitmap);
    expect(gpu.copyTextureToTexture).toHaveBeenCalledTimes(1);
    const [picture, target, region, at] = gpu.copyTextureToTexture.mock.calls[0];
    expect(picture.image).toBe(bitmap);
    expect(target).toBe(atlas.texture);
    expect(region).toMatchObject({ min: { x: 0, y: 0 }, max: { x: 2, y: 2 } });
    expect(at).toMatchObject({ x: 4, y: 8 });
  });

  it('copies_nothing_for_a_picture_whose_pixels_are_gone', () => {
    const gpu = fakeGpu();
    const atlas = new AtlasTexture(gpu as unknown as AtlasGpu, SIDE, WHITE);
    gpu.copyTextureToTexture.mockClear();
    atlas.upload([{ key: 'gone', x: 4, y: 8, width: 2, height: 2 }], () => undefined);
    expect(gpu.copyTextureToTexture).not.toHaveBeenCalled();
  });

  it('clears_the_texture_and_lays_the_white_square_again_on_reset', () => {
    const gpu = fakeGpu();
    const atlas = new AtlasTexture(gpu as unknown as AtlasGpu, SIDE, WHITE);
    gpu.clear.mockClear();
    gpu.copyTextureToTexture.mockClear();
    atlas.reset();
    expect(gpu.clear).toHaveBeenCalledTimes(1);
    expect(gpu.copyTextureToTexture).toHaveBeenCalledTimes(1);
    expect(gpu.setRenderTarget).toHaveBeenLastCalledWith(null);
  });
});
