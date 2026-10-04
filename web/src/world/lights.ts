/**
 * The light of the world over the map, as the Rust window's `LightMap`
 * paints it: one premultiplied pixel per cell of the light map, stretched
 * over the view from its top left with smooth (linear) sampling, and laid
 * over the world by its alpha. The light rules are `uoterm_view::lights`.
 */

import * as THREE from 'three';
import { paintMaterial, paintTexture } from './material';

const CORNERS = 4;
const XY = 2;
const RGBA = 4;
const WHITE = 255;
/** The two triangles of the quad, from its corners round it. */
const QUAD = [0, 1, 2, 0, 2, 3];
/** The u, v of the corners: top left, top right, bottom right, bottom left. */
const QUAD_UVS = [0, 0, 1, 0, 1, 1, 0, 1];

export class LightLayer {
  readonly mesh: THREE.Mesh;
  private readonly material: THREE.RawShaderMaterial;
  private readonly corners = new THREE.BufferAttribute(new Float32Array(CORNERS * XY), XY);
  private texture: THREE.DataTexture;

  constructor() {
    this.texture = LightLayer.cellTexture(new Uint8Array(RGBA), 1, 1);
    this.material = paintMaterial(this.texture);
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', this.corners);
    geometry.setAttribute('uv', new THREE.BufferAttribute(new Float32Array(QUAD_UVS), XY));
    geometry.setAttribute('color', new THREE.BufferAttribute(new Uint8Array(CORNERS * RGBA).fill(WHITE), RGBA, true));
    geometry.setIndex(QUAD);
    this.mesh = new THREE.Mesh(geometry, this.material);
    this.mesh.frustumCulled = false;
    this.mesh.visible = false;
  }

  /**
   * Lays the light map of `width` by `height` cells, each `cell` points
   * square, from the top left of the view. A map with no cells hides the
   * layer: the world shows in its own light.
   */
  draw(width: number, height: number, cells: Uint8Array, cell: number): void {
    this.mesh.visible = width > 0 && height > 0;
    if (!this.mesh.visible) return;
    const image = this.texture.image;
    if (image.width === width && image.height === height) {
      image.data = cells;
      this.texture.needsUpdate = true;
    } else {
      this.texture.dispose();
      this.texture = LightLayer.cellTexture(cells, width, height);
      paintTexture(this.material, this.texture);
    }
    const [right, bottom] = [width * cell, height * cell];
    this.corners.array.set([0, 0, right, 0, right, bottom, 0, bottom]);
    this.corners.needsUpdate = true;
  }

  dispose(): void {
    this.mesh.geometry.dispose();
    this.material.dispose();
    this.texture.dispose();
  }

  private static cellTexture(cells: Uint8Array, width: number, height: number): THREE.DataTexture {
    const texture = new THREE.DataTexture(cells, width, height);
    texture.magFilter = THREE.LinearFilter;
    texture.minFilter = THREE.LinearFilter;
    texture.needsUpdate = true;
    return texture;
  }
}
