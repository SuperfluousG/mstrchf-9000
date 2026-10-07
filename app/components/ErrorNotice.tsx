import { useId } from 'react';
import type { PresentationAction } from './NextActionStrip';

export interface ErrorNoticeProps {
  id?: string;
  title: string;
  message: string;
  recovery?: PresentationAction;
}

export function ErrorNotice({ id, title, message, recovery }: ErrorNoticeProps) {
  const generatedId = useId();
  const messageId = `${id ?? generatedId}-message`;
  return <div id={id} className="error-notice" role="alert">
    <strong>{title}</strong>
    <p id={messageId}>{message}</p>
    {recovery && <button type="button" onClick={recovery.onAction} aria-describedby={messageId}>{recovery.label}</button>}
  </div>;
}
