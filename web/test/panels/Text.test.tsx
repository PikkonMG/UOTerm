import { render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Board } from '../../src/panels/Board';
import { Entry } from '../../src/panels/Entry';
import { Journal } from '../../src/panels/Journal';
import { OldMenu } from '../../src/panels/OldMenu';
import { Shop } from '../../src/panels/Shop';
import { Plates } from '../../src/panels/Plates';
import { Tooltip } from '../../src/panels/Tooltip';
import type { JournalData } from '../../src/panels/types';

/** Words a shard may send that look like markup. */
const MARKUP = '<b>x</b>';

describe('shard words', () => {
  it('show_as_text_in_a_tooltip_a_journal_line_and_a_plate', () => {
    const tip = render(<Tooltip data={{ lines: [MARKUP], footer: '' }} at={{ x: 0, y: 0 }} />);
    expect(tip.container.querySelector('b')).toBeNull();
    expect(tip.container.textContent).toContain(MARKUP);
    const journal: JournalData = {
      tabs: [],
      kinds: [],
      new_tab: '+',
      new_tab_hint: '',
      tab_hint: '',
      tab_name_hint: '',
      rename: '',
      delete_tab: '',
      search: '',
      search_hint: '',
      save: '',
      note: null,
      filters: [],
      lines: [{ stamp: null, name: MARKUP, text: MARKUP, color: 'var(--text)' }],
      no_lines: null,
      back: null,
      glass: 'var(--glass)',
      glass_opacity: 1,
    };
    const lines = render(<Journal data={journal} send={vi.fn()} />);
    expect(lines.container.querySelector('b')).toBeNull();
    expect(lines.container.textContent).toContain(MARKUP);
    const plate = {
      area: { min: { x: 0, y: 0 }, max: { x: 10, y: 10 } },
      name_at: { x: 5, y: 0 },
      name: MARKUP,
      name_color: [255, 255, 255, 255] as [number, number, number, number],
      bar: null,
    };
    const plates = render(<Plates plates={[plate]} floats={[{ words: MARKUP, x: 0, y: 0, color: [255, 255, 255, 255], alpha: 1, number: false }]} />);
    expect(plates.container.querySelector('b')).toBeNull();
    expect(plates.container.textContent).toBe(`${MARKUP}${MARKUP}`);
  });

  it('show_as_text_in_the_windows_of_the_shard', () => {
    const shown = [
      render(
        <Shop
          data={{
            live: true,
            goods: [{ serial: 1, picture: null, name: MARKUP, left: 'x1', price: '1 gp', count: { words: '0', color: 'var(--text)' }, hover: { serial: 1, words: MARKUP, footer: '' } }],
            total: '',
            gold: null,
            deal: 'Buy',
            clear: 'Clear',
            close: 'Close',
            step_down: '-',
            step_up: '+',
          }}
          send={vi.fn()}
        />,
      ),
      render(<OldMenu data={{ live: true, entries: [{ picture: null, name: MARKUP }], cancel: null }} send={vi.fn()} />),
      render(<Entry data={{ live: false, description: MARKUP, hint: '', words: '', focus: false, okay: null, cancel: null, take_control: null }} send={vi.fn()} />),
      render(
        <Board
          data={{
            live: false,
            posts: [{ serial: 1, subject: MARKUP, poster: MARKUP, indent: 0, reading: true }],
            text: MARKUP,
            subject: '',
            body: '',
            subject_hint: '',
            text_hint: '',
            post: '',
            reply: null,
            remove: null,
            close: '',
            paper: '',
            ink: '',
          }}
          send={vi.fn()}
        />,
      ),
    ];
    for (const view of shown) {
      expect(view.container.querySelector('b')).toBeNull();
      expect(view.container.textContent).toContain(MARKUP);
    }
  });
});
