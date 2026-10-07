export type StatusState = 'not-started' | 'in-progress' | 'booked' | 'loading' | 'empty' | 'disabled' | 'blocked' | 'failed' | 'cancelled' | 'uncertain';

const labels: Record<StatusState, { text: string; symbol: string }> = {
  'not-started': { text: 'Not started', symbol: '○' },
  'in-progress': { text: 'In progress', symbol: '◐' },
  booked: { text: 'Booked', symbol: '✓' },
  loading: { text: 'Loading', symbol: '…' },
  empty: { text: 'Open meal', symbol: '+' },
  disabled: { text: 'Unavailable', symbol: '—' },
  blocked: { text: 'Needs attention', symbol: '!' },
  failed: { text: 'Failed', symbol: '!' },
  cancelled: { text: 'Cancelled', symbol: '×' },
  uncertain: { text: 'Outcome uncertain', symbol: '?' },
};

export interface StatusLabelProps {
  state: StatusState;
  label?: string;
  announce?: boolean;
}

/** The caller owns the meaning. This component never infers a saved/booked state. */
export function StatusLabel({ state, label, announce = false }: StatusLabelProps) {
  return <span className="status-label" data-state={state} role={announce ? 'status' : undefined}>
    <span aria-hidden="true">{labels[state].symbol}</span>
    <span>{label ?? labels[state].text}</span>
  </span>;
}
