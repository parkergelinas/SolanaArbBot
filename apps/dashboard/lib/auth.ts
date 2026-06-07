// Web Crypto only — works in both Edge (middleware) and Node.js 18+

const SESSION_TTL_S = 8 * 60 * 60; // 8 hours
const NONCE_TTL_MS = 5 * 60 * 1_000; // 5 minutes — generous for slow wallet popups

// Anchor to global so Next.js hot-module reloads in dev don't wipe nonces
// between the /challenge and /verify requests.
declare global {
  // eslint-disable-next-line no-var
  var __arbNonceStore: Map<string, { nonce: string; expiresAt: number }> | undefined;
}
const nonceStore: Map<string, { nonce: string; expiresAt: number }> =
  (global.__arbNonceStore ??= new Map());

function toBase64Url(buf: ArrayBuffer | Uint8Array): string {
  const bytes = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}

function fromBase64Url(s: string): ArrayBuffer {
  const padded = s
    .replace(/-/g, '+')
    .replace(/_/g, '/')
    .padEnd(Math.ceil(s.length / 4) * 4, '=');
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes.buffer;
}

async function getHmacKey(): Promise<CryptoKey> {
  const secret = process.env.AUTH_SECRET ?? 'dev-secret-change-me-in-production';
  return globalThis.crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign', 'verify'],
  );
}

export function generateNonce(walletAddress: string): string {
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  const nonce = Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
  nonceStore.set(walletAddress, { nonce, expiresAt: Date.now() + NONCE_TTL_MS });
  return nonce;
}

export function consumeNonce(walletAddress: string, nonce: string): boolean {
  const entry = nonceStore.get(walletAddress);
  if (!entry) return false;
  nonceStore.delete(walletAddress);
  if (Date.now() > entry.expiresAt) return false;
  return entry.nonce === nonce;
}

const DEFAULT_APPROVED_WALLETS = ['127fFEvPQ4FtaLQGmNC9MNQBPA6UJ9HftndJNxJNDVJE'];

export function isApprovedWallet(walletAddress: string): boolean {
  const approved = process.env.APPROVED_WALLETS ?? '';
  // If env var is not set, fall back to the default list.
  // If env var is set to "disabled" or "*", allow any wallet.
  if (!approved || approved === '*' || approved === 'disabled') {
    return DEFAULT_APPROVED_WALLETS.includes(walletAddress) || !approved;
  }
  return approved
    .split(',')
    .map((s) => s.trim())
    .includes(walletAddress);
}

export async function signJwt(walletAddress: string): Promise<string> {
  const enc = new TextEncoder();
  const header = toBase64Url(enc.encode(JSON.stringify({ alg: 'HS256', typ: 'JWT' })));
  const payload = toBase64Url(
    enc.encode(
      JSON.stringify({
        sub: walletAddress,
        iat: Math.floor(Date.now() / 1000),
        exp: Math.floor(Date.now() / 1000) + SESSION_TTL_S,
      }),
    ),
  );
  const key = await getHmacKey();
  const sig = await globalThis.crypto.subtle.sign('HMAC', key, enc.encode(`${header}.${payload}`));
  return `${header}.${payload}.${toBase64Url(sig)}`;
}

export async function verifyJwt(token: string): Promise<string | null> {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const [header, payload, sig] = parts;
    const enc = new TextEncoder();
    const key = await getHmacKey();
    const valid = await globalThis.crypto.subtle.verify(
      'HMAC',
      key,
      fromBase64Url(sig),
      enc.encode(`${header}.${payload}`),
    );
    if (!valid) return null;
    const { sub, exp } = JSON.parse(new TextDecoder().decode(new Uint8Array(fromBase64Url(payload))));
    if (exp < Math.floor(Date.now() / 1000)) return null;
    return sub as string;
  } catch {
    return null;
  }
}
