/**
 * How the world paints, as egui paints a mesh: each vertex has a position
 * in points, a u, v in a texture and a premultiplied color; a pixel is the
 * texture times the color, laid over what is under it by its alpha
 * (premultiplied blending). Colors stay as they are: no color space
 * conversion, as egui blends in the colors it is given.
 */

import * as THREE from 'three';

const VERTEX_SHADER = `
precision highp float;
uniform mat4 projectionMatrix;
uniform mat4 modelViewMatrix;
in vec2 position;
in vec2 uv;
in vec4 color;
out vec2 vUv;
out vec4 vColor;
void main() {
  vUv = uv;
  vColor = color;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 0.0, 1.0);
}
`;

// Both the texture and the color are premultiplied, so their product is
// too: the alpha is not multiplied in again.
const FRAGMENT_SHADER = `
precision highp float;
uniform sampler2D picture;
in vec2 vUv;
in vec4 vColor;
out vec4 paint;
void main() {
  paint = texture(picture, vUv) * vColor;
}
`;

/** A material that paints `texture` times the vertex colors, premultiplied. */
export function paintMaterial(texture: THREE.Texture): THREE.RawShaderMaterial {
  return new THREE.RawShaderMaterial({
    glslVersion: THREE.GLSL3,
    vertexShader: VERTEX_SHADER,
    fragmentShader: FRAGMENT_SHADER,
    uniforms: { picture: { value: texture } },
    transparent: true,
    depthTest: false,
    depthWrite: false,
    blending: THREE.CustomBlending,
    blendEquation: THREE.AddEquation,
    blendSrc: THREE.OneFactor,
    blendDst: THREE.OneMinusSrcAlphaFactor,
  });
}

/** Points the material at another texture. */
export function paintTexture(material: THREE.RawShaderMaterial, texture: THREE.Texture): void {
  material.uniforms.picture.value = texture;
}
