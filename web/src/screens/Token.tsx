import { useRef, useState } from 'preact/hooks';
import { giveToken } from '../net/api';

export const WRONG_TOKEN = 'Wrong token.';

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
      setWords(error instanceof Error ? error.message : String(error));
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
      <input id="token" class="field" type="password" ref={field} autocomplete="current-password" autofocus />
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
