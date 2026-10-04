/**
 * Draws one frame of the world with WebGL, in the order the view gives:
 * the world mesh in paint order, the light map over it, then the overlay
 * mesh (quest arrow, placing preview) over the light. Coordinates are
 * points from the top left of the view, y down, the same as the view's
 * draw lists; the renderer scales them to device pixels.
 */

import * as THREE from 'three';
import { pixelsOf } from '../net/art';
import { AtlasTexture, type Upload } from './atlas';
import { LightLayer } from './lights';
import { paintMaterial } from './material';

const XY = 2;
const RGBA = 4;
/** Room for this many more vertices or indices than a frame needs, so a mesh grows rarely. */
const GROWTH = 2;
const FIRST_ROOM = 1024;
/** The color behind the world, where nothing is drawn: black, as the Rust window shows. */
const BACKGROUND = new THREE.Color(0, 0, 0);
const OPAQUE = 1;
/** The camera sees from just in front of the flat world to just behind it. */
const NEAR = -1;
const FAR = 1;
/** Before the first resize the view is one point square. */
const NO_SIZE = 1;

/** What one frame draws: the `DrawBuffers` of the view. */
export interface WorldDraw {
  positions(): Float32Array;
  uvs(): Float32Array;
  colors(): Uint8Array;
  indices(): Uint32Array;
  overlayPositions(): Float32Array;
  overlayUvs(): Float32Array;
  overlayColors(): Uint8Array;
  overlayIndices(): Uint32Array;
  uploads(): Upload[];
  atlasReset(): boolean;
  lightWidth(): number;
  lightHeight(): number;
  lightCells(): Uint8Array;
  lightCell(): number;
}

/** One mesh whose triangles change each frame, kept in buffers that grow when a frame needs more room. */
class MeshLayer {
  readonly mesh: THREE.Mesh;
  private vertexRoom = 0;
  private indexRoom = 0;

  constructor(material: THREE.Material) {
    this.mesh = new THREE.Mesh(new THREE.BufferGeometry(), material);
    this.mesh.frustumCulled = false;
    this.grow(0, 0);
  }

  update(positions: Float32Array, uvs: Float32Array, colors: Uint8Array, indices: Uint32Array): void {
    const vertices = positions.length / XY;
    if (vertices > this.vertexRoom || indices.length > this.indexRoom) this.grow(vertices, indices.length);
    const geometry = this.mesh.geometry;
    MeshLayer.fill(geometry.getAttribute('position') as THREE.BufferAttribute, positions);
    MeshLayer.fill(geometry.getAttribute('uv') as THREE.BufferAttribute, uvs);
    MeshLayer.fill(geometry.getAttribute('color') as THREE.BufferAttribute, colors);
    MeshLayer.fill(geometry.getIndex() as THREE.BufferAttribute, indices);
    geometry.setDrawRange(0, indices.length);
    this.mesh.visible = indices.length > 0;
  }

  dispose(): void {
    this.mesh.geometry.dispose();
  }

  /** New buffers with room for `vertices` and `indices`, and more. */
  private grow(vertices: number, indices: number): void {
    this.vertexRoom = Math.max(FIRST_ROOM, this.vertexRoom, vertices * GROWTH);
    this.indexRoom = Math.max(FIRST_ROOM, this.indexRoom, indices * GROWTH);
    const geometry = new THREE.BufferGeometry();
    const dynamic = <T extends THREE.BufferAttribute>(attribute: T) => attribute.setUsage(THREE.DynamicDrawUsage);
    geometry.setAttribute('position', dynamic(new THREE.BufferAttribute(new Float32Array(this.vertexRoom * XY), XY)));
    geometry.setAttribute('uv', dynamic(new THREE.BufferAttribute(new Float32Array(this.vertexRoom * XY), XY)));
    geometry.setAttribute('color', dynamic(new THREE.BufferAttribute(new Uint8Array(this.vertexRoom * RGBA), RGBA, true)));
    geometry.setIndex(dynamic(new THREE.BufferAttribute(new Uint32Array(this.indexRoom), 1)));
    this.mesh.geometry.dispose();
    this.mesh.geometry = geometry;
  }

  /** Copies `values` to the start of `attribute` and sends only that part to the GPU. */
  private static fill(attribute: THREE.BufferAttribute, values: ArrayLike<number>): void {
    if (values.length === 0) return;
    (attribute.array as Float32Array | Uint8Array | Uint32Array).set(values);
    attribute.clearUpdateRanges();
    attribute.addUpdateRange(0, values.length);
    attribute.needsUpdate = true;
  }
}

/** The canvas size in whole device pixels for a view of `width` by `height` points, so the viewport fills it exactly. */
export function deviceSize(width: number, height: number, pixelRatio: number): { width: number; height: number } {
  return { width: Math.round(width * pixelRatio), height: Math.round(height * pixelRatio) };
}

