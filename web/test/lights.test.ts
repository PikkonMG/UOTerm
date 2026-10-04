import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import { LightLayer } from '../src/world/lights';

const CELL = 4;

/** The texture the light layer paints with. */
const textureOf = (layer: LightLayer) => (layer.mesh.material as THREE.RawShaderMaterial).uniforms.picture.value as THREE.DataTexture;

describe('LightLayer', () => {
  it('stretches_the_cells_over_the_view_from_its_top_left', () => {
    const layer = new LightLayer();
    const cells = new Uint8Array(3 * 2 * 4);
    layer.draw(3, 2, cells, CELL);
    expect(layer.mesh.visible).toBe(true);
    expect([...(layer.mesh.geometry.getAttribute('position').array as Float32Array)]).toEqual([0, 0, 12, 0, 12, 8, 0, 8]);
    const texture = textureOf(layer);
    expect(texture.image).toMatchObject({ width: 3, height: 2, data: cells });
    expect(texture.magFilter).toBe(THREE.LinearFilter);
  });

  it('takes_new_cells_of_the_same_size_into_the_same_texture', () => {
    const layer = new LightLayer();
    layer.draw(1, 1, new Uint8Array(4), CELL);
    const first = textureOf(layer);
    const next = new Uint8Array([1, 2, 3, 4]);
    layer.draw(1, 1, next, CELL);
    expect(textureOf(layer)).toBe(first);
    expect(first.image.data).toBe(next);
  });

  it('hides_when_the_world_shows_in_its_own_light', () => {
    const layer = new LightLayer();
    layer.draw(0, 0, new Uint8Array(0), CELL);
    expect(layer.mesh.visible).toBe(false);
  });
});
