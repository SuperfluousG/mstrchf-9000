import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from 'react';

export interface DialogProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
}

function focusable(dialog: HTMLDialogElement): HTMLElement[] {
  return Array.from(dialog.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [tabindex]'))
    .filter(element => !element.matches(':disabled, [hidden], [type="hidden"]') && element.tabIndex >= 0
      && !element.closest('[hidden], [inert]') && getComputedStyle(element).display !== 'none'
      && getComputedStyle(element).visibility !== 'hidden');
}

/** Native modal top layer plus explicit keyboard/focus lifecycle. */
export function Dialog({ open, title, onClose, children }: DialogProps) {
  const titleId = useId();
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    if (!open) return;
    const dialog = dialogRef.current!;
    const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.showModal();
    (focusable(dialog)[0] ?? dialog).focus();
    return () => {
      dialog.close();
      if (trigger?.isConnected) trigger.focus();
    };
  }, [open]);

  function handleKeyDown(event: KeyboardEvent<HTMLDialogElement>) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== 'Tab') return;
    const items = focusable(event.currentTarget);
    const first = items[0];
    const last = items.at(-1);
    if (!first || !last) {
      event.preventDefault();
      event.currentTarget.focus();
    } else if (event.shiftKey && (document.activeElement === first || document.activeElement === event.currentTarget)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === event.currentTarget)) {
      event.preventDefault();
      first.focus();
    }
  }

  return <dialog ref={dialogRef} className="meal-dialog" aria-labelledby={titleId} tabIndex={-1}
    onCancel={event => { event.preventDefault(); onClose(); }} onKeyDown={handleKeyDown}>
    <h2 id={titleId}>{title}</h2>
    {children}
    <div className="dialog-actions"><button type="button" onClick={onClose}>Close</button></div>
  </dialog>;
}
