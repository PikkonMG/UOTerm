import { useEffect, useState } from 'preact/hooks';
import type { LoginReply } from '../net/login';
import { leave, play, remove, type CharacterList, type LoginWords } from './login_state';

interface CharactersProps {
  /** The characters of the account by slot; an empty name is a free slot. */
  names: string[];
  /** The words of the shard's last refusal. */
  refused: string | null;
  /** The note New character gives when the account has no room, or null when it has room. */
  noRoom: string | null;
  /** A new list of the view, which keeps the Delete that asks once more. */
  newList(): CharacterList;
  words: LoginWords;
  onReply(reply: LoginReply): void;
  onMake(): void;
}

/**
 * The characters of the account, one row a slot: a click plays one,
 * Delete asks once more before it deletes, New character makes one when
 * the account has room, and Leave ends the login with none. The view
 * keeps the rules; the list is freed once the screen is gone.
 */
export function Characters({ names, refused, noRoom, newList, words, onReply, onMake }: CharactersProps) {
  const [list] = useState(newList);
  useEffect(() => () => list.free(), [list]);
  const [, setDeleteAsked] = useState<number | undefined>(undefined);
  const [note, setNote] = useState<string | null>(null);
  const slots = Array.from({ length: words.character_slots }, (_, slot) => names[slot] ?? '');

  const pressDelete = (slot: number) => {
    if (list.pressDelete(slot)) onReply(remove(slot));
    setDeleteAsked(list.deleteAsked());
  };
  const begin = () => {
    if (noRoom === null) onMake();
    else setNote(noRoom);
  };
  const fault = note ?? refused;

  return (
    <section class="panel screen characters">
      <h1 class="title">{words.pick_character}</h1>
      {slots.map((name, slot) => (
        <div class="slot-row" key={slot}>
          <button type="button" class="button row" disabled={!name} onClick={() => name && onReply(play(slot))}>
            {name || words.empty_slot}
          </button>
          {name && (
            <button type="button" class="button alarm" onClick={() => pressDelete(slot)}>
              {list.deleteWords(slot)}
            </button>
          )}
        </div>
      ))}
      <div class="button-row">
        <button type="button" class="button goal" onClick={begin}>
          {words.make}
        </button>
        <button type="button" class="button" onClick={() => onReply(leave())}>
          {words.leave}
        </button>
      </div>
      {fault && (
        <p class="fault" role="alert">
          {fault}
        </p>
      )}
    </section>
  );
}
