import { entry } from '../../test/fixtures';
import { categoriesOf, countUpdates, inView, matches } from './filters';

describe('inView', () => {
  const layout = entry({ player_layout: true });
  const theme = entry({ theme_pack: true });
  const plain = entry();

  it('splits one catalog into plugins, layouts and themes', () => {
    expect([plain, layout, theme].filter((e) => inView(e, 'plugins'))).toEqual([plain]);
    expect([plain, layout, theme].filter((e) => inView(e, 'layouts'))).toEqual([layout]);
    expect([plain, layout, theme].filter((e) => inView(e, 'themes'))).toEqual([theme]);
  });
});

describe('matches', () => {
  const e = entry({ name: 'A-B Repeat', description: 'Practice looper', category: 'Audio' });

  it('searches name, description and others, ignoring case', () => {
    expect(matches(e, 'repeat', '')).toBe(true);
    expect(matches(e, 'LOOPER', '')).toBe(true);
    expect(matches(e, 'zzz', '')).toBe(false);
  });

  it('filters by category', () => {
    expect(matches(e, '', 'Audio')).toBe(true);
    expect(matches(e, '', 'Themes')).toBe(false);
  });
});

describe('helpers', () => {
  it('lists distinct sorted categories and counts updates', () => {
    const list = [
      entry({ category: 'B' }),
      entry({ id: 'x', category: 'A', state: 'update' }),
      entry({ id: 'y', category: 'B' }),
      entry({ id: 'z', category: '' }),
    ];
    expect(categoriesOf(list)).toEqual(['A', 'B']);
    expect(countUpdates(list)).toBe(1);
  });
});
