import preact from '@preact/preset-vite';
import type { ClientRequest } from 'node:http';
import { defineConfig, type ProxyOptions } from 'vite';

/** Where `uoterm web` listens by default (`api_bind` in uoterm.toml). */
export const API_TARGET = 'http://127.0.0.1:7733';
/** The paths the dev server hands to the API. */
const API_PATHS = ['/v1', '/health'];

/**
 * Sends a request of the page to the API as if the page came from the API.
 *
 * The API answers only the pages it serves: it refuses (403) a request
 * whose `Origin` is not the API itself. The dev page comes from the Vite
 * port, so the proxy puts the API in `Origin` (and, by `changeOrigin`, in
 * `Host`) on every HTTP and WebSocket request it hands on. Without it,
 * every call of the dev page is refused.
 */
export function apiProxy(): ProxyOptions {
  const asTheApi = (request: ClientRequest) => request.setHeader('origin', API_TARGET);
  return {
    target: API_TARGET,
    ws: true,
    changeOrigin: true,
    configure(proxy) {
      proxy.on('proxyReq', asTheApi);
      proxy.on('proxyReqWs', asTheApi);
    },
  };
}

export default defineConfig({
  plugins: [preact()],
  server: {
    proxy: Object.fromEntries(API_PATHS.map((path) => [path, apiProxy()])),
  },
});
