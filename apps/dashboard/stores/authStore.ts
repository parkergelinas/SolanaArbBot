'use client';

import { create } from 'zustand';
import bs58 from 'bs58';

interface AuthState {
  isAuthenticated: boolean;
  walletAddress: string | null;
  isLoading: boolean;
  error: string | null;
  authenticate: (
    signMessage: (message: Uint8Array) => Promise<Uint8Array>,
    walletAddress: string,
  ) => Promise<void>;
  logout: () => Promise<void>;
  hydrateFromSession: () => Promise<void>;
}

export const useAuthStore = create<AuthState>()((set) => ({
  isAuthenticated: true, // Auth disabled — always open
  walletAddress: null,
  isLoading: false,
  error: null,

  authenticate: async (signMessage, walletAddress) => {
    set({ isLoading: true, error: null });
    try {
      const challengeRes = await fetch('/api/auth/challenge', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ walletAddress }),
      });
      if (!challengeRes.ok) throw new Error('Failed to get challenge');
      const { nonce, message } = await challengeRes.json();

      const messageBytes = new TextEncoder().encode(message);
      const signatureBytes = await signMessage(messageBytes);
      const signature = bs58.encode(signatureBytes);

      const verifyRes = await fetch('/api/auth/verify', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ walletAddress, signature, nonce }),
      });
      if (!verifyRes.ok) {
        const err = await verifyRes.json();
        throw new Error(err.error ?? 'Authentication failed');
      }

      set({ isAuthenticated: true, walletAddress, isLoading: false, error: null });
    } catch (err) {
      set({
        isLoading: false,
        error: err instanceof Error ? err.message : 'Authentication failed',
        isAuthenticated: false,
      });
    }
  },

  logout: async () => {
    await fetch('/api/auth/logout', { method: 'POST' });
    set({ isAuthenticated: false, walletAddress: null, error: null });
  },

  hydrateFromSession: async () => {
    try {
      const res = await fetch('/api/auth/session');
      if (res.ok) {
        const { walletAddress } = await res.json();
        set({ isAuthenticated: true, walletAddress });
      }
    } catch {
      // session check failed — stay unauthenticated
    }
  },
}));
