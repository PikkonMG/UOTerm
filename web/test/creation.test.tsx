import { render } from 'preact';
import { act } from 'preact/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Creation } from '../src/screens/Creation';
import type { CreationModel } from '../src/screens/creation_model';
import { CREATE, CREATION_WORDS as WORDS, GOOD, namePageView, NEXT, SHORT } from './fake_creation';

afterEach(() => {
  document.body.innerHTML = '';
});

function show(view: CreationModel, onFinish = vi.fn(), onLeave = vi.fn()): HTMLElement {
  const root = document.createElement('div');
  document.body.append(root);
  act(() => render(<Creation model={view} words={WORDS} onFinish={onFinish} onLeave={onLeave} />, root));
  return root;
}

function button(root: HTMLElement, words: string): HTMLButtonElement {
  const found = [...root.querySelectorAll('button')].find((each) => each.textContent === words);
  if (!found) throw new Error(`no button ${words}`);
  return found;
}

function typeName(root: HTMLElement, name: string) {
  const field = root.querySelector<HTMLInputElement>('input.name-field');
  if (!field) throw new Error('no name field');
  act(() => {
    field.value = name;
    field.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

describe('Creation', () => {
  it('shows_why_a_name_is_refused_and_keeps_create_off', () => {
    const view = namePageView();
    const root = show(view);
    typeName(root, 'M');
    expect(view.setName).toHaveBeenCalledWith('M');
    expect(root.textContent).toContain(SHORT);
    expect(button(root, CREATE).disabled).toBe(true);
  });

  it('makes_the_character_once_the_name_is_good', () => {
    const view = namePageView();
    const onFinish = vi.fn();
    const root = show(view, onFinish);
    typeName(root, 'Mara');
    expect(root.textContent).toContain(GOOD);
    act(() => button(root, CREATE).click());
    expect(onFinish).toHaveBeenCalledOnce();
  });

  it('goes_back_to_the_list_from_the_first_page', () => {
    const view = namePageView();
    view.back = vi.fn(() => true);
    const onLeave = vi.fn();
    const root = show(view, vi.fn(), onLeave);
    act(() => button(root, WORDS.back).click());
    expect(onLeave).toHaveBeenCalledOnce();
  });

  it('shows_next_on_a_page_that_is_not_the_last', () => {
    const view = namePageView();
    const screen = view.screen;
    view.screen = () => ({ ...screen(), footer: { words: WORDS.keys, blocked: false, next: NEXT } });
    const root = show(view);
    expect(button(root, NEXT).disabled).toBe(false);
  });
});
