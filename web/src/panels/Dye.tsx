import { HuePicker } from './HuePicker';
import { Picture } from './Picture';
import type { DyeData, Send } from './types';

/** The dye panel: the grid of hues, the tub in the picked hue, Okay, and the eyedropper. */
export function Dye({ data, send }: { data: DyeData; send: Send }) {
  return (
    <div class="dye">
      <div class="dye-top">
        <HuePicker data={data.grid} live={data.live} send={send} />
        <div class="dye-tub">
          <span class="cell-art tub-art">
            <Picture picture={data.tub} />
          </span>
          <span class="number small dim">{data.hue}</span>
        </div>
      </div>
      {data.okay && (
        <div class="button-row">
          <button type="button" class="button goal" onClick={() => send({ okay: true })}>
            {data.okay}
          </button>
          {data.eyedropper && (
            <button type="button" class="button" style={{ color: data.eyedropper.color }} onClick={() => send({ eyedropper: true })}>
              {data.eyedropper.words}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