/**
 * The layers of the world and the camera that sees them, with no GPU: the
 * world mesh, the light layer, then the overlay mesh, painted in that
 * order (the scene order; the renderer does not sort).
 */
export class WorldScene {
  readonly scene = new THREE.Scene();
  readonly camera = new THREE.OrthographicCamera(0, NO_SIZE, 0, NO_SIZE, NEAR, FAR);
  private readonly material: THREE.RawShaderMaterial;
  private readonly world: MeshLayer;
  private readonly lights = new LightLayer();
  private readonly overlay: MeshLayer;

  /** The layers, with the pictures of `atlas`. */
  constructor(atlas: THREE.Texture) {
    this.material = paintMaterial(atlas);
    this.world = new MeshLayer(this.material);
    this.overlay = new MeshLayer(this.material);
    this.scene.add(this.world.mesh, this.lights.mesh, this.overlay.mesh);
  }

  /** Takes the triangles and the light of one frame. */
  update(buffers: WorldDraw): void {
    this.world.update(buffers.positions(), buffers.uvs(), buffers.colors(), buffers.indices());
    this.lights.draw(buffers.lightWidth(), buffers.lightHeight(), buffers.lightCells(), buffers.lightCell());
    this.overlay.update(buffers.overlayPositions(), buffers.overlayUvs(), buffers.overlayColors(), buffers.overlayIndices());
  }

  /** The camera sees `width` by `height` points from the top left, y down. */
  resize(width: number, height: number): void {
    this.camera.right = width;
    this.camera.bottom = height;
    this.camera.updateProjectionMatrix();
  }

  dispose(): void {
    this.world.dispose();
    this.overlay.dispose();
    this.lights.dispose();
    this.material.dispose();
  }
}

const CONTEXT_LOST = 'webglcontextlost';
const CONTEXT_RESTORED = 'webglcontextrestored';

/**
 * Keeps a WebGL context the browser takes away (to free the GPU, or after
 * a driver fault) ready to come back, and calls `restored` when it does:
 * it comes back empty. Gives the function that stops watching.
 */
export function watchContext(canvas: HTMLCanvasElement, restored: () => void): () => void {
  const lost = (event: Event) => event.preventDefault();
  canvas.addEventListener(CONTEXT_LOST, lost);
  canvas.addEventListener(CONTEXT_RESTORED, restored);
  return () => {
    canvas.removeEventListener(CONTEXT_LOST, lost);
    canvas.removeEventListener(CONTEXT_RESTORED, restored);
  };
}

export class WorldRenderer {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly atlas: AtlasTexture;
  private readonly world: WorldScene;
  private readonly stopWatching: () => void;

  /**
   * A renderer on `canvas`, with a texture of the pictures `atlasSide`
   * square and its white square of `whiteSide`. When the browser gives back
   * a lost context, three.js uploads the meshes again by itself, but the
   * texture of the pictures comes back empty: `atlasLost` tells the view,
   * which then clears it and places every picture again.
   */
  constructor(canvas: HTMLCanvasElement, atlasSide: number, whiteSide: number, atlasLost: () => void) {
    this.renderer = new THREE.WebGLRenderer({ canvas, alpha: false, antialias: false, premultipliedAlpha: true });
    this.stopWatching = watchContext(canvas, atlasLost);
    // The layers paint in the order the view gives; sorting would also ask
    // each 2D mesh for a 3D bounding sphere it does not have.
    this.renderer.sortObjects = false;
    this.atlas = new AtlasTexture(this.renderer, atlasSide, whiteSide);
    this.world = new WorldScene(this.atlas.texture);
  }

  /** Draws one frame: the pictures placed this frame go into the texture first. */
  draw(buffers: WorldDraw): void {
    if (buffers.atlasReset()) this.atlas.reset();
    this.atlas.upload(buffers.uploads(), pixelsOf);
    this.world.update(buffers);
    this.renderer.setClearColor(BACKGROUND, OPAQUE);
    this.renderer.render(this.world.scene, this.world.camera);
  }

  /**
   * The view is `width` by `height` points, with `pixelRatio` device pixels
   * to a point. The canvas takes whole device pixels and the viewport fills
   * it to the pixel.
   */
  resize(width: number, height: number, pixelRatio: number): void {
    const device = deviceSize(width, height, pixelRatio);
    this.renderer.setPixelRatio(1);
    this.renderer.setSize(device.width, device.height, false);
    this.world.resize(width, height);
  }

  /** Lets every GPU resource go, and the context with them. */
  dispose(): void {
    this.stopWatching();
    this.world.dispose();
    this.atlas.dispose();
    this.renderer.dispose();
    this.renderer.forceContextLoss();
  }
}
