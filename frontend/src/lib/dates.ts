const DAY_LABELS = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];
const MONTH_NAMES = [
  "janvier", "février", "mars", "avril", "mai", "juin",
  "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

export function getMonday(date: Date): Date {
  const result = new Date(date);
  const day = result.getDay(); // 0 = dimanche
  const diff = day === 0 ? -6 : 1 - day;
  result.setDate(result.getDate() + diff);
  result.setHours(0, 0, 0, 0);
  return result;
}

export function addDays(date: Date, amount: number): Date {
  const result = new Date(date);
  result.setDate(result.getDate() + amount);
  return result;
}

export function toISODate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function getWeekDays(monday: Date): { date: Date; iso: string; label: string }[] {
  return Array.from({ length: 7 }, (_, i) => {
    const date = addDays(monday, i);
    return { date, iso: toISODate(date), label: `${DAY_LABELS[i]} ${date.getDate()}` };
  });
}

export function formatWeekRange(monday: Date): string {
  const sunday = addDays(monday, 6);
  const sameMonth = monday.getMonth() === sunday.getMonth();
  const sameYear = monday.getFullYear() === sunday.getFullYear();

  if (sameMonth && sameYear) {
    return `${monday.getDate()} – ${sunday.getDate()} ${MONTH_NAMES[monday.getMonth()]} ${monday.getFullYear()}`;
  }
  if (sameYear) {
    return `${monday.getDate()} ${MONTH_NAMES[monday.getMonth()]} – ${sunday.getDate()} ${MONTH_NAMES[sunday.getMonth()]} ${sunday.getFullYear()}`;
  }
  return `${monday.getDate()} ${MONTH_NAMES[monday.getMonth()]} ${monday.getFullYear()} – ${sunday.getDate()} ${MONTH_NAMES[sunday.getMonth()]} ${sunday.getFullYear()}`;
}
