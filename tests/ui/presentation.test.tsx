// @vitest-environment jsdom
import { useState } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { WeekBoard } from '../../app/features/week/WeekBoard';
import { NextActionStrip } from '../../app/components/NextActionStrip';
import { StatusLabel, type StatusState } from '../../app/components/StatusLabel';
import { ErrorNotice } from '../../app/components/ErrorNotice';
import { Dialog } from '../../app/components/Dialog';
import { boardFixture, mealFixture } from '../fixtures/ui/board';

afterEach(cleanup);

function BoardHarness() {
  const [selected, select] = useState(0);
  return <WeekBoard heading="Synthetic presentation board" days={boardFixture}
    selectedSlotId={boardFixture[selected]!.slotId} selectedMeal={mealFixture(selected)}
    onSelectSlot={id => select(boardFixture.findIndex(day => day.slotId === id))}
    nextAction={{ title: 'Choose the open meal', action: { label: 'Choose Sunday dinner', onAction: () => select(6) } }} />;
}

describe('seven-slip presentation', () => {
  it('keeps all seven dates and one stable detail region when selection changes', () => {
    render(<BoardHarness />);
    const board = screen.getByRole('list', { name: 'Seven day meal board' });
    expect(within(board).getAllByRole('listitem')).toHaveLength(7);
    expect(within(board).getAllByRole('button')).toHaveLength(7);
    expect(board.textContent).toContain('Saturday');
    expect(board.textContent).toContain('Sunday');
    const details = screen.getByRole('region', { name: 'Selected meal details' });
    const sunday = within(board).getByRole('button', { name: /Sunday/ });
    sunday.focus();
    fireEvent.click(sunday);
    expect(screen.getByRole('region', { name: 'Selected meal details' })).toBe(details);
    expect(within(details).getByRole('heading', { name: 'Synthetic meal 7' })).toBeTruthy();
    expect(sunday.getAttribute('aria-pressed')).toBe('true');
    expect(document.activeElement).toBe(sunday);
    expect(within(board).getAllByRole('listitem')).toHaveLength(7);
  });

  it('uses native keyboard-focusable buttons with an associated detail region', () => {
    render(<BoardHarness />);
    const button = screen.getByRole('button', { name: /Select Monday/ });
    expect(button.tagName).toBe('BUTTON');
    expect(button.getAttribute('type')).toBe('button');
    expect(button.getAttribute('aria-controls')).toBe(screen.getByRole('region', { name: 'Selected meal details' }).id);
    button.focus();
    expect(document.activeElement).toBe(button);
    // The DOM uses native button activation; browser Enter/Space activation needs WebView verification.
  });

  it.each([1440, 720, 390])('keeps next action then detail then all days in DOM at %i px', width => {
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: width });
    render(<BoardHarness />);
    const action = screen.getByRole('region', { name: 'Next action' });
    const detail = screen.getByRole('region', { name: 'Selected meal details' });
    const board = screen.getByRole('list', { name: 'Seven day meal board' });
    expect(action.compareDocumentPosition(detail) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(detail.compareDocumentPosition(board) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('next open meal action selects the named meal without inventing any producer', () => {
    render(<BoardHarness />);
    fireEvent.click(screen.getByRole('button', { name: 'Choose Sunday dinner' }));
    expect(within(screen.getByRole('region', { name: 'Selected meal details' })).getByRole('heading').textContent).toBe('Synthetic meal 7');
  });

  it('does not present details for a different selected identity as the current meal', () => {
    render(<WeekBoard heading="Synthetic presentation board" days={boardFixture}
      selectedSlotId={boardFixture[1].slotId} selectedMeal={mealFixture(0)} onSelectSlot={vi.fn()}
      nextAction={{ title: 'Read-only example', blockingReason: 'No producer is connected.' }} />);
    const details = screen.getByRole('region', { name: 'Selected meal details' });
    expect(within(details).getByRole('heading', { name: 'Choose a meal' })).toBeTruthy();
    expect(details.textContent).not.toContain('Synthetic meal 1');
  });
});

describe('shared status and recovery', () => {
  it.each<StatusState>(['not-started', 'in-progress', 'booked', 'loading', 'empty', 'disabled', 'blocked', 'failed', 'cancelled', 'uncertain'])('renders %s as text, independently of colour', state => {
    const { container } = render(<StatusLabel state={state} />);
    expect(container.textContent?.trim().length).toBeGreaterThan(1);
    expect(container.querySelector('[aria-hidden="true"]')).toBeTruthy();
    expect(container.querySelector('[data-state]')?.getAttribute('data-state')).toBe(state);
  });

  it('keeps a blocking reason and permitted recovery together', () => {
    const recover = vi.fn();
    render(<NextActionStrip title="Outcome uncertain" blockingReason="Saved state must be recovered before another change."
      action={{ label: 'Recover the same command', onAction: recover }} />);
    expect(screen.getByText('Saved state must be recovered before another change.')).toBeTruthy();
    const button = screen.getByRole('button', { name: 'Recover the same command' });
    expect(button.hasAttribute('disabled')).toBe(false);
    fireEvent.click(button);
    expect(recover).toHaveBeenCalledTimes(1);
  });

  it('shows a precise blocking reason when no action exists', () => {
    render(<NextActionStrip title="Saved state is unavailable" blockingReason="Close and restart the development app." />);
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.getByText('Close and restart the development app.')).toBeTruthy();
  });

  it('announces an error with associated recovery copy and no injected markup', () => {
    render(<ErrorNotice title="Could not read state" message="<script>unsafe()</script>" recovery={{ label: 'Refresh', onAction: vi.fn() }} />);
    const alert = screen.getByRole('alert');
    const recovery = within(alert).getByRole('button', { name: 'Refresh' });
    expect(document.getElementById(recovery.getAttribute('aria-describedby')!)?.textContent).toBe('<script>unsafe()</script>');
    expect(alert.querySelector('script')).toBeNull();
  });
});

describe('dialog keyboard focus', () => {
  // jsdom has no modal top layer. These doubles test our focus lifecycle only.
  beforeEach(() => {
    HTMLDialogElement.prototype.showModal = function () { this.setAttribute('open', ''); };
    HTMLDialogElement.prototype.close = function () { this.removeAttribute('open'); };
  });

  function DialogHarness() {
    const [open, setOpen] = useState(false);
    return <><button onClick={() => setOpen(true)}>Open details</button>
      <Dialog open={open} title="Synthetic details" onClose={() => setOpen(false)}>
        <button>First action</button><button>Last action</button>
      </Dialog></>;
  }

  it('focuses the dialog, wraps keyboard focus, closes on Escape and restores its trigger', () => {
    render(<DialogHarness />);
    const trigger = screen.getByRole('button', { name: 'Open details' });
    trigger.focus();
    fireEvent.click(trigger);
    const dialog = screen.getByRole('dialog', { name: 'Synthetic details' });
    const first = within(dialog).getByRole('button', { name: 'First action' });
    const close = within(dialog).getByRole('button', { name: 'Close' });
    expect(document.activeElement).toBe(first);
    close.focus();
    fireEvent.keyDown(close, { key: 'Tab' });
    expect(document.activeElement).toBe(first);
    fireEvent.keyDown(first, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(close);
    fireEvent.keyDown(close, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it('does not steal focus when an open dialog rerenders', () => {
    const close = vi.fn();
    const { rerender } = render(<Dialog open title="Details" onClose={close}><button>Keep focus</button></Dialog>);
    const button = screen.getByRole('button', { name: 'Close' });
    button.focus();
    rerender(<Dialog open title="Changed details" onClose={() => close()}><button>Keep focus</button></Dialog>);
    expect(document.activeElement).toBe(button);
  });

  it('restores focus after a native cancel event', () => {
    render(<DialogHarness />);
    const trigger = screen.getByRole('button', { name: 'Open details' });
    trigger.focus();
    fireEvent.click(trigger);
    fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true, bubbles: false }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});
