import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { ConfirmDialog } from './ConfirmDialog';
import { ErrorBanner } from './ErrorBanner';
import { ProgressBar } from './ProgressBar';
import { Switch } from './Switch';

describe('ProgressBar', () => {
  it('exposes the percentage to assistive technology', () => {
    render(<ProgressBar label="Example" progress={{ phase: 'downloading', done: 1, total: 4 }} />);
    expect(screen.getByRole('progressbar', { name: 'Example' })).toHaveAttribute(
      'aria-valuenow',
      '25',
    );
    expect(screen.getByText('Downloading… 25%')).toBeInTheDocument();
  });

  it('is indeterminate without a total', () => {
    render(<ProgressBar label="Example" progress={null} />);
    expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  });
});

describe('ErrorBanner', () => {
  it('is announced as an alert and can be dismissed', async () => {
    const onDismiss = vi.fn();
    render(<ErrorBanner message="It broke" onDismiss={onDismiss} />);
    expect(screen.getByRole('alert')).toHaveTextContent('It broke');
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(onDismiss).toHaveBeenCalled();
  });
});

describe('ConfirmDialog', () => {
  it('confirms and cancels', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(
      <ConfirmDialog
        open
        title="Replace?"
        message="Really?"
        confirmLabel="Replace"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />,
    );
    expect(screen.getByRole('dialog', { name: 'Replace?' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Replace' }));
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});

describe('Switch', () => {
  it('reports the new state', async () => {
    const onChange = vi.fn();
    render(<Switch checked label="Enabled" onChange={onChange} />);
    const toggle = screen.getByRole('switch', { name: 'Enabled' });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    expect(onChange).toHaveBeenCalledWith(false);
  });
});
