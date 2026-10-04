import type { QuestionData, Send } from './types';

/** The question that waits for Yes or No: of the guard, or of the Modern style. */
export function Question({ data, send }: { data: QuestionData; send: Send }) {
  return (
    <div class="question">
      <p class={`question-words${data.alarm ? ' alarm-words' : ''}`}>{data.words}</p>
      <div class="button-row question-buttons">
        <button type="button" class="button goal-words" onClick={() => send({ answer: true })}>
          {data.yes}
        </button>
        <button type="button" class="button" onClick={() => send({ answer: false })}>
          {data.no}
        </button>
      </div>
    </div>
  );
}
