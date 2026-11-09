import { StatusLabel, type StatusState } from '../../components/StatusLabel';

export interface DaySlipView {
  slotId: string;
  weekday: string;
  dateLabel: string;
  dateIso: string;
  mealLabel: string;
  title: string;
  summary: string;
  state: StatusState;
  statusLabel?: string;
  /** Artwork for the exact saved recipe; omitted for open or unillustrated meals. */
  image?: { src: string; alt: string };
}

export function DaySlip({ day, selected, detailRegionId, onSelect }: {
  day: DaySlipView;
  selected: boolean;
  detailRegionId: string;
  onSelect: (slotId: string) => void;
}) {
  return <li className="day-slip" data-selected={selected} data-state={day.state} data-has-image={!!day.image}>
    <div className="day-slip-date"><strong>{day.weekday}</strong><time dateTime={day.dateIso}>{day.dateLabel}</time></div>
    {day.image ? <div className="day-slip-artwork"><img className="day-slip-image" src={day.image.src} alt={day.image.alt} loading="lazy" /></div>
      : <div className="day-slip-placeholder" aria-hidden="true"><span className="day-slip-placeholder-icon">{day.state === 'empty' ? '+' : day.state === 'disabled' ? '—' : '◯'}</span></div>}
    <div className="day-slip-body">
      <p className="day-slip-meal">{day.mealLabel}</p>
      <h3>{day.title}</h3>
      <p>{day.summary}</p>
      <StatusLabel state={day.state} label={day.statusLabel} />
      <button type="button" className="day-slip-select" aria-pressed={selected} aria-controls={detailRegionId}
        aria-label={`Select ${day.weekday} ${day.dateLabel}, ${day.mealLabel}: ${day.title}`}
        onClick={() => onSelect(day.slotId)}><span>{selected ? 'Selected meal' : day.state === 'empty' ? 'Choose meal' : 'View meal'}</span><span className="day-slip-arrow" aria-hidden="true">↗</span></button>
    </div>
  </li>;
}
