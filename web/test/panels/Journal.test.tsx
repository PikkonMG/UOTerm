import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Journal } from '../../src/panels/Journal';
import type { JournalData } from '../../src/panels/types';

const data: JournalData = {
  tabs: [
    { name: 'All', chosen: true, kinds: [true, false] },
    { name: 'Chat', chosen: false, kinds: [true, true] },
  ],
  kinds: ['Speech', 'System'],
  new_tab: '+',
  new_tab_hint: 'Add a tab.',
  tab_hint: 'Right-click: rename, kinds of lines, delete.',
  tab_name_hint: 'tab name',
  rename: 'Rename',
  delete_tab: 'Delete tab',
  search: '',
  search_hint: 'search the journal',
  save: 'Save',
  note: null,
  filters: [{ words: 'Shard', shown: true }],
  lines: [{ stamp: '12:30', name: 'Bob', text: 'hail', color: 'var(--text)' }],
  no_lines: null,
  back: null,
  glass: 'var(--glass)',
  glass_opacity: 1,
};

describe('Journal', () => {
  it('shows_each_line_with_its_time_and_speaker', () => {
    const { getByText } = render(<Journal data={data} send={vi.fn()} />);
    expect(getByText('hail').style.color).toBe('var(--text)');
    expect(getByText('12:30')).toBeTruthy();
    expect(getByText('Bob:')).toBeTruthy();
  });

  it('turns_tabs_and_filters_searches_and_saves', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText } = render(<Journal data={data} send={send} />);
    fireEvent.click(getByText('Chat'));
    expect(send).toHaveBeenCalledWith({ tab: 1 });
    fireEvent.click(getByText('Shard'));
    expect(send).toHaveBeenCalledWith({ filter: 0 });
    fireEvent.input(getByPlaceholderText('search the journal'), { target: { value: 'orc' } });
    expect(send).toHaveBeenCalledWith({ search: 'orc' });
    fireEvent.click(getByText('Save'));
    expect(send).toHaveBeenLastCalledWith({ save: true });
  });

  it('adds_a_tab_by_name_and_its_menu_renames_flips_a_kind_and_deletes', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText } = render(<Journal data={data} send={send} />);
    fireEvent.click(getByText('+'));
    const name = getByPlaceholderText('tab name');
    fireEvent.input(name, { target: { value: 'Mine' } });
    fireEvent.keyDown(name, { key: 'Enter' });
    expect(send).toHaveBeenCalledWith({ new_tab: 'Mine' });
    fireEvent.contextMenu(getByText('Chat'));
    fireEvent.click(getByText('System'));
    expect(send).toHaveBeenCalledWith({ kind: { tab: 1, kind: 1 } });
    fireEvent.click(getByText('Delete tab'));
    expect(send).toHaveBeenLastCalledWith({ delete_tab: 1 });
  });

  it('reads_back_with_the_wheel', () => {
    const send = vi.fn();
    const { container } = render(<Journal data={data} send={send} />);
    fireEvent.wheel(container.querySelector('.journal-lines') as HTMLElement, { deltaY: -100, deltaMode: 0 });
    expect(send).toHaveBeenLastCalledWith({ wheel: 1 });
  });
});
