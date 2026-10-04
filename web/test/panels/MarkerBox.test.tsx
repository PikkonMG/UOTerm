import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { MarkerBox } from '../../src/panels/MarkerBox';
import type { MarkerBoxData } from '../../src/panels/types';

const data: MarkerBoxData = {
  x: '1000',
  y: '1000',
  name: 'MarkerName',
  icon: '',
  color: 3,
  colors: ['none', 'red', 'green', 'blue'],
  labels: ['X', 'Y', 'Name', 'Icon', 'Color'],
  icon_hint: 'no icon',
  error: 'Give a name, and x and y inside the facet.',
  submit: 'Create',
  cancel: 'Cancel',
};

describe('MarkerBox', () => {
  it('sends_the_fields_as_typed_and_creates_or_cancels', () => {
    const send = vi.fn();
    const { getByLabelText, getByText } = render(<MarkerBox data={data} send={send} />);
    fireEvent.input(getByLabelText('Name'), { target: { value: 'Home' } });
    expect(send).toHaveBeenLastCalledWith({ fields: { x: '1000', y: '1000', name: 'Home', icon: '', color: 3 } });
    fireEvent.change(getByLabelText('Color'), { target: { value: '1' } });
    expect(send).toHaveBeenLastCalledWith({ fields: { x: '1000', y: '1000', name: 'MarkerName', icon: '', color: 1 } });
    fireEvent.keyDown(getByLabelText('X'), { key: 'Enter' });
    expect(send).toHaveBeenLastCalledWith({ submit: true });
    fireEvent.click(getByText('Create'));
    expect(send).toHaveBeenLastCalledWith({ submit: true });
    fireEvent.click(getByText('Cancel'));
    expect(send).toHaveBeenLastCalledWith({ cancel: true });
    expect(getByText(data.error as string)).toBeTruthy();
  });
});
