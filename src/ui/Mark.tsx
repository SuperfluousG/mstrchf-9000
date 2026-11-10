/** Countercut: an angular M with a fine diagonal cut, proposed for the UI review. */
export function Mark({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 64 64" aria-hidden="true" focusable="false">
      <path fill="#E2C779" d="M6 56V8h11l15 22L47 8h11v48H45V31L32 49 19 31v25Z" />
      <path fill="#4A1F2B" d="m12 8 33 48h5L17 8Z" />
      <path fill="#EEEDE5" d="m40 18 7-10h11v7H47l-4 6Z" />
    </svg>
  );
}
