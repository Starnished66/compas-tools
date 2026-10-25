import type { ButtonHTMLAttributes } from 'react';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
}

export function Button({ variant = 'secondary', className = '', type = 'button', ...rest }: Props) {
  const classes = `button button--${variant} ${className}`.trim();
  return <button type={type} className={classes} {...rest} />;
}
