import { useRef, useState } from 'preact/hooks';
import { giveToken, wordsOf } from '../net/api';

export const WRONG_TOKEN = 'Wrong token.';
/** The name of the token field: not a password of the site, so the browser keeps it apart from them. */
export const TOKEN_FIELD_NAME = 'uoterm-token';
/** The browser neither saves the token as the password of the site nor fills one in. */
export const TOKEN_AUTOCOMPLETE = 'one-time-code';

/** Asks for the token of this UOTerm, and gives it to the API. */
export function Token({ onAccepted }: { onAccepted: () => void }) {
  const field = useRef<HTMLInputElement>(null);
  const [words, setWords] = useState('');
  const [busy, setBusy] = useState(false);

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    setBusy(true);
    try {
      if (await giveToken(field.current?.value ?? '')) onAccepted();
      else setWords(WRONG_TOKEN);
    } catch (error) {
      setWords(wordsOf(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form class="panel screen" onSubmit={submit}>
      <h1 class="title">UOTerm</h1>
      <label class="label" for="token">
        Token
      </label>
      <input id="token" name={TOKEN_FIELD_NAME} class="field" type="password" ref={field} autocomplete={TOKEN_AUTOCOMPLETE} autofocus />
      <button class="button" type="submit" disabled={busy}>
        Go on
      </button>
      {words && (
        <p class="fault" role="alert">
          {words}
        </p>
      )}
    </form>
  );
}
