import { NextResponse } from 'next/server';

import { runSolscanResearcher } from '@/lib/scanners/solscanResearcher';

export const dynamic = 'force-dynamic';

export async function GET() {
  try {
    const data = await runSolscanResearcher();
    return NextResponse.json(data);
  } catch (e) {
    const message = e instanceof Error ? e.message : 'scanner failed';
    return NextResponse.json({ error: message, hits: [], meta: { degraded: true } }, { status: 500 });
  }
}
