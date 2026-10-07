import { useId, type ReactNode } from 'react';
import { NextActionStrip, type NextActionStripProps } from '../../components/NextActionStrip';
import { SelectedMealRegion } from '../meal/SelectedMealRegion';
import type { SelectedMealView } from '../meal/MealDetails';
import { DaySlip, type DaySlipView } from './DaySlip';

export type { DaySlipView } from './DaySlip';
export type SevenDays = readonly [DaySlipView, DaySlipView, DaySlipView, DaySlipView, DaySlipView, DaySlipView, DaySlipView];

export interface WeekBoardProps {
  heading: string;
  days: SevenDays;
  selectedSlotId: string | null;
  selectedMeal: SelectedMealView | null;
  onSelectSlot: (slotId: string) => void;
  nextAction: NextActionStripProps;
  mealActions?: ReactNode;
}

/** A controlled presentation only. The caller supplies dates, status and selected identity. */
export function WeekBoard({ heading, days, selectedSlotId, selectedMeal, onSelectSlot, nextAction, mealActions }: WeekBoardProps) {
  const detailRegionId = useId();
  const headingId = useId();
  return <section className="week-board" aria-labelledby={headingId}>
    <h1 id={headingId}>{heading}</h1>
    <NextActionStrip {...nextAction} />
    <div className="week-worktop">
      {/* Keep reading/focus order stable at every width; CSS only places the wide columns. */}
      <SelectedMealRegion id={detailRegionId} meal={selectedMeal?.slotId === selectedSlotId ? selectedMeal : null}>{mealActions}</SelectedMealRegion>
      <ol className="day-slips" aria-label="Seven day meal board">
        {days.map(day => <DaySlip key={day.slotId} day={day} selected={day.slotId === selectedSlotId}
          detailRegionId={detailRegionId} onSelect={onSelectSlot} />)}
      </ol>
    </div>
  </section>;
}
