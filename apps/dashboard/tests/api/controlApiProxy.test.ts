import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextRequest } from 'next/server';

// Dynamic import so env stubs applied before module resolution.
async function loadProxy() {
  return import('@/lib/control-api-proxy');
}

function makeReq(method = 'GET', path = '/api/status', body?: string): NextRequest {
  const url = `http://localhost${path}`;
  return new NextRequest(url, {
    method,
    body: body ?? undefined,
    headers: body ? { 'content-type': 'application/json' } : {},
  });
}

describe('controlApiBase', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('returns null when CONTROL_API_URL unset outside development', async () => {
    vi.stubEnv('CONTROL_API_URL', '');
    vi.stubEnv('NODE_ENV', 'production');
    const { controlApiBase } = await loadProxy();
    expect(controlApiBase()).toBeNull();
  });

  it('returns 127.0.0.1:3001 in development when CONTROL_API_URL unset', async () => {
    vi.stubEnv('CONTROL_API_URL', '');
    vi.stubEnv('NODE_ENV', 'development');
    const { controlApiBase } = await loadProxy();
    expect(controlApiBase()).toBe('http://127.0.0.1:3001');
  });

  it('returns configured URL with trailing slash stripped', async () => {
    vi.stubEnv('CONTROL_API_URL', 'https://control.example.com/');
    const { controlApiBase } = await loadProxy();
    expect(controlApiBase()).toBe('https://control.example.com');
  });
});

describe('proxyToControlApi', () => {
  beforeEach(() => {
    vi.stubEnv('NODE_ENV', 'production');
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
    vi.restoreAllMocks();
  });

  it('returns 503 when CONTROL_API_URL is not set', async () => {
    vi.stubEnv('CONTROL_API_URL', '');
    const { proxyToControlApi } = await loadProxy();
    const req = makeReq();
    const res = await proxyToControlApi(req, ['status']);
    expect(res.status).toBe(503);
    const body = await res.json();
    expect(body.error).toContain('CONTROL_API_URL');
  });

  it('forwards x-api-key header when CONTROL_API_KEY is set', async () => {
    vi.stubEnv('CONTROL_API_URL', 'https://control.example.com');
    vi.stubEnv('CONTROL_API_KEY', 'secret-key-xyz');
    const { proxyToControlApi } = await loadProxy();

    let capturedHeaders: Headers | undefined;
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (_url, init) => {
      capturedHeaders = init?.headers as Headers;
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });

    const req = makeReq();
    await proxyToControlApi(req, ['status']);

    expect(capturedHeaders?.get('x-api-key')).toBe('secret-key-xyz');
  });

  it('does NOT forward x-api-key when CONTROL_API_KEY is absent', async () => {
    vi.stubEnv('CONTROL_API_URL', 'https://control.example.com');
    vi.stubEnv('CONTROL_API_KEY', '');
    const { proxyToControlApi } = await loadProxy();

    let capturedHeaders: Headers | undefined;
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (_url, init) => {
      capturedHeaders = init?.headers as Headers;
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });

    const req = makeReq();
    await proxyToControlApi(req, ['status']);

    expect(capturedHeaders?.get('x-api-key')).toBeNull();
  });

  it('returns 502 when control-api is unreachable', async () => {
    vi.stubEnv('CONTROL_API_URL', 'https://control.example.com');
    vi.stubEnv('CONTROL_API_KEY', '');
    const { proxyToControlApi } = await loadProxy();

    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('ECONNREFUSED'));

    const req = makeReq();
    const res = await proxyToControlApi(req, ['status']);
    expect(res.status).toBe(502);
    const body = await res.json();
    expect(body.error).toContain('unreachable');
  });

  it('forwards response status from upstream', async () => {
    vi.stubEnv('CONTROL_API_URL', 'https://control.example.com');
    vi.stubEnv('CONTROL_API_KEY', '');
    const { proxyToControlApi } = await loadProxy();

    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ error: 'not found' }), {
        status: 404,
        headers: { 'content-type': 'application/json' },
      }),
    );

    const req = makeReq();
    const res = await proxyToControlApi(req, ['missing-route']);
    expect(res.status).toBe(404);
  });

  it('sends POST body through to upstream', async () => {
    vi.stubEnv('CONTROL_API_URL', 'https://control.example.com');
    vi.stubEnv('CONTROL_API_KEY', '');
    const { proxyToControlApi } = await loadProxy();

    let capturedBody: string | null = null;
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (_url, init) => {
      capturedBody = init?.body as string;
      return new Response('{}', {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });

    const payload = JSON.stringify({ action: 'pause' });
    const req = makeReq('POST', '/api/bot/control', payload);
    await proxyToControlApi(req, ['bot', 'control']);
    expect(capturedBody).toBe(payload);
  });

  it('builds the correct upstream URL from path segments', async () => {
    vi.stubEnv('CONTROL_API_URL', 'https://control.example.com');
    vi.stubEnv('CONTROL_API_KEY', '');
    const { proxyToControlApi } = await loadProxy();

    let capturedUrl: string | undefined;
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (url) => {
      capturedUrl = url as string;
      return new Response('{}', {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    });

    const req = makeReq('GET', '/api/bot/metrics');
    await proxyToControlApi(req, ['bot', 'metrics']);
    expect(capturedUrl).toBe('https://control.example.com/api/bot/metrics');
  });
});
