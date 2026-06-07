'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';

// Auth disabled — immediately redirect to the dashboard.
export default function LoginPage() {
  const router = useRouter();
  useEffect(() => {
    router.replace('/');
  }, [router]);
  return null;
}
