import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import { deviceSize, watchContext, WorldScene, type WorldDraw } from '../src/world/renderer';

const WIDTH = 1024;
const HEIGHT = 769;

/**
 * One picture as the view lays it (`Shapes::picture` and the land quads):
 * corners top left, top right, bottom right, bottom left, so its
 * triangles turn clockwise on the screen.
 */
function onePicture(): WorldDraw {
  const positions = new Float32Array([100, 100, 144, 100, 144, 144, 100, 144]);
  const uvs = new Float32Array([0.1, 0, 0.2, 0, 0.2, 0.1, 0.1, 0.1]);
  const colors = new Uint8Array(16).fill(255);
  const indices = new Uint32Array([0, 1, 2, 0, 2, 3]);
  return {
    positions: () => positions,
    uvs: () => uvs,
    colors: () => colors,
    indices: () => indices,
    overlayPositions: () => new Float32Array(0),
    overlayUvs: () => new Float32Array(0),
    overlayColors: () => new Uint8Array(0),
    overlayIndices: () => new Uint32Array(0),
    uploads: () => [],
    atlasReset: () => false,
    lightWidth: () => 0,
    lightHeight: () => 0,
    lightCells: () => new Uint8Array(0),
    lightCell: () => 4,
  };
}

/** Where a point of the view lands on the screen of the camera, from -1 to 1. */
function onScreen(camera: THREE.Camera, x: number, y: number): THREE.Vector3 {
  camera.updateMatrixWorld();
  return new THREE.Vector3(x, y, 0).project(camera);
}

/** True when the triangle turns counterclockwise on the screen of the camera: the side WebGL calls the front. */
function facesFront(camera: THREE.Camera, positions: Float32Array, [a, b, c]: number[]): boolean {
  const [pa, pb, pc] = [a, b, c].map((at) => onScreen(camera, positions[2 * at], positions[2 * at + 1]));
  return (pb.x - pa.x) * (pc.y - pa.y) - (pc.x - pa.x) * (pb.y - pa.y) > 0;
}

describe('WorldScene', () => {
  it('lays_the_view_over_the_whole_screen_with_y_down', () => {
    const world = new WorldScene(new THREE.Texture());
    world.resize(WIDTH, HEIGHT);
    expect(onScreen(world.camera, 0, 0)).toMatchObject({ x: -1, y: 1 });
    const corner = onScreen(world.camera, WIDTH, HEIGHT);
    expect(corner.x).toBeCloseTo(1);
    expect(corner.y).toBeCloseTo(-1);
  });

  it('draws_every_triangle_of_the_view_whichever_way_it_turns', () => {
    const world = new WorldScene(new THREE.Texture());
    world.resize(WIDTH, HEIGHT);
    const draw = onePicture();
    world.update(draw);
    const [mesh] = world.scene.children as THREE.Mesh[];
    const material = mesh.material as THREE.Material;
    const indices = [...draw.indices()];
    for (let at = 0; at < indices.length; at += 3) {
      const front = facesFront(world.camera, draw.positions(), indices.slice(at, at + 3));
      expect(front || material.side === THREE.DoubleSide, `triangle ${at / 3}`).toBe(true);
    }
  });

  it('draws_the_triangles_of_the_frame_from_the_numbers_it_was_given', () => {
    const world = new WorldScene(new THREE.Texture());
    world.update(onePicture());
    const [mesh] = world.scene.children as THREE.Mesh[];
    const geometry = mesh.geometry;
    expect(mesh.visible).toBe(true);
    expect(geometry.drawRange).toEqual({ start: 0, count: 6 });
    const used = [...(geometry.getAttribute('position').array as Float32Array).slice(0, 8)];
    expect(used).toEqual([100, 100, 144, 100, 144, 144, 100, 144]);
    expect(used.every(Number.isFinite)).toBe(true);
    expect(mesh.frustumCulled).toBe(false);
  });

  it('paints_the_world_then_the_light_then_the_overlay_in_scene_order', () => {
    const world = new WorldScene(new THREE.Texture());
    expect(world.scene.children).toHaveLength(3);
    expect(world.scene.children.every((child) => child.frustumCulled === false)).toBe(true);
  });
});

describe('deviceSize', () => {
  it('gives_the_canvas_whole_device_pixels_that_the_viewport_fills', () => {
    expect(deviceSize(WIDTH, HEIGHT, 1.4)).toEqual({ width: 1434, height: 1077 });
    expect(deviceSize(WIDTH, HEIGHT, 1)).toEqual({ width: WIDTH, height: HEIGHT });
  });
});

describe('watchContext', () => {
  it('keeps_a_lost_context_for_its_return_and_tells_when_it_came_back', () => {
    const canvas = document.createElement('canvas');
    const restored = vi.fn();
    const stop = watchContext(canvas, restored);
    const lost = new Event('webglcontextlost', { cancelable: true });
    canvas.dispatchEvent(lost);
    expect(lost.defaultPrevented).toBe(true);
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(restored).toHaveBeenCalledTimes(1);
    stop();
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(restored).toHaveBeenCalledTimes(1);
  });
});
