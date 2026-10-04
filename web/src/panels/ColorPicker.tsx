import { HuePicker } from './HuePicker';
import type { ColorPickerData, Send } from './types';

/** The color picker a hue row of the Options opened: the grid of hues, the hue it holds, the eyedropper, Okay and Cancel. */
export function ColorPicker({ data, send }: { data: ColorPickerData; send: Send }) {
  return (
    <div class="dye">
      <div class="dye-top">
        <HuePicker data={data.grid} live={true} send={send} />
        <div class="dye-tub">
          <span class="swatch picked-swatch" style={{ background: data.color }} />
          <span class="number small dim">{data.words}</span>
        </div>
      </div>
      <div class="button-row">
        <button type="button" class="button" style={{ color: data.eyedropper.color }} onClick={() => send({ eyedropper: true })}>
          {data.eyedropper.words}
        </button>
        <button type="button" class="button goal" onClick={() => send({ okay: true })}>
          {data.okay}
        </button>
        <button type="button" class="button" onClick={() => send({ cancel: true })}>
          {data.cancel}
        </button>
      </div>
    </div>
  );
}
