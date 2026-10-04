import { render } from '@testing-library/preact';
import { describe, expect, it } from 'vitest';
import { Activity } from '../../src/panels/Activity';
import { Pack } from '../../src/panels/Pack';
import { Vitals } from '../../src/panels/Vitals';

describe('Activity', () => {
  it('shows_the_goal_and_its_details_or_the_idle_words', () => {
    const { getByText } = render(
      <Activity data={{ heading: 'Loot', idle: null, details: [{ label: 'Job', value: { words: 'skin, walk', color: 'var(--text)' } }] }} />,
    );
    expect(getByText('Loot')).toBeTruthy();
    expect(getByText('skin, walk').style.color).toBe('var(--text)');
  });
});

describe('Vitals', () => {
  it('draws_each_bar_at_its_share_with_its_ghost', () => {
    const data = {
      name: { words: 'Mara', color: 'var(--noto-self)' },
      states: [{ words: 'Poisoned', color: 'var(--hits-poisoned)' }],
      bars: [{ label: 'Hits', numbers: { words: '25/50', color: 'var(--text)' }, fill: 0.5, ghost: 0.75, color: 'var(--hits)', main: true }],
      fights: null,
    };
    const { getByText, container } = render(<Vitals data={data} />);
    expect(getByText('Poisoned')).toBeTruthy();
    expect(getByText('25/50')).toBeTruthy();
    const fill = container.querySelector('.bar-fill') as HTMLElement;
    expect(fill.style.width).toBe('50%');
    expect((container.querySelector('.bar-ghost') as HTMLElement).style.width).toBe('75%');
  });
});

describe('Pack', () => {
  it('shows_the_gold_and_each_row', () => {
    const data = {
      heading: 'Pack',
      gold: { words: '120', color: 'var(--noto-self)' },
      gold_words: 'gold',
      rows: [{ label: 'Weight', value: { words: '40/400 stones', color: 'var(--text)' } }],
    };
    const { getByText } = render(<Pack data={data} />);
    expect(getByText('120')).toBeTruthy();
    expect(getByText('40/400 stones')).toBeTruthy();
  });
});
