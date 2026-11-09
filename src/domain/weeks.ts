const DAY = 86_400_000;
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
export const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

function iso(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function parse(id: string): Date {
  const [y, m, d] = id.split('-').map(Number);
  return new Date(y, m - 1, d);
}

/** ISO date (local) of the Monday starting the week containing `d`. */
export function mondayOf(d: Date): string {
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  x.setDate(x.getDate() - ((x.getDay() + 6) % 7));
  return iso(x);
}

export function addWeeks(id: string, n: number): string {
  const d = parse(id);
  d.setDate(d.getDate() + 7 * n);
  return iso(d);
}

export function weeksBetween(a: string, b: string): number {
  return Math.round((parse(b).getTime() - parse(a).getTime()) / (7 * DAY));
}

/** "16–22 Nov" or "30 Nov – 6 Dec". */
export function weekLabel(id: string): string {
  const s = parse(id);
  const e = new Date(s); e.setDate(e.getDate() + 6);
  return s.getMonth() === e.getMonth()
    ? `${s.getDate()}–${e.getDate()} ${MONTHS[e.getMonth()]}`
    : `${s.getDate()} ${MONTHS[s.getMonth()]} – ${e.getDate()} ${MONTHS[e.getMonth()]}`;
}

export function dayDate(id: string, day: number): string {
  const d = parse(id);
  d.setDate(d.getDate() + day);
  return `${d.getDate()} ${MONTHS[d.getMonth()]}`;
}
