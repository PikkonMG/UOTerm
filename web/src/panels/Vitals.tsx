import { DetailRow, ShareBar } from './Rows';
import type { VitalsData } from './types';

/** The name, the states as chips, the hits, mana and stamina bars, and the fight. */
export function Vitals({ data }: { data: VitalsData }) {
  return (
    <div class="rows">
      <h2 class="title" style={{ color: data.name.color }}>
        {data.name.words}
      </h2>
      {data.states.length > 0 && (
        <div class="chips">
          {data.states.map((state, at) => (
            <span class="state-chip" key={at} style={{ color: state.color }}>
              {state.words}
            </span>
          ))}
        </div>
      )}
      {data.bars.map((bar, at) => (
        <div class={`vital${bar.main ? ' main' : ''}`} key={at}>
          <span class="dim small">{bar.label}</span>
          <ShareBar fill={bar.fill} ghost={bar.ghost} color={bar.color} main={bar.main} />
          <span class="number" style={{ color: bar.numbers.color }}>
            {bar.numbers.words}
          </span>
        </div>
      ))}
      {data.fights && <DetailRow detail={data.fights} />}
    </div>
  );
}
