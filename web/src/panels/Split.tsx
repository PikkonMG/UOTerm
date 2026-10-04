import type { Send, SplitData } from './types';

const FEWEST = 1;

/** The box that asks how many of a pile to move, and Move. */
export function Split({ data, send }: { data: SplitData; send: Send }) {
  return (
    <div class="split">
      <div class="points">
        <input type="range" min={FEWEST} max={data.most} value={data.amount} onInput={(event) => send({ amount: Number(event.currentTarget.value) })} />
        <span class="number goal-words">{data.amount}</span>
      </div>
      <button type="button" class="button goal" onClick={() => send({ go: true })}>
        {data.go_words}
      </button>
    </div>
  );
}
