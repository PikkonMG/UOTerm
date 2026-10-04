import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Launcher } from '../../src/panels/Launcher';
import { Split } from '../../src/panels/Split';
import { TitleBar } from '../../src/panels/TitleBar';

describe('Launcher', () => {
  it('turns_the_panel_of_a_button_and_marks_the_ones_that_show', () => {
    const send = vi.fn();
    const data = { buttons: [{ words: 'Radar', shows: true }, { words: 'Journal', shows: false }] };
    const { getByText } = render(<Launcher data={data} send={send} />);
    expect(getByText('Radar').classList.contains('chosen')).toBe(true);
    fireEvent.click(getByText('Journal'));
    expect(send).toHaveBeenCalledWith({ toggle: 1 });
  });
});

describe('TitleBar', () => {
  it('puts_the_title_of_the_view_on_the_page', () => {
    render(<TitleBar title="UOTerm watch - Mara [H 50/50 M 0/0 S 0/0]" />);
    expect(document.title).toBe('UOTerm watch - Mara [H 50/50 M 0/0 S 0/0]');
  });
});

describe('Split', () => {
  it('sets_how_many_and_moves_them', () => {
    const send = vi.fn();
    const { getByRole, getByText } = render(<Split data={{ amount: 10, most: 10, go_words: 'Move' }} send={send} />);
    fireEvent.input(getByRole('slider'), { target: { value: '4' } });
    expect(send).toHaveBeenCalledWith({ amount: 4 });
    fireEvent.click(getByText('Move'));
    expect(send).toHaveBeenLastCalledWith({ go: true });
  });
});
