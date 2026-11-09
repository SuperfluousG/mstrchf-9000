import { useEffect, useRef, type ReactNode } from 'react';

/** Modal side sheet built on <dialog>: focus handling and Escape come from the browser. */
export function Sheet({ open, onClose, label, children, wide = false }: { open: boolean; onClose: () => void; label: string; children: ReactNode; wide?: boolean }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    if (!open && d.open) d.close();
  }, [open]);
  return (
    <dialog
      ref={ref}
      className={`sheet${wide ? ' wide' : ''}`}
      aria-label={label}
      onClose={onClose}
      onClick={e => { if (e.target === ref.current) onClose(); }}
    >
      <div className="sheet-body">
        <button type="button" className="sheet-close" onClick={onClose} aria-label="Close">×</button>
        {open && children}
      </div>
    </dialog>
  );
}
