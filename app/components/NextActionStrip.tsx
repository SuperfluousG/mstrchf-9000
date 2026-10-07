import { useId } from 'react';

export interface PresentationAction {
  label: string;
  onAction: () => void;
}

export type NextActionStripProps = {
  title: string;
  description?: string;
} & (
  | { action: PresentationAction; blockingReason?: string }
  | { action?: undefined; blockingReason: string }
);

/** Resolve domain eligibility upstream. A strip has a permitted action or a reason. */
export function NextActionStrip({ title, description, action, blockingReason }: NextActionStripProps) {
  const descriptionId = useId();
  return <section className="next-action-strip" aria-label="Next action">
    <div>
      <h2>{title}</h2>
      <div id={descriptionId}>
        {description && <p>{description}</p>}
        {blockingReason && <p className="next-action-reason">{blockingReason}</p>}
      </div>
    </div>
    {action && <button type="button" className="button-primary" onClick={action.onAction}
      aria-describedby={description || blockingReason ? descriptionId : undefined}>{action.label}</button>}
  </section>;
}
