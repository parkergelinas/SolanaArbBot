import { NextRequest, NextResponse } from 'next/server';

/** Resolve control-api base URL (no trailing slash, no /api suffix). */
export function controlApiBase(): string | null {
  const configured = process.env.CONTROL_API_URL?.trim().replace(/\/$/, '');
  if (configured) return configured;
  if (process.env.NODE_ENV === 'development') return 'http://localhost:3001';
  return null;
}

export async function proxyToControlApi(
  req: NextRequest,
  pathSegments: string[],
): Promise<NextResponse> {
  const base = controlApiBase();
  if (!base) {
    return NextResponse.json(
      {
        error:
          'CONTROL_API_URL is not configured. Add it in Vercel → Settings → Environment Variables.',
      },
      { status: 503 },
    );
  }

  const path = pathSegments.join('/');
  const url = `${base}/api/${path}${req.nextUrl.search}`;

  const headers = new Headers();
  const contentType = req.headers.get('content-type');
  if (contentType) headers.set('content-type', contentType);

  const init: RequestInit = {
    method: req.method,
    headers,
    cache: 'no-store',
  };

  if (req.method !== 'GET' && req.method !== 'HEAD') {
    init.body = await req.text();
  }

  try {
    const res = await fetch(url, init);
    const body = await res.text();
    return new NextResponse(body, {
      status: res.status,
      headers: {
        'content-type': res.headers.get('content-type') ?? 'application/json',
      },
    });
  } catch {
    return NextResponse.json(
      { error: `control-api unreachable at ${base}` },
      { status: 502 },
    );
  }
}
