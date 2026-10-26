import { render, screen } from '@testing-library/react';

import { AboutScreen } from './AboutScreen';

describe('AboutScreen', () => {
  it('shows the licence texts that ship with the app', () => {
    const { container } = render(<AboutScreen />);
    expect(screen.getByText(/released under the GNU General Public License/)).toBeInTheDocument();
    expect(screen.getByText('Third-party material')).toBeInTheDocument();
    const text = container.textContent;
    expect(text).toContain('GNU GENERAL PUBLIC LICENSE');
    expect(text).toContain('SIL OPEN FONT LICENSE');
    expect(text).toContain('Breeze icons');
  });
});
