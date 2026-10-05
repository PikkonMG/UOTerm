/**
 * The screenshot of the page: the world of the WebGL canvas and the layer
 * of the panels over it, in one PNG at the pixels of the screen, saved by
 * the server in the `screenshots` folder as the Rust window saves its own.
 *
 * The canvas keeps no drawing after the frame (`preserveDrawingBuffer` is
 * off), so the page takes the screenshot right after a draw, in the same
 * task. The panels are HTML: they are drawn through an SVG picture of the
 * layer, with the styles of the page and the pictures of its canvases in
 * it. A browser that refuses such a picture on a canvas gets the world
 * alone.
 */

import { api, METHOD_POST } from './net/api';

/** Every PNG file starts with these bytes; the server takes nothing else. */
export const PNG_SIGNATURE = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

const SCREENSHOTS_PATH = '/v1/screenshots';
const PNG_TYPE = 'image/png';
const SVG_DATA = 'data:image/svg+xml;charset=utf-8,';
const SVG_NAMESPACE = 'http://www.w3.org/2000/svg';
const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';

/** Draws the layer of the panels as a picture of `width` by `height` CSS pixels; null when it cannot be drawn. */
export type OverlayPicture = (overlay: HTMLElement, width: number, height: number) => Promise<CanvasImageSource | null>;

/** The rules of every style sheet of the page that may be read. */
function pageStyles(): string {
  return [...document.styleSheets]
    .flatMap((sheet) => {
      try {
        return [...sheet.cssRules].map((rule) => rule.cssText);
      } catch {
        return [];
      }
    })
    .join('\n');
}

/** A copy of `overlay` with each canvas as a picture of what it shows, since a copy of a canvas is blank. */
function copyWithPictures(overlay: HTMLElement): HTMLElement {
  const copy = overlay.cloneNode(true) as HTMLElement;
  const canvases = overlay.querySelectorAll('canvas');
  copy.querySelectorAll('canvas').forEach((blank, at) => {
    const picture = document.createElement('img');
    picture.className = blank.className;
    picture.style.cssText = blank.style.cssText;
    picture.width = canvases[at].clientWidth;
    picture.height = canvases[at].clientHeight;
    try {
      picture.src = canvases[at].toDataURL(PNG_TYPE);
    } catch {
      // A canvas that may not be read stays empty in the screenshot.
    }
    blank.replaceWith(picture);
  });
  return copy;
}

/** The layer of the panels drawn through an SVG picture of it. */
export const svgOverlay: OverlayPicture = (overlay, width, height) => {
  const svg = document.createElementNS(SVG_NAMESPACE, 'svg');
  svg.setAttribute('width', String(width));
  svg.setAttribute('height', String(height));
  const body = document.createElementNS(SVG_NAMESPACE, 'foreignObject');
  body.setAttribute('width', '100%');
  body.setAttribute('height', '100%');
  const page = document.createElementNS(XHTML_NAMESPACE, 'div');
  const style = document.createElementNS(XHTML_NAMESPACE, 'style');
  style.textContent = pageStyles();
  page.append(style, copyWithPictures(overlay));
  body.append(page);
  svg.append(body);
  return new Promise((resolve) => {
    const picture = new Image();
    picture.onload = () => resolve(picture);
    picture.onerror = () => resolve(null);
    // A data address, not an object one: Chrome lets a canvas be read after
    // drawing such a picture from a data address only.
    picture.src = `${SVG_DATA}${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
  });
};

/** The PNG of a canvas; null when the browser refuses to read it. */
function pngOf(canvas: HTMLCanvasElement): Promise<Blob | null> {
  return new Promise((resolve) => {
    try {
      canvas.toBlob(resolve, PNG_TYPE);
    } catch {
      resolve(null);
    }
  });
}

/** The world, with the panels over it when `panels` is given, at the size of the world canvas. */
function compose(world: HTMLCanvasElement, panels: CanvasImageSource | null): HTMLCanvasElement {
  const shot = document.createElement('canvas');
  shot.width = world.width;
  shot.height = world.height;
  const context = shot.getContext('2d');
  context?.drawImage(world, 0, 0, shot.width, shot.height);
  if (panels) context?.drawImage(panels, 0, 0, shot.width, shot.height);
  return shot;
}

/**
 * Takes the screenshot of the world `canvas` (call it right after a draw)
 * and the panel layer `overlay`, and saves it. Gives the name of the file
 * the server wrote; fails with the words of the fault.
 */
export async function takeScreenshot(canvas: HTMLCanvasElement, overlay: HTMLElement, picture: OverlayPicture = svgOverlay): Promise<string> {
  // The world is copied before any wait: the canvas loses its drawing after the task.
  const world = compose(canvas, null);
  const panels = await picture(overlay, window.innerWidth, window.innerHeight);
  const png = (panels && (await pngOf(compose(world, panels)))) || (await pngOf(world));
  if (!png) throw new Error('the browser gave no picture of the page');
  const { file } = await api<{ file: string }>(SCREENSHOTS_PATH, { method: METHOD_POST, headers: { 'content-type': PNG_TYPE }, body: png });
  return file;
}
