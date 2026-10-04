import { Note } from './Note';
import { Picture } from './Picture';
import type { BuildData, Choice, Send } from './types';
import { WishField } from './WishField';

/** The class of a storey's button by how it shows: all shown, some see-through, some hidden. */
const STOREY_LOOKS = ['', ' waiting', ' alarm'];

function Choices({ choices, send, name, rows }: { choices: Choice[]; send: Send; name: string; rows?: boolean }) {
  return (
    <div class={rows ? 'build-styles' : 'button-row'}>
      {choices.map((choice, at) => (
        <button type="button" class={`button${rows ? ' row' : ''}${choice.chosen ? ' chosen' : ''}`} key={at} onClick={() => send({ [name]: at })}>
          {choice.words}
        </button>
      ))}
    </div>
  );
}

/**
 * The house designer: the kinds, the styles of the kind and the pieces of
 * the style; the field Jev picks a part for; and, while the human has
 * control, the steps of the designer, the levels and how each storey
 * shows. The counts of the design sit at the foot.
 */
export function Build({ data, send }: { data: BuildData; send: Send }) {
  const buttons = data.buttons;
  return (
    <div class="build">
      <Choices choices={data.kinds} send={send} name="kind" />
      {data.no_parts ? <p class="faint">{data.no_parts}</p> : <Choices choices={data.styles} send={send} name="style" rows />}
      <div class="build-pieces">
        {data.pieces.map((piece, at) => (
          <button type="button" class={`button build-piece${piece.chosen ? ' chosen' : ''}`} key={at} onClick={() => send({ piece: at })}>
            {piece.picture && <Picture picture={piece.picture} />}
          </button>
        ))}
      </div>
      <WishField hint={data.wish_hint} words={data.find} send={(wish) => send({ wish })} />
      {buttons && (
        <>
          <div class="button-row">
            <button type="button" class={`button${buttons.remove.chosen ? ' alarm' : ''}`} onClick={() => send({ remove: true })}>
              {buttons.remove.words}
            </button>
            {buttons.changes.map((words, at) => (
              <button type="button" class="button" key={words} onClick={() => send({ change: at })}>
                {words}
              </button>
            ))}
          </div>
          <div class="button-row">
            {buttons.kept.map((words, at) => (
              <button type="button" class="button" key={words} onClick={() => send({ keep: at })}>
                {words}
              </button>
            ))}
            <button type="button" class={`button${buttons.pick.chosen ? ' chosen' : ''}`} title={buttons.pick_tip} onClick={() => send({ pick: true })}>
              {buttons.pick.words}
            </button>
          </div>
          <div class="button-row">
            <span class="dim">{buttons.floor_words}</span>
            {buttons.floors.map((floor) => (
              <button type="button" class={`button${floor.chosen ? ' chosen' : ''}`} key={floor.words} onClick={() => send({ floor: Number(floor.words) })}>
                {floor.words}
              </button>
            ))}
          </div>
          <div class="button-row">
            {buttons.storeys.map((storey, at) => (
              <button type="button" class={`button${STOREY_LOOKS[storey.look] ?? ''}`} key={storey.words} title={storey.tip} onClick={() => send({ storey: at })}>
                {storey.words}
              </button>
            ))}
          </div>
        </>
      )}
      <p class="build-counts" title={data.counts_tip ?? undefined}>
        {data.counts.map((count) => (
          <span class={count.alarm ? 'alarm' : undefined} key={count.words}>
            {count.words}
          </span>
        ))}
      </p>
      <Note note={data.note} />
    </div>
  );
}
