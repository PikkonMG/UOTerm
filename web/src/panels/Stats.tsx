import type { Send, StatsData } from './types';

/** The network statistics or the debug window: lines of words in their color; a double click shows more or less. */
export function Stats({ data, send }: { data: StatsData; send: Send }) {
  return (
    <pre class="stats small number" title={data.hint} style={{ color: data.words.color }} onDblClick={() => send({ double: true })}>
      {data.words.words}
    </pre>
  );
}
