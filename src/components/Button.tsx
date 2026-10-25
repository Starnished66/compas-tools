import type { ButtonHTMLAttributes } from 'react';

import { Icon } from './Icon';
import type { IconName } from './icons';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
  icon?: IconName;
}

export function Button({
  variant = 'secondary',
  icon,
  className = '',
  type = 'button',
  children,
  ...rest
}: Props) {
  const classes = `button button--${variant} ${className}`.trim();
  return (
    <button type={type} className={classes} {...rest}>
      {icon && <Icon name={icon} />}
      {children}
    </button>
  );
}
