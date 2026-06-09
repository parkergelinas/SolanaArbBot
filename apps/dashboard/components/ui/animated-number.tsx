'use client';

import { useEffect, useRef } from 'react';
import { useMotionValue, useSpring, useTransform, motion } from 'framer-motion';

import type { CSSProperties } from 'react';

interface AnimatedNumberProps {
  value: number;
  format?: (n: number) => string;
  className?: string;
  style?: CSSProperties;
}

export function AnimatedNumber({ value, format, className, style }: AnimatedNumberProps) {
  const motionVal = useMotionValue(value);
  const spring = useSpring(motionVal, { stiffness: 120, damping: 20, mass: 0.8 });
  const display = useTransform(spring, (v) =>
    format ? format(v) : v.toLocaleString('en-US', { maximumFractionDigits: 2 }),
  );

  const prevRef = useRef(value);
  useEffect(() => {
    if (prevRef.current !== value) {
      motionVal.set(value);
      prevRef.current = value;
    }
  }, [value, motionVal]);

  return <motion.span className={className} style={style}>{display}</motion.span>;
}
