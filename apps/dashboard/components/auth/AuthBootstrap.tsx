'use client';

import { useEffect } from 'react';

import { useAuthStore } from '@/stores/authStore';

export default function AuthBootstrap() {
  const hydrateFromSession = useAuthStore((s) => s.hydrateFromSession);

  useEffect(() => {
    hydrateFromSession();
  }, [hydrateFromSession]);

  return null;
}
