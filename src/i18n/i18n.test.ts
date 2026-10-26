import { t } from './index';

describe('t', () => {
  it('picks plural forms and fills variables', () => {
    expect(t('{count} plugin', { count: 1 })).toBe('1 plugin');
    expect(t('{count} plugin', { count: 3 })).toBe('3 plugins');
    expect(t('Hello {name}', { name: 'Ana' })).toBe('Hello Ana');
  });

  it('shows unknown source strings as written', () => {
    expect(t('Not in the catalog')).toBe('Not in the catalog');
  });

  it('uses the singular for one file, name, song and album', () => {
    expect(
      t('Copied {count} files ({replaced} replaced), skipped {skipped} identical.', {
        count: 1,
        replaced: 0,
        skipped: 0,
      }),
    ).toBe('Copied 1 file (0 replaced), skipped 0 identical.');
    expect(t('{count} files will not be copied:', { count: 1 })).toBe('1 file will not be copied:');
    expect(t('{count} names change because FAT does not allow them:', { count: 1 })).toBe(
      '1 name changes because FAT does not allow it:',
    );
    expect(t('{count} songs, library generation {generation}.', { count: 1, generation: 2 })).toBe(
      '1 song, library generation 2.',
    );
    expect(t('Album art checked for {count} albums.', { count: 1 })).toBe(
      'Album art checked for 1 album.',
    );
    expect(
      t('{count} files, {size}; {free} free on the player.', {
        count: 1,
        size: '1 KB',
        free: '2 GB',
      }),
    ).toBe('1 file, 1 KB; 2 GB free on the player.');
  });
});
