import { formatBytes, progressLabel } from './format';

describe('formatBytes', () => {
  it('scales units and keeps small numbers whole', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(64e9)).toBe('59.6 GB');
    expect(formatBytes(150 * 1024 * 1024)).toBe('150 MB');
  });
});

describe('progressLabel', () => {
  it('describes each phase', () => {
    expect(progressLabel({ phase: 'downloading', done: 1, total: 4 })).toBe('Downloading… 25%');
    expect(progressLabel({ phase: 'downloading', done: 0, total: 0 })).toBe('Downloading… 0%');
    expect(progressLabel({ phase: 'updating', done: 1, total: 4 })).toBe('Updating plugins… 25%');
    expect(progressLabel({ phase: 'removing', done: 0, total: 1 })).toBe('Removing…');
  });
});
