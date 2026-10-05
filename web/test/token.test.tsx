import { render } from 'preact';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Token, TOKEN_AUTOCOMPLETE, TOKEN_FIELD_NAME, WRONG_TOKEN } from '../src/screens/Token';

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = '';
});

/** Shows the screen, types `token` and presses the button. */
function giveOnScreen(token: string, onAccepted: () => void): HTMLElement {
  const root = document.createElement('div');
  document.body.append(root);
  render(<Token onAccepted={onAccepted} />, root);
  const field = root.querySelector('input');
  if (!field) throw new Error('no token field');
  field.value = token;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  root.querySelector('button')?.click();
  return root;
}

describe('Token', () => {
  it('says_wrong_token_when_the_server_refuses_it', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"error":"unauthorized"}', { status: 401 }));
    const onAccepted = vi.fn();
    const root = giveOnScreen('wrong', onAccepted);
    await vi.waitFor(() => expect(root.textContent).toContain(WRONG_TOKEN));
    expect(onAccepted).not.toHaveBeenCalled();
  });

  it('goes_on_when_the_server_takes_it', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 204 }));
    const onAccepted = vi.fn();
    giveOnScreen('right', onAccepted);
    await vi.waitFor(() => expect(onAccepted).toHaveBeenCalled());
    expect(fetchSpy).toHaveBeenCalledWith('/v1/web/token', expect.objectContaining({ body: '{"token":"right"}' }));
  });

  it('a_password_field_hides_the_token', () => {
    const root = document.createElement('div');
    render(<Token onAccepted={() => {}} />, root);
    expect(root.querySelector('input')?.type).toBe('password');
  });

  it('is_not_saved_or_filled_as_the_password_of_the_site', () => {
    const root = document.createElement('div');
    render(<Token onAccepted={() => {}} />, root);
    const field = root.querySelector('input');
    expect(field?.getAttribute('autocomplete')).toBe(TOKEN_AUTOCOMPLETE);
    expect(field?.name).toBe(TOKEN_FIELD_NAME);
    expect(TOKEN_AUTOCOMPLETE).toBe('one-time-code');
  });
});
