import { ShareBar } from './Rows';
import type { InfoBarData } from './types';

/** The info bar: the items of the Info Bar page in one row, each label and value in its hue, with a bar when the page asks for one. */
export function InfoBar({ data }: { data: InfoBarData }) {
  return (
    <div class="info-bar">
      {data.parts.map((part, at) => (
        <span class="info-part" key={at}>
          <span style={{ color: part.label.color }}>{part.label.words}</span>
          <span class="info-value">
            <span style={{ color: part.words.color }}>{part.words.words}</span>
            {part.fill !== null && <ShareBar fill={part.fill} color={part.bar_color} />}
          </span>
        </span>
      ))}
    </div>
  );
}
