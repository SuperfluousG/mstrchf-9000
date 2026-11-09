import { useId } from 'react';

/** The MSTRCHF 9000 mark: a gold-rimmed stoneware bowl seen from above, which also reads as a red machine eye. */
export function Mark({ className }: { className?: string }) {
  const glaze = `glaze-${useId().replace(/:/g, '')}`;
  return (
    <svg className={className} viewBox="0 0 64 64" aria-hidden="true" focusable="false">
      <defs>
        <radialGradient id={glaze} cx="54%" cy="56%" r="70%">
          <stop offset="0" stopColor="#2A1517" />
          <stop offset="0.56" stopColor="#4A1F2B" />
          <stop offset="1" stopColor="#9B1B26" />
        </radialGradient>
      </defs>
      <circle cx="32" cy="32" r="28" fill="#D7B461" />
      <circle cx="32" cy="32" r="25" fill={`url(#${glaze})`} />
      <ellipse cx="24.5" cy="23" rx="4" ry="2.5" transform="rotate(-35 24.5 23)" fill="#EEEDE5" opacity="0.46" />
    </svg>
  );
}
