import type { ReactNode } from 'react';

interface Props {
  tone?: 'neutral' | 'accent' | 'success' | 'warning';
  children: ReactNode;
}

export function Badge({ tone = 'neutral', children }: Props) {
  return <span className={tone === 'neutral' ? 'badge' : `badge badge--${tone}`}>{children}</span>;
}
