// Placeholder data for every composition. Nothing here comes from a real
// account; names and numbers are invented.

export type Pace = { state: "on" | "reserve" | "deficit"; delta?: number; at: number };

export type Quota = {
  id: string;
  /// Upper-cased by the card.
  title: "session" | "weekly" | string;
  left: number;
  reset: string;
  ticks: number;
  pace?: Pace;
  notStarted?: boolean;
};

export type Layout = "cards" | "lines" | "rings" | "tiles";

export type Section = {
  provider: string;
  name: string;
  plan?: string;
  account?: string;
  badge?: { text: string; color: string };
  layout: Layout;
  quotas: Quota[];
  resets?: { count: number; expires: string };
  credit?: string;
};

export type Strings = {
  usage: string;
  today: string;
  yesterday: string;
  session: string;
  weekly: string;
  onPace: string;
  left: (v: number) => string;
  useReset: string;
  /// Fallback when the full label does not fit (use-reset-short).
  useResetShort: string;
  resets: (n: number) => string;
  notStarted: string;
};

const en: Strings = {
  usage: "Usage",
  today: "Today",
  yesterday: "Yesterday",
  session: "5h session",
  weekly: "Weekly",
  onPace: "On pace",
  left: (v) => `${v}% left`,
  useReset: "Use reset",
  useResetShort: "Use",
  resets: (n) => `${n} Reset${n === 1 ? "" : "s"}`,
  notStarted: "Not started",
};

// From locales/*/app.ftl.
export const STRINGS: Record<string, { label: string; s: Strings }> = {
  en: { label: "English", s: en },
  ru: {
    label: "Русский",
    s: {
      usage: "Расход",
      today: "Сегодня",
      yesterday: "Вчера",
      session: "Сессия 5 ч",
      weekly: "Неделя",
      onPace: "В норме",
      left: (v) => `Осталось ${v}%`,
      useReset: "Использовать сброс",
      useResetShort: "Использовать",
      resets: (n) => `${n} ${n === 1 ? "сброс" : n < 5 ? "сброса" : "сбросов"}`,
      notStarted: "Не начата",
    },
  },
  de: {
    label: "Deutsch",
    s: {
      usage: "Nutzung",
      today: "Heute",
      yesterday: "Gestern",
      session: "5-Stunden-Sitzung",
      weekly: "Wöchentlich",
      onPace: "Im Soll",
      left: (v) => `${v}% verbleibend`,
      useReset: "Zurücksetzung nutzen",
      useResetShort: "Nutzen",
      resets: (n) => `${n} Reset${n === 1 ? "" : "s"}`,
      notStarted: "Nicht gestartet",
    },
  },
  fr: {
    label: "Français",
    s: {
      usage: "Utilisation",
      today: "Aujourd'hui",
      yesterday: "Hier",
      session: "Session de 5 h",
      weekly: "Hebdomadaire",
      onPace: "Au bon rythme",
      left: (v) => `${v} % restants`,
      useReset: "Utiliser une réinitialisation",
      useResetShort: "Utiliser",
      resets: (n) => `${n} réinitialisation${n === 1 ? "" : "s"}`,
      notStarted: "Non démarrée",
    },
  },
  es: {
    label: "Español",
    s: {
      usage: "Uso",
      today: "Hoy",
      yesterday: "Ayer",
      session: "Sesión de 5h",
      weekly: "Semanal",
      onPace: "Al ritmo previsto",
      left: (v) => `${v}% restante`,
      useReset: "Usar reinicio",
      useResetShort: "Usar",
      resets: (n) => `${n} restablecimiento${n === 1 ? "" : "s"}`,
      notStarted: "No iniciada",
    },
  },
  "pt-BR": {
    label: "Português",
    s: {
      usage: "Uso",
      today: "Hoje",
      yesterday: "Ontem",
      session: "Sessão de 5h",
      weekly: "Semanal",
      onPace: "No ritmo",
      left: (v) => `${v}% restante`,
      useReset: "Usar redefinição",
      useResetShort: "Usar",
      resets: (n) => `${n} ${n === 1 ? "redefinição" : "redefinições"}`,
      notStarted: "Não iniciada",
    },
  },
  ja: {
    label: "日本語",
    s: {
      usage: "使用量",
      today: "今日",
      yesterday: "昨日",
      session: "5時間セッション",
      weekly: "毎週",
      onPace: "適正ペース",
      left: (v) => `残り ${v}%`,
      useReset: "リセットを使用",
      useResetShort: "使用",
      resets: (n) => `${n} 回のリセット`,
      notStarted: "未開始",
    },
  },
  "zh-CN": {
    label: "简体中文",
    s: {
      usage: "用量",
      today: "今天",
      yesterday: "昨天",
      session: "5 小时会话",
      weekly: "每周",
      onPace: "进度正常",
      left: (v) => `剩余 ${v}%`,
      useReset: "使用重置",
      useResetShort: "使用",
      resets: (n) => `${n} 次重置`,
      notStarted: "尚未开始",
    },
  },
};

