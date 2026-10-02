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
}

export function DaySlip({ day, selected, detailRegionId, onSelect }: {
  day: DaySlipView;
  selected: boolean;
  detailRegionId: string;
  onSelect: (slotId: string) => void;
}) {
  return <li className="day-slip" data-selected={selected} data-state={day.state}>
    <div className="day-slip-date"><strong>{day.weekday}</strong><time dateTime={day.dateIso}>{day.dateLabel}</time></div>
    <div className="day-slip-body">
      <p className="day-slip-meal">{day.mealLabel}</p>
      <h3>{day.title}</h3>
      <p>{day.summary}</p>
      <StatusLabel state={day.state} label={day.statusLabel} />
      <button type="button" className="day-slip-select" aria-pressed={selected} aria-controls={detailRegionId}
        aria-label={`Select ${day.weekday} ${day.dateLabel}, ${day.mealLabel}: ${day.title}`}
        onClick={() => onSelect(day.slotId)}>{selected ? 'Selected meal' : 'View meal'}</button>
    </div>
  </li>;
}
