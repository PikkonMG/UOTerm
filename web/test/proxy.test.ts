import { EventEmitter } from 'node:events';
import { describe, expect, it, vi } from 'vitest';
import { API_TARGET, apiProxy } from '../vite.config';

describe('apiProxy', () => {
  it('puts_the_api_in_the_origin_of_http_and_websocket_requests', () => {
    const proxy = new EventEmitter();
    const options = apiProxy();
    options.configure?.(proxy as never, options);
    for (const event of ['proxyReq', 'proxyReqWs']) {
      const request = { setHeader: vi.fn() };
      proxy.emit(event, request);
      expect(request.setHeader).toHaveBeenCalledWith('origin', API_TARGET);
    }
    expect(options).toMatchObject({ target: API_TARGET, ws: true, changeOrigin: true });
  });
});