export const sections = (layouts: Partial<Record<string, Layout>> = {}): Section[] => [
  {
    provider: "claude",
    name: "Claude",
    plan: "Max",
    account: "Personal",
    badge: { text: "P", color: "#E07A3F" },
    layout: layouts.claude ?? "cards",
    quotas: [
      { id: "c5", title: "session", left: 64, reset: "2h 41m", ticks: 4, pace: { state: "reserve", delta: 12, at: 46 } },
      { id: "cw", title: "weekly", left: 38, reset: "3d 7h", ticks: 6, pace: { state: "on", at: 58 } },
    ],
    resets: { count: 1, expires: "11d 4h" },
  },
  {
    provider: "codex",
    name: "Codex",
    plan: "Pro",
    account: "Truman-fan",
    layout: layouts.codex ?? "rings",
    quotas: [
      { id: "x5", title: "session", left: 82, reset: "4h 12m", ticks: 4, pace: { state: "reserve", delta: 9, at: 20 } },
      { id: "xw", title: "weekly", left: 57, reset: "5d 2h", ticks: 6, pace: { state: "on", at: 30 } },
    ],
    resets: { count: 2, expires: "18d 6h" },
  },
  {
    provider: "cursor",
    name: "Cursor",
    plan: "Pro",
    layout: layouts.cursor ?? "lines",
    quotas: [
      { id: "u1", title: "Cursor models", left: 73, reset: "21d 9h", ticks: 0, pace: { state: "on", at: 30 } },
      { id: "u2", title: "Other models", left: 91, reset: "21d 9h", ticks: 0, pace: { state: "reserve", delta: 4, at: 30 } },
    ],
  },
  {
    provider: "claude",
    name: "Claude",
    plan: "Team",
    account: "Work",
    badge: { text: "W", color: "#2F7BEA" },
    layout: layouts.claudeWork ?? "rings",
    quotas: [
      { id: "w5", title: "session", left: 100, reset: "", ticks: 4, notStarted: true },
      { id: "ww", title: "weekly", left: 71, reset: "5d 2h", ticks: 6, pace: { state: "reserve", delta: 6, at: 18 } },
    ],
  },
];

/// Home usage donut: spend per account for the selected day.
export const SPEND = [
  { provider: "claude", name: "Claude · Personal", badge: { text: "P", color: "#E07A3F" }, amount: 38.42, color: "#F29E66" },
  { provider: "codex", name: "Codex", amount: 17.9, color: "rgb(128,159,255)" },
  { provider: "cursor", name: "Cursor", amount: 6.25, color: "#ffffff" },
  { provider: "claude", name: "Claude · Work", badge: { text: "W", color: "#2F7BEA" }, amount: 2.11, color: "#B8603F" },
];

/// Usage tab cost series for 30 days, deterministic.
export const COST_SERIES = (() => {
  const series: { claude: number; codex: number; cursor: number }[] = [];
  let seed = 7;
  const rand = () => {
    seed = (seed * 16807) % 2147483647;
    return seed / 2147483647;
  };
  for (let day = 0; day < 30; day++) {
    const weekday = day % 7 < 5 ? 1 : 0.35;
    series.push({
      claude: (18 + 40 * rand()) * weekday + (day > 18 ? 22 : 0) * rand(),
      codex: (8 + 22 * rand()) * weekday,
      cursor: (2 + 9 * rand()) * weekday,
    });
  }
  return series;
})();
