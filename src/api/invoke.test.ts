import { invoke } from '@tauri-apps/api/core';

import { AppError, call, errorMessage } from './invoke';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: unknown;
  },
}));

describe('call', () => {
  it('passes the result through', async () => {
    vi.mocked(invoke).mockResolvedValue(42);
    await expect(call<number>('x')).resolves.toBe(42);
  });

  it('turns { code, message } rejections into AppError', async () => {
    vi.mocked(invoke).mockRejectedValue({ code: 'busy', message: 'Another operation' });
    await expect(call('x')).rejects.toMatchObject({ code: 'busy', message: 'Another operation' });
    await expect(call('x')).rejects.toBeInstanceOf(AppError);
  });

  it('wraps anything else', () => {
    expect(errorMessage('boom')).toBe('boom');
  });
});
