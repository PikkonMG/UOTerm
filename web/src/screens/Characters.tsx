import { useState } from 'preact/hooks';
import type { LoginReply } from '../net/login';
import { leave, play, remove, type LoginWords } from './login_state';

interface CharactersProps {
  /** The characters of the account by slot; an empty name is a free slot. */
  names: string[];
  /** The words of the shard's last refusal. */
  refused: string | null;
  /** The account has room for one more character. */
  room: boolean;
  words: LoginWords;
  onReply(reply: LoginReply): void;
  onMake(): void;
}

/**
 * The characters of the account, one row a slot: a click plays one,
 * Delete asks once more before it deletes, New character makes one when
 * the account has room, and Leave ends the login with none.
 */
export function Characters({ names, refused, room, words, onReply, onMake }: CharactersProps) {
  const [deleteAsked, setDeleteAsked] = useState<number | null>(null);
  const [noRoom, setNoRoom] = useState(false);
  const slots = Array.from({ length: words.character_slots }, (_, slot) => names[slot] ?? '');

  const askDelete = (slot: number) => {
    if (deleteAsked === slot) onReply(remove(slot));
    else setDeleteAsked(slot);
  };
  const begin = () => {
    if (room) onMake();
    else setNoRoom(true);
  };

  return (
    <section class="panel screen characters">
      <h1 class="title">{words.pick_character}</h1>
      {slots.map((name, slot) => (
        <div class="slot-row" key={slot}>
          <button type="button" class="button row" disabled={!name} onClick={() => name && onReply(play(slot))}>
            {name || words.empty_slot}
          </button>
          {name && (
            <button type="button" class="button alarm" onClick={() => askDelete(slot)}>
              {deleteAsked === slot ? words.delete_sure : words.delete}
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
      {(noRoom || refused) && (
        <p class="fault" role="alert">
          {noRoom ? words.no_room : refused}
        </p>
      )}
    </section>
  );
}
