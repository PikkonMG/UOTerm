import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Question } from '../../src/panels/Question';
import { Report } from '../../src/panels/Report';

describe('Question', () => {
  it('answers_yes_or_no', () => {
    const send = vi.fn();
    const data = { words: 'Quit\nUltima Online?', alarm: false, yes: 'Yes', no: 'No', place: { x: 0, y: 0, w: 320, h: 120 }, framed: null };
    const { getByText } = render(<Question data={data} send={send} />);
    fireEvent.click(getByText('Yes'));
    expect(send).toHaveBeenCalledWith({ answer: true });
    fireEvent.click(getByText('No'));
    expect(send).toHaveBeenLastCalledWith({ answer: false });
  });
});

describe('Report', () => {
  it('shows_a_failed_report_in_the_alarm_look', () => {
    const { getByText } = render(<Report data={{ text: 'Too far.', failed: true, at: { x: 100, y: 120 } }} />);
    expect(getByText('Too far.').classList.contains('failed')).toBe(true);
  });
});
