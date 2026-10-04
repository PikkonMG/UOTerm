import { ShareBar } from './Rows';
import type { CastData, CooldownsData } from './types';

/** The cooldown bars that run: each label, the seconds left, and the bar in the hue of its rule. */
export function Cooldowns({ data }: { data: CooldownsData }) {
  return (
    <div class="cooldowns">
      {data.bars.map((bar, at) => (
        <div class="cooldown" key={at}>
          <p class="detail small">
            <span>{bar.label}</span>
            <span class="dim number">{bar.seconds}</span>
          </p>
          <ShareBar fill={bar.fill} color={bar.color} />
        </div>
      ))}
    </div>
  );
}

/** The spell being cast, in the color of what it does: its name, who stands in range, and how far the cast is. */
export function Cast({ data }: { data: CastData }) {
  return (
    <div class="cast">
      <p class="detail">
        <span style={{ color: data.name.color }}>{data.name.words}</span>
        <span class="dim small">{data.aside}</span>
      </p>
      <ShareBar fill={data.fill} color={data.name.color} />
    </div>
  );
}
