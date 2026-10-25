import { t } from '../i18n';
import { Button } from './Button';
import { Icon } from './Icon';

interface Props {
  message: string;
  onDismiss?: () => void;
}

export function ErrorBanner({ message, onDismiss }: Props) {
  return (
    <div className="notice notice--danger" role="alert">
      <Icon name="error" />
      <span className="notice__body">{message}</span>
      {onDismiss && (
        <Button icon="close" onClick={onDismiss}>
          {t('Dismiss')}
        </Button>
      )}
    </div>
  );
}
