import { useState } from 'preact/hooks';

const ENTER = 'Enter';

/**
 * A field that takes plain words, and the button that asks Jev with them.
 * The words wait here until Enter or the button sends them.
 */
export function WishField({ hint, words, send }: { hint: string; words: string; send: (wish: string) => void }) {
  const [wish, setWish] = useState('');
  const ask = () => {
    if (wish.trim()) send(wish);
  };
  return (
    <div class="wish-row">
      <input
        class="field"
        placeholder={hint}
        value={wish}
        onInput={(event) => setWish(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === ENTER) ask();
        }}
      />
      <button type="button" class="button goal" onClick={ask}>
        {words}
      </button>
    </div>
  );
}
