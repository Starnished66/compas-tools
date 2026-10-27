import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { openUrl } from '@tauri-apps/plugin-opener';

import { aboutNotices } from '../../api/about';
import { AboutScreen } from './AboutScreen';

vi.mock('../../api/about');
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));

describe('AboutScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('shows the licence texts that ship with the app', () => {
    const { container } = render(<AboutScreen />);
    expect(screen.getByText(/released under the GNU General Public License/)).toBeInTheDocument();
    expect(screen.getByText('Third-party material')).toBeInTheDocument();
    const text = container.textContent;
    expect(text).toContain('GNU GENERAL PUBLIC LICENSE');
    expect(text).toContain('SIL OPEN FONT LICENSE');
    expect(text).toContain('Breeze icons');
  });

  it('reads the dependency notices only when they are opened', async () => {
    vi.mocked(aboutNotices).mockResolvedValue('react 18.3.1 (npm, MIT)');
    render(<AboutScreen />);
    expect(aboutNotices).not.toHaveBeenCalled();
    await userEvent.click(screen.getByText('Libraries inside the app'));
    expect(await screen.findByText(/react 18\.3\.1/)).toBeInTheDocument();
  });

  it('says so when the build has no notices', async () => {
    vi.mocked(aboutNotices).mockResolvedValue(null);
    render(<AboutScreen />);
    await userEvent.click(screen.getByText('Libraries inside the app'));
    expect(await screen.findByText(/not in this build/)).toBeInTheDocument();
  });

  it('opens the Ko-fi page in the system browser', async () => {
    render(<AboutScreen />);
    await userEvent.click(screen.getByRole('link', { name: 'Support the project on Ko-fi' }));
    expect(openUrl).toHaveBeenCalledWith('https://ko-fi.com/starnished');
  });
});
