import type { ReportData } from './types';

/** The words of the last act under the control bar, in the alarm look when it failed. */
export function Report({ data }: { data: ReportData }) {
  return (
    <p class={`report shadowed${data.failed ? ' failed' : ''}`} style={{ left: `${data.at.x}px`, top: `${data.at.y}px` }}>
      {data.text}
    </p>
  );
}
