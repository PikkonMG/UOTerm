/**
 * The one large texture that holds every picture the map draws, so the
 * whole map is one mesh and one draw call. The view places each picture
 * (`uoterm_view::atlas`); here the pixels go where it placed them, read
 * the nearest pixel as the Rust window does (`TextureOptions::NEAREST`).
 */

import * as THREE from 'three';

/** One picture placed this frame: copy the pixels of `key` to `x`, `y`. */
export interface Upload {
  key: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The parts of the renderer the atlas uses. */
export type AtlasGpu = Pick<THREE.WebGLRenderer, 'copyTextureToTexture' | 'setRenderTarget' | 'getRenderTarget' | 'setClearColor' | 'clear'>;

const CLEAR = new THREE.Color(0, 0, 0);
const CLEAR_ALPHA = 0;
const CHANNEL_MAX = 255;
const RGBA = 4;
const CORNER = new THREE.Vector2(0, 0);

export class AtlasTexture {
  readonly texture: THREE.Texture;
  private readonly target: THREE.WebGLRenderTarget;
  /** The plain white square: a shape with no picture reads it and keeps its own color. */
  private readonly white: THREE.DataTexture;

  /** A texture `side` pixels square, cleared, with the white square of `whiteSide` in its corner. */
  constructor(
    private readonly gpu: AtlasGpu,
    side: number,
    whiteSide: number,
  ) {
    this.target = new THREE.WebGLRenderTarget(side, side, {
      minFilter: THREE.NearestFilter,
      magFilter: THREE.NearestFilter,
      generateMipmaps: false,
      depthBuffer: false,
    });
    this.texture = this.target.texture;
    this.white = new THREE.DataTexture(new Uint8Array(whiteSide * whiteSide * RGBA).fill(CHANNEL_MAX), whiteSide, whiteSide);
    this.reset();
  }

  /** Copies the pixels of each picture to its place. A picture whose pixels are gone stays blank. */
  upload(list: Upload[], pixelsOf: (key: string) => ImageBitmap | undefined): void {
    for (const { key, x, y, width, height } of list) {
      const pixels = pixelsOf(key);
      if (!pixels) continue;
      const picture = new THREE.Texture(pixels);
      const region = new THREE.Box2(new THREE.Vector2(0, 0), new THREE.Vector2(width, height));
      this.gpu.copyTextureToTexture(picture, this.texture, region, new THREE.Vector2(x, y));
    }
  }

  /** Clears every picture and lays the white square again. */
  reset(): void {
    const before = this.gpu.getRenderTarget();
    this.gpu.setRenderTarget(this.target);
    this.gpu.setClearColor(CLEAR, CLEAR_ALPHA);
    this.gpu.clear(true, false, false);
    this.gpu.setRenderTarget(before);
    this.gpu.copyTextureToTexture(this.white, this.texture, null, CORNER);
  }

  dispose(): void {
    this.target.dispose();
    this.white.dispose();
  }
}
