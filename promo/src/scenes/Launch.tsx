// Launch / promo video, cut to "Electro Dreams" by Arulo (Mixkit, 120 BPM).
// Every scene is one 8-beat phrase; the build-up names the providers, the
// drop opens the app and the end card lands on the breakdown.
import React from "react";
import {
  AbsoluteFill,
  Audio,
  Img,
  Sequence,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { FontGate } from "../ui/Desktop";
import { Icon, PROVIDER_ICON, ProviderMark, Segmented, T, cardStyle } from "../ui/kit";
import { DEFAULT_TABS, Footer, HomePage, POPUP_WIDTH, PopupFrame, Tab } from "../ui/Popup";
import { QuotaGroup, ResetRow } from "../ui/cards";
import { UsagePage } from "../ui/Usage";
import { ToastStack, Timed } from "../ui/Toast";
import { BRAND, ORANGE, Palette, SEGOE, dracula, fluent, solarized, vercel } from "../ui/palette";
import { COST_SERIES, Quota, STRINGS, Section, sections } from "../data";
import { notificationTimeline } from "./Notifications";
import { LANGS, Offset, Speed, Usage } from "./PopupScenes";

/* ------------------------------------------------------------------ tempo */

const BPM = 120;
/// One beat in seconds.
const B = 60 / BPM;
/// Position of the track's first beat.
const TRACK_PHASE = 0.24;
/// Track beat at video beat 0: the build-up, so the bass drops on video
/// beat DROP and the breakdown starts on BREAKDOWN.
const TRACK_START = 24;
const DROP = 8;
/// Showcase phrases after the drop, 8 beats each.
const OPENROUTER_AT = DROP + 16;
const ACTIVATION_AT = DROP + 24;
const NOTIFICATIONS_AT = DROP + 32;
/// The end card starts as the music falls into its breakdown.
const BREAKDOWN = DROP + 72;
const END = BREAKDOWN + 8;
/// The track has 64 full-energy beats and the showcase needs 72: at this
/// video beat the music jumps back one phrase, so the breakdown still lands
/// on BREAKDOWN.
const MUSIC_LOOP_AT = DROP + 48;
const MUSIC_LOOP_BACK = 8;

export const LAUNCH_SECONDS = END * B;

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;
const ease = {
  outExpo: (t: number) => (t >= 1 ? 1 : 1 - Math.pow(2, -10 * Math.max(0, t))),
  inCubic: (t: number) => Math.pow(Math.min(1, Math.max(0, t)), 3),
  inOut: (t: number) => {
    const x = Math.min(1, Math.max(0, t));
    return x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2;
  },
};
const lerp = (a: number, b: number, k: number) => a + (b - a) * k;

const useSeconds = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return frame / fps;
};

/// 1 on every beat, decaying quickly: drives kick-synced pulses.
const pulse = (t: number, from = 0, decay = 0.14) => {
  const local = t - from * B;
  if (local < 0) return 0;
  return Math.exp(-(local % B) / decay);
};

/// Spring that starts at a beat (in scene seconds).
const useSpringAt = (
  seconds: number,
  config: { damping?: number; stiffness?: number; mass?: number } = { damping: 14, stiffness: 170, mass: 0.7 },
) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const offset = React.useContext(Offset);
  const value = spring({ frame: frame - Math.round((seconds + offset) * fps), fps, config });
  // A spring only approaches its target; land on it so settled layouts stop
  // moving by fractions of a pixel, which makes text snap back and forth.
  return Math.abs(1 - value) < 0.002 ? 1 : value;
};

/* -------------------------------------------------------------- providers */

const PROVIDERS = [
  { id: "codex", name: "Codex" },
  { id: "claude", name: "Claude" },
  { id: "cursor", name: "Cursor" },
  { id: "openrouter", name: "OpenRouter" },
  { id: "antigravity", name: "Antigravity" },
  { id: "grok", name: "Grok" },
  { id: "kiro", name: "Kiro" },
  { id: "opencode", name: "OpenCode Zen" },
  { id: "opencode", name: "OpenCode Go" },
];
const color = (id: string) => BRAND[id].dark;

/* ------------------------------------------------------------- background */

/// Bloom wallpaper whose glow follows the beat and an optional tint.
const Backdrop: React.FC<{ t: number; tint?: string; energy: number }> = ({ t, tint, energy }) => {
  const kick = pulse(t) * energy;
  const blobs = [
    { x: 18, y: 30, s: 1000, c: "#3a2bd6", o: 0.55 },
    { x: 78, y: 78, s: 1100, c: "#d2643c", o: 0.45 },
    { x: 88, y: 12, s: 760, c: "#7a2bb8", o: 0.42 },
    { x: 32, y: 92, s: 860, c: "#0f6a8a", o: 0.4 },
  ];
  return (
    <AbsoluteFill style={{ background: "linear-gradient(135deg, #070a16 0%, #0e0e1f 45%, #150b18 100%)", overflow: "hidden" }}>
      {blobs.map((b, i) => (
        <div
          key={i}
          style={{
            position: "absolute",
            left: `calc(${b.x}% - ${b.s / 2}px + ${Math.sin(t * 0.55 + i * 1.7) * 160}px)`,
            top: `calc(${b.y}% - ${b.s / 2}px + ${Math.cos(t * 0.45 + i * 2.3) * 120}px)`,
            width: b.s,
            height: b.s,
            borderRadius: "50%",
            background: b.c,
            opacity: b.o * (0.9 + kick * 0.1),
            filter: "blur(150px)",
          }}
        />
      ))}
      {tint && (
        <div
          style={{
            position: "absolute",
            inset: 0,
            background: `radial-gradient(circle at 50% 46%, ${tint} 0%, transparent 55%)`,
            opacity: 0.16 + kick * 0.05,
            mixBlendMode: "screen",
          }}
        />
      )}
      {/* Perspective grid floor that scrolls with time. */}
      <div style={{ position: "absolute", left: -400, right: -400, bottom: -40, height: 520, perspective: 600, opacity: 0.22 }}>
        <div
          style={{
            position: "absolute",
            inset: 0,
            transform: "rotateX(72deg)",
            transformOrigin: "50% 100%",
            backgroundImage:
              "linear-gradient(rgba(255,255,255,0.5) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.5) 1px, transparent 1px)",
            backgroundSize: "80px 80px",
            backgroundPosition: `0px ${(t * 160) % 80}px`,
            maskImage: "linear-gradient(to top, black, transparent)",
            WebkitMaskImage: "linear-gradient(to top, black, transparent)",
          }}
        />
      </div>
    </AbsoluteFill>
  );
};

/// Full-frame flash for cuts and drops.
const Flash: React.FC<{ t: number; at: number; strength?: number; tint?: string }> = ({ t, at, strength = 1, tint = "#fff" }) => {
  const k = t - at;
  if (k < 0 || k > 0.5) return null;
  return (
    <AbsoluteFill
      style={{
        background: `radial-gradient(circle at 50% 50%, ${tint} 0%, transparent 70%)`,
        opacity: Math.exp(-k / 0.09) * 0.75 * strength,
        mixBlendMode: "screen",
        pointerEvents: "none",
      }}
    />
  );
};

/// Decaying camera shake after a hit.
const shake = (t: number, at: number, amount = 18) => {
  const k = t - at;
  if (k < 0 || k > 0.6) return { x: 0, y: 0, r: 0 };
  const d = Math.exp(-k / 0.12) * amount;
  return { x: Math.sin(k * 90) * d, y: Math.cos(k * 77) * d * 0.7, r: Math.sin(k * 60) * d * 0.05 };
};

/* ---------------------------------------------------------------- caption */

type Line = { text: string; beat: number; accent?: string[]; size?: number; dim?: boolean };

/// Big kinetic headline: each line slams in on its beat, word by word.
const Headline: React.FC<{ lines: Line[]; t: number; side: 1 | -1; exit?: number }> = ({ lines, t, side, exit = 0 }) => (
  <div
    style={{
      position: "absolute",
      top: 0,
      bottom: 0,
      left: side === 1 ? 130 : 1030,
      width: 780,
      display: "flex",
      flexDirection: "column",
      justifyContent: "center",
      transform: `translateX(${-side * exit * 400}px)`,
      opacity: 1 - exit,
      filter: exit > 0.02 ? `blur(${exit * 16}px)` : undefined,
    }}
  >
    {lines.map((line, li) => {
      const size = line.size ?? 96;
      return (
        <div
          key={li}
          style={{
            fontFamily: SEGOE,
            fontWeight: line.dim ? 500 : 800,
            fontSize: size,
            lineHeight: 1.04,
            letterSpacing: line.dim ? -0.5 : -3,
            color: line.dim ? "rgba(255,255,255,0.7)" : "#fff",
            marginTop: line.dim ? 22 : 0,
            whiteSpace: "nowrap",
          }}
        >
          {line.text.split(" ").map((word, wi) => (
            <Word key={wi} word={word} at={line.beat * B + wi * 0.045} t={t} accent={line.accent?.includes(word)} />
          ))}
        </div>
      );
    })}
  </div>
);

const Word: React.FC<{ word: string; at: number; t: number; accent?: boolean }> = ({ word, at, t, accent }) => {
  const k = useSpringAt(at, { damping: 13, stiffness: 210, mass: 0.6 });
  if (t < at) return null;
  return (
    <span
      style={{
        display: "inline-block",
        marginRight: "0.24em",
        color: accent ? ORANGE : undefined,
        opacity: Math.min(1, k * 2),
        transform: `translateY(${(1 - k) * 60}px) scale(${lerp(1.5, 1, Math.min(1, k))})`,
        transformOrigin: "50% 80%",
        filter: k < 0.8 ? `blur(${(1 - k) * 12}px)` : undefined,
        
      }}
    >
      {word}
    </span>
  );
};

/* ----------------------------------------------------------------- camera */

type Key = { beat: number; s: number; x: number; y: number; r?: number };

/// Places a `width`×`height` clip so its focus point sits on the anchor,
/// easing between keyframes (beats are scene-local) and punching on kicks.
const Camera: React.FC<{
  t: number;
  width: number;
  height: number;
  keys: Key[];
  side: 1 | -1;
  punch?: number;
  children: React.ReactNode;
}> = ({ t, width, height, keys, side, punch = 0.006, children }) => {
  const beat = t / B;
  const i = Math.max(0, keys.filter((key) => key.beat <= beat).length - 1);
  const a = keys[i];
  const b = keys[Math.min(i + 1, keys.length - 1)];
  const k = a === b ? 0 : ease.inOut((beat - a.beat) / (b.beat - a.beat));
  const s = lerp(a.s, b.s, k) * (1 + pulse(t) * punch);
  const x = lerp(a.x, b.x, k);
  const y = lerp(a.y, b.y, k);
  const r = lerp(a.r ?? 0, b.r ?? 0, k);
  const anchor = side === 1 ? 1320 : 600;
  // Keep the clip turned toward the caption and swinging slowly in 3D.
  const turn = -side * (9 + 5 * Math.sin(t * 1.3));
  const tilt = 3 * Math.sin(t * 0.9 + 1);
  return (
    <div
      style={{
        position: "absolute",
        left: anchor - x,
        top: 540 - y,
        width,
        height,
        transform: `perspective(2000px) rotateY(${turn}deg) rotateX(${tilt}deg) scale(${s}) rotate(${r}deg)`,
        transformOrigin: `${x}px ${y}px`,
      }}
    >
      {children}
    </div>
  );
};

/* ----------------------------------------------------------------- phrase */

/// One 8-beat scene: whips in from its side and out to the other.
const Phrase: React.FC<{
  start: number;
  beats?: number;
  side: 1 | -1;
  /// Appear in place (on a drop) instead of whipping in.
  slam?: boolean;
  children: (t: number, exit: number) => React.ReactNode;
}> = ({ start, beats = 8, side, slam, children }) => {
  const { fps } = useVideoConfig();
  const pre = slam ? 0 : 0.12;
  const post = 0.16;
  return (
    <Sequence from={Math.round((start * B - pre) * fps)} durationInFrames={Math.round((beats * B + pre + post) * fps)}>
      <Offset.Provider value={pre}>
        <PhraseBody beats={beats} side={side} pre={pre} slam={slam}>
          {children}
        </PhraseBody>
      </Offset.Provider>
    </Sequence>
  );
};

const PhraseBody: React.FC<{
  beats: number;
  side: 1 | -1;
  pre: number;
  slam?: boolean;
  children: (t: number, exit: number) => React.ReactNode;
}> = ({ beats, side, pre, slam, children }) => {
  const t = useSeconds() - pre;
  const enter = slam ? 1 : ease.outExpo((t + pre) / 0.34);
  const exit = ease.inCubic((t - (beats * B - 0.1)) / 0.26);
  const blur = (1 - enter) * 22 + exit * 22;
  return (
    <AbsoluteFill
      style={{
        transform: `translateX(${(1 - enter) * side * 1100 - exit * side * 1100}px) scale(${lerp(1.12, 1, enter) + exit * 0.08})`,
        filter: blur > 0.3 ? `blur(${blur}px)` : undefined,
        opacity: Math.min(1, enter * 3) * (1 - exit * 0.6),
      }}
    >
      {children(t, exit)}
    </AbsoluteFill>
  );
};

/* ------------------------------------------------------------------- hook */

/// Beats 0..8: every provider flashes in, then shrinks into a 3×3 wall that
/// collapses into the app icon on the drop.
const FLASH_BEATS = [0, 1, 2, 2.5, 3, 3.5, 4, 4.5, 5];
/// The last name leaves and the wall line appears.
const WALL_AT = 5.5;

const Hook: React.FC<{ t: number }> = ({ t }) => {
  const beat = t / B;
  const current = FLASH_BEATS.filter((b) => b <= beat).length - 1;
  const collapse = ease.inCubic((beat - (DROP - 1.4)) / 1.4);
  const wallIn = useSpringAt(WALL_AT * B, { damping: 16, stiffness: 120, mass: 0.8 });
  return (
    <AbsoluteFill>
      {PROVIDERS.map((provider, i) => {
        const at = FLASH_BEATS[i] * B;
        if (t < at) return null;
        const next = (FLASH_BEATS[i + 1] ?? WALL_AT) * B;
        // Big in the middle while current, then flies to its wall slot.
        const hit = Math.min(1, (t - at) / 0.16);
        const fly = ease.outExpo((t - next) / 0.4);
        const slotX = 960 + ((i % 3) - 1) * 200;
        const slotY = 470 + (Math.floor(i / 3) - 1) * 200;
        const big = 300 * lerp(1.35, 1, ease.outExpo(hit));
        const size = lerp(big, 140, fly) * (1 - collapse * 0.75) * (1 + pulse(t, 0, 0.1) * 0.04 * (1 - fly));
        const cx = lerp(lerp(960, slotX, fly), 960, collapse);
        const cy = lerp(lerp(440, slotY, fly), 470, collapse);
        const tilt = (1 - fly) * (i % 2 ? 6 : -6) * (1 - hit);
        const wallSpin = fly * (1 - collapse) * Math.sin(t * 2 + i) * 6;
        return (
          <div
            key={i}
            style={{
              position: "absolute",
              left: cx - size / 2,
              top: cy - size / 2,
              width: size,
              height: size,
              borderRadius: size * 0.24,
              background: `linear-gradient(145deg, rgba(255,255,255,${0.12 + fly * 0.04}), rgba(255,255,255,0.03))`,
              border: `1px solid rgba(255,255,255,${0.18})`,
              boxShadow: `0 ${size * 0.12}px ${size * 0.35}px rgba(0,0,0,0.45)`,
              backdropFilter: "blur(20px)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              transform: `rotate(${tilt + wallSpin}deg)`,
              opacity: 1 - ease.inCubic((beat - (DROP - 0.5)) / 0.5),
            }}
          >
            <Icon name={PROVIDER_ICON[provider.id]} size={size * 0.5} color={color(provider.id)} />
          </div>
        );
      })}
      {/* Name of the provider currently on screen. */}
      {current >= 0 && beat < WALL_AT && (
        <ProviderName key={current} name={PROVIDERS[current].name} at={FLASH_BEATS[current] * B} t={t} />
      )}
      {beat >= WALL_AT && (
        <div
          style={{
            position: "absolute",
            left: 0,
            right: 0,
            top: 790,
            textAlign: "center",
            fontFamily: SEGOE,
            fontWeight: 800,
            fontSize: 84,
            letterSpacing: -2.5,
            color: "#fff",
            opacity: wallIn * (1 - collapse),
            transform: `translateY(${(1 - wallIn) * 60}px) scale(${1 - collapse * 0.3})`,
          }}
        >
          All your AI limits<span style={{ color: ORANGE }}>.</span>
        </div>
      )}
    </AbsoluteFill>
  );
};

const ProviderName: React.FC<{ name: string; at: number; t: number }> = ({ name, at, t }) => {
  const k = useSpringAt(at, { damping: 12, stiffness: 240, mass: 0.6 });
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        top: 660,
        textAlign: "center",
        fontFamily: SEGOE,
        fontWeight: 800,
        fontSize: 120,
        letterSpacing: -4,
        color: "#fff",
        opacity: Math.min(1, k * 2),
        transform: `translateY(${(1 - k) * 80}px) scale(${lerp(1.3, 1, Math.min(1, k))}) skewX(${(1 - k) * -10}deg)`,
        filter: k < 0.8 ? `blur(${(1 - k) * 14}px)` : undefined,
      }}
    >
      {name}
    </div>
  );
};

/* -------------------------------------------------------------- drop: app */

/// Token series for the Claude tab chart: input, cache and output per day.
const TOKEN_DAYS = COST_SERIES.map((day, i) => ({
  input: day.claude * 0.55,
  cache: day.claude * 0.3,
  output: day.claude * 0.15 + (i % 3) * 0.6,
}));
const INPUT = "#9A9CF2";
const OUTPUT = "#5FD3A7";

/// A two-row card with its own right column (credits and similar).
const CreditCard: React.FC<{ p: Palette }> = ({ p }) => (
  <div style={{ ...cardStyle(p), height: 58, overflow: "hidden" }}>
    <div style={{ position: "absolute", inset: 0, width: "47%", background: `color-mix(in srgb, ${p.accent} 20%, transparent)` }} />
    <div style={{ position: "relative", height: "100%", padding: "0 12px", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
      <div>
        <T size={12} color={p.textSecondary}>
          CLOUD SESSION CREDITS
        </T>
        <T size={14} color={p.accent} weight={600}>
          $47.12 of $100.00 left
        </T>
      </div>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 2 }}>
        <T size={12} color={p.textSecondary}>
          Nov 5, 10:59
        </T>
        <div style={{ display: "flex", gap: 4 }}>
          <T size={12} color={p.textTertiary}>
            Expires in
          </T>
          <T size={12} color={p.textPrimary} weight={600}>
            24d 10h
          </T>
        </div>
      </div>
    </div>
  </div>
);

const Dot: React.FC<{ color: string; hollow?: boolean; label: string; p: Palette }> = ({ color, hollow, label, p }) => (
  <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
    <div style={{ width: 8, height: 8, borderRadius: 4, background: hollow ? "transparent" : color, border: hollow ? `1.5px solid ${color}` : "none", boxSizing: "border-box" }} />
    <T size={12} color={p.textSecondary}>
      {label}
    </T>
  </div>
);

/// The Claude tab with the account switcher, as the app shows a grouped tab.
const CLAUDE_TAB_HEIGHT = 640;

const ClaudeTab: React.FC<{ p: Palette; draw: number }> = ({ p, draw }) => {
  const s = STRINGS.en.s;
  const personal = sections()[0];
  const max = Math.max(...TOKEN_DAYS.map((d) => d.input + d.cache + d.output));
  const shown = Math.round(TOKEN_DAYS.length * draw);
  return (
    <div style={{ position: "absolute", inset: 0 }}>
      <div style={{ padding: "4px 16px 16px", display: "flex", flexDirection: "column", gap: p.cardGap }}>
        <div style={{ display: "flex", alignItems: "center", height: 28, marginTop: 8, padding: "0 4px" }}>
          <T size={14} color={p.textPrimary} weight={600}>
            Claude
          </T>
          <T size={14} color={p.textTertiary} style={{ marginLeft: 4 }}>
            Max
          </T>
          <div style={{ flex: 1 }} />
          <T size={12} color={p.textTertiary}>
            Personal
          </T>
        </div>
        <QuotaGroup quotas={personal.quotas} layout="cards" p={p} s={s} />
        <CreditCard p={p} />
        <ResetRow count={1} expires="11d 4h" p={p} s={s} />
        <div style={{ ...cardStyle(p), padding: 12 }}>
          <div style={{ display: "flex", alignItems: "flex-start" }}>
            <div style={{ flex: 1 }}>
              <T size={12} color={p.textSecondary}>
                Today
              </T>
              <T size={14} color={p.textPrimary} weight={600}>
                12.4M <span style={{ color: p.textTertiary, fontWeight: 400 }}>≈ $4.92</span>
              </T>
            </div>
            <div style={{ flex: 1.3 }}>
              <T size={12} color={p.textSecondary}>
                Last 30 days
              </T>
              <T size={14} color={p.textPrimary} weight={600}>
                546.6M <span style={{ color: p.textTertiary, fontWeight: 400 }}>≈ $216.03</span>
              </T>
            </div>
            <Segmented items={["Tokens", "$"]} selected={0} p={p} />
          </div>
          <div style={{ position: "relative", display: "flex", alignItems: "flex-end", gap: 3, height: 76, marginTop: 12 }}>
            <div style={{ position: "absolute", left: 0, right: 0, bottom: 0, borderTop: `1.5px dashed ${p.chartGrid}` }} />
            {TOKEN_DAYS.map((d, i) => (
              <div key={i} style={{ flex: 1, display: "flex", flexDirection: "column-reverse", height: "100%", opacity: i < shown ? 1 : 0 }}>
                <div style={{ height: `${(d.input / max) * 100}%`, background: INPUT, borderRadius: "0 0 1px 1px" }} />
                <div style={{ height: `${(d.cache / max) * 100}%`, background: `color-mix(in srgb, ${INPUT} 45%, transparent)` }} />
                <div style={{ height: `${(d.output / max) * 100}%`, background: OUTPUT, borderRadius: "1px 1px 0 0" }} />
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 14, marginTop: 12 }}>
            <Dot color={INPUT} label="Input" p={p} />
            <Dot color={p.textSecondary} hollow label="Cache" p={p} />
            <Dot color={OUTPUT} label="Output" p={p} />
            <div style={{ flex: 1 }} />
            <Segmented items={["Type", "Model"]} selected={0} p={p} />
          </div>
        </div>
      </div>
      {/* Account switcher pinned above the footer. */}
      <div style={{ position: "absolute", left: 16, right: 16, bottom: 10, height: 40, display: "flex", borderRadius: p.controlRadius + 2, background: p.controlFill, overflow: "hidden" }}>
        {[
          { label: "Personal", badge: personal.badge!, on: true },
          { label: "Work", badge: sections()[3].badge!, on: false },
        ].map((item) => (
          <div
            key={item.label}
            style={{
              flex: 1,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: 8,
              borderRadius: p.controlRadius + 2,
              background: item.on ? p.accent : "transparent",
              color: item.on ? p.textOnAccent : p.textPrimary,
              fontSize: 14,
              fontWeight: 600,
            }}
          >
            <div style={{ height: 16, minWidth: 16, padding: "0 3px", borderRadius: 4, background: item.badge.color, color: "#fff", fontSize: 11, fontWeight: 700, display: "flex", alignItems: "center", justifyContent: "center", boxSizing: "border-box", border: "1px solid rgba(255,255,255,0.6)" }}>
              {item.badge.text}
            </div>
            {item.label}
          </div>
        ))}
      </div>
    </div>
  );
};

/// Simple tray glyphs for the zoomed taskbar.
const TrayGlyph: React.FC<{ kind: "chevron" | "wifi" | "volume" }> = ({ kind }) => (
  <svg width={18} height={18} viewBox="0 0 18 18" fill="none" stroke="#fff" strokeWidth={1.4} strokeLinecap="round" strokeLinejoin="round">
    {kind === "chevron" && <path d="M5 11 L9 7 L13 11" />}
    {kind === "wifi" && (
      <>
        <path d="M2.5 7 Q9 1.5 15.5 7" />
        <path d="M5 9.8 Q9 6.5 13 9.8" />
        <path d="M7.4 12.5 Q9 11.2 10.6 12.5" />
      </>
    )}
    {kind === "volume" && (
      <>
        <path d="M3 7 H6 L10 3.5 V14.5 L6 11 H3 Z" />
        <path d="M12.5 6.5 Q14.5 9 12.5 11.5" />
      </>
    )}
  </svg>
);

const TASKBAR = 48;
/// Gap between the popup and the screen edge / taskbar.
const POPUP_EDGE = 12;
/// Home with four accounts, without empty space under the last card.
const HOME_POPUP_HEIGHT = 950;
const TRAY = { x: 1650, y: 1056 };

/// Phrase after the drop: the app icon lands in the tray, the popup slides
/// in, then one move clicks the Claude tab, shrinks the popup to it and
/// pushes the camera in on the taskbar corner.
const OpenApp: React.FC<{ t: number }> = ({ t }) => {
  const beat = t / B;
  const icon = useSpringAt(0, { damping: 9, stiffness: 180, mass: 0.8 });
  const toTray = ease.inOut((beat - 1.6) / 0.9);
  const bar = useSpringAt(1.3 * B, { damping: 20, stiffness: 140 });
  const rise = useSpringAt(2.1 * B, { damping: 26, stiffness: 170, mass: 0.9 });
  const word = useSpringAt(0.15, { damping: 13, stiffness: 200 });
  // Tab switch, popup resize and camera push share one curve.
  const go = ease.inOut((beat - 4.4) / 1.1);
  const zoom = 1 + 0.45 * go;
  const height = lerp(HOME_POPUP_HEIGHT, CLAUDE_TAB_HEIGHT, go);
  const iconSize = lerp(260 * icon, 26, toTray);
  const ix = lerp(960, TRAY.x, toTray);
  const iy = lerp(430, TRAY.y, toTray);
  const caption: Line[] = [
    { text: "Every limit.", beat: 2.3, accent: ["limit."] },
    { text: "One click away.", beat: 3.3 },
  ];
  const p = fluent(true);
  return (
    <AbsoluteFill>
      {beat >= 2.1 && <Headline lines={caption} t={t} side={1} />}
      {/* Desktop corner: popup, taskbar and tray zoom together. */}
      <AbsoluteFill style={{ transform: `scale(${zoom})`, transformOrigin: "1920px 1080px" }}>
        {beat >= 2.1 && (
          // Like the app: the popup slides in from the right edge above the tray.
          <div
            style={{
              position: "absolute",
              left: 1920 - POPUP_EDGE - POPUP_WIDTH,
              top: 1080 - TASKBAR - POPUP_EDGE - height,
              transform: `translateX(${(1 - rise) * (POPUP_WIDTH + POPUP_EDGE + 20)}px)`,
            }}
          >
            <PopupFrame p={p} height={height} footer={<Footer p={p} active={3 * go} />}>
              {/* Home slides out as the Claude tab slides in. */}
              <div style={{ position: "absolute", inset: 0, opacity: 1 - go, transform: `translateX(${-go * 90}px)` }}>
                <HomePage p={p} s={STRINGS.en.s} sections={sections()} usageProgress={ease.outExpo((t - 2.3 * B) / 1.4)} />
              </div>
              {go > 0 && (
                <div style={{ position: "absolute", inset: 0, opacity: go, transform: `translateX(${(1 - go) * 90}px)` }}>
                  <ClaudeTab p={p} draw={ease.outExpo((beat - 5) / 1.6)} />
                </div>
              )}
            </PopupFrame>
          </div>
        )}
        {/* Taskbar with the tray. */}
        <div
          style={{
            position: "absolute",
            left: 0,
            right: 0,
            bottom: 0,
            height: TASKBAR,
            transform: `translateY(${(1 - bar) * 60}px)`,
            background: "rgba(22,22,28,0.85)",
            backdropFilter: "blur(30px)",
            borderTop: "1px solid rgba(255,255,255,0.08)",
            fontFamily: SEGOE,
            color: "#fff",
            fontSize: 13,
          }}
        >
          <div style={{ position: "absolute", left: TRAY.x - 44, top: 15 }}>
            <TrayGlyph kind="chevron" />
          </div>
          <div style={{ position: "absolute", left: 1684, top: 16, fontSize: 12 }}>ENG</div>
          <div style={{ position: "absolute", left: 1724, top: 15 }}>
            <TrayGlyph kind="wifi" />
          </div>
          <div style={{ position: "absolute", left: 1752, top: 15 }}>
            <TrayGlyph kind="volume" />
          </div>
          <div style={{ position: "absolute", right: 16, top: 7, textAlign: "right", lineHeight: "17px" }}>
            <div>14:58</div>
            <div>11.10.2026</div>
          </div>
        </div>
        {toTray >= 1 && beat >= 2.6 && (
          <Img
            src={staticFile("app-icon.png")}
            style={{ position: "absolute", left: TRAY.x - 13, top: TRAY.y - 13, width: 26, height: 26, transform: `scale(${1 + pulse(t) * 0.06})` }}
          />
        )}
      </AbsoluteFill>
      {(toTray < 1 || beat < 2.6) && (
        <Img
          src={staticFile("app-icon.png")}
          style={{
            position: "absolute",
            left: ix - iconSize / 2,
            top: iy - iconSize / 2,
            width: iconSize,
            height: iconSize,
            transform: `rotate(${(1 - icon) * -40}deg)`,
          }}
        />
      )}
      {beat < 2.6 && (
        <div
          style={{
            position: "absolute",
            left: 0,
            right: 0,
            top: 600,
            textAlign: "center",
            fontFamily: SEGOE,
            fontWeight: 800,
            fontSize: 130,
            letterSpacing: -4,
            color: "#fff",
            opacity: word * (1 - toTray * 2),
            transform: `scale(${lerp(1.4, 1, Math.min(1, word))})`,
          }}
        >
          Agent Minibar
        </div>
      )}
    </AbsoluteFill>
  );
};

/* ----------------------------------------------------- multiple accounts */

const ACCOUNTS: Section[] = [
  sections()[0],
  { ...sections()[3], layout: "lines" },
  {
    ...sections()[1],
    account: "Truman-fan",
    badge: { text: "V", color: "#7A5CFA" },
    layout: "rings",
  },
  {
    provider: "codex",
    name: "Codex",
    plan: "Plus",
    account: "steipete",
    badge: { text: "S", color: "#2FA36B" },
    layout: "tiles",
    quotas: [
      { id: "s5", title: "session", left: 23, reset: "1h 05m", ticks: 4, pace: { state: "deficit", delta: 14, at: 70 } },
      { id: "sw", title: "weekly", left: 66, reset: "4d 1h", ticks: 6, pace: { state: "on", at: 32 } },
    ],
  },
];

const ACCOUNT_TABS: Tab[] = [
  DEFAULT_TABS[0],
  DEFAULT_TABS[1],
  { key: "claude-p", provider: "claude", badge: ACCOUNTS[0].badge },
  { key: "claude-w", provider: "claude", badge: ACCOUNTS[1].badge },
  { key: "codex-a", provider: "codex", badge: ACCOUNTS[2].badge },
  { key: "codex-s", provider: "codex", badge: ACCOUNTS[3].badge },
  { key: "cursor", provider: "cursor" },
  { key: "refresh", icon: "fluent-arrow-sync-20-filled" },
  { key: "settings", icon: "fluent-settings-20-filled" },
];

const AccountRow: React.FC<{ section: Section; p: Palette; at: number; t: number }> = ({ section, p, at, t }) => {
  const k = useSpringAt(at, { damping: 15, stiffness: 190, mass: 0.7 });
  if (t < at - 0.01) return null;
  const s = STRINGS.en.s;
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: p.cardGap,
        opacity: Math.min(1, k * 2),
        transform: `translateX(${(1 - k) * 260}px) scale(${lerp(0.9, 1, Math.min(1, k))})`,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", height: 28, marginTop: 6, padding: "0 4px" }}>
        <ProviderMark provider={section.provider} size={16} p={p} badge={section.badge} />
        <div style={{ width: 14 }} />
        <T size={14} color={p.textPrimary} weight={600}>
          {section.name}
        </T>
        <T size={14} color={p.textTertiary} style={{ marginLeft: 4 }}>
          {section.plan}
        </T>
        <div style={{ flex: 1 }} />
        <T size={12} color={p.textTertiary}>
          {section.account}
        </T>
      </div>
      <QuotaGroup quotas={section.quotas} layout={section.layout} p={p} s={s} />
    </div>
  );
};

/// Four accounts of two providers deal into the popup.
const Accounts: React.FC<{ t: number }> = ({ t }) => {
  const p = fluent(true);
  const beat = t / B;
  // The tab indicator glides to each account as it lands.
  const index = Math.min(3, Math.max(0, Math.floor(beat / 1.5)));
  const active = 2 + (index === 0 ? 0 : index - 1 + ease.outExpo((beat - index * 1.5) / 0.6));
  return (
    <AbsoluteFill>
      <Headline
        side={-1}
        t={t}
        lines={[
          { text: "Every account.", beat: 0.3, accent: ["account."] },
          { text: "Side by side.", beat: 1.5 },
          { text: "Name, badge and layout for each one.", beat: 3, size: 34, dim: true },
        ]}
      />
      <Camera
        t={t}
        side={-1}
        width={POPUP_WIDTH}
        height={604}
        keys={[
          { beat: 0, s: 1.24, x: 130, y: 295, r: -2 },
          { beat: 8, s: 1.3, x: 130, y: 300, r: 1 },
        ]}
      >
        <PopupFrame p={p} height={604} footer={<Footer p={p} tabs={ACCOUNT_TABS} active={active} />}>
          <div style={{ padding: "4px 16px 16px", display: "flex", flexDirection: "column", gap: p.cardGap }}>
            {ACCOUNTS.map((section, i) => (
              <AccountRow key={i} section={section} p={p} at={i * 1.5 * B} t={t} />
            ))}
          </div>
        </PopupFrame>
      </Camera>
      {/* Badge chips that pop beside the popup as each account lands. */}
      {ACCOUNTS.map((section, i) => {
        const at = i * 1.5 * B + 0.1;
        return <Chip key={i} section={section} at={at} t={t} index={i} />;
      })}
    </AbsoluteFill>
  );
};

const Chip: React.FC<{ section: Section; at: number; t: number; index: number }> = ({ section, at, t, index }) => {
  const k = useSpringAt(at, { damping: 11, stiffness: 220, mass: 0.6 });
  if (t < at) return null;
  const y = 190 + index * 190;
  return (
    <div
      style={{
        position: "absolute",
        left: 24 + (index % 2) * 46,
        top: y,
        display: "flex",
        alignItems: "center",
        gap: 14,
        padding: "16px 28px 16px 20px",
        borderRadius: 16,
        background: "rgba(30,30,40,0.7)",
        backdropFilter: "blur(24px)",
        border: "1px solid rgba(255,255,255,0.12)",
        boxShadow: "0 18px 40px rgba(0,0,0,0.4)",
        fontFamily: SEGOE,
        color: "#fff",
        transform: `translateX(${(1 - k) * -200}px) scale(${Math.min(1.1, k)}) rotate(${(1 - k) * -8 + Math.sin(t * 2 + index) * 1.5}deg)`,
        opacity: Math.min(1, k * 2),
      }}
    >
      <ProviderMark provider={section.provider} size={40} p={fluent(true)} badge={section.badge} />
      <div style={{ width: 4 }} />
      <div>
        <div style={{ fontSize: 30, fontWeight: 700 }}>
          {section.name} {section.plan}
        </div>
        <div style={{ fontSize: 22, opacity: 0.7 }}>{section.account}</div>
      </div>
    </div>
  );
};

/* ----------------------------------------------------------- notifications */

/// Toast arrivals in scene beats, with their sounds.
const NOTIFY = [
  { beat: 0.1, sound: "warning" },
  { beat: 1.5, sound: "reset" },
  { beat: 3, sound: "info" },
  { beat: 4.6, sound: "info" },
];

/// A low limit, a fresh window and a login that needs renewing, on beats.
const beatTimeline = (): Timed[] => [
  // The oldest card leaves to make room for the last one.
  { ...notificationTimeline()[0], at: NOTIFY[0].beat * B, leave: (NOTIFY[3].beat - 0.3) * B, replaces: undefined },
  {
    at: NOTIFY[1].beat * B,
    spec: {
      id: "reset",
      kind: "reset",
      title: "5-hour limit reset",
      alert: {
        provider: "codex",
        name: "Codex",
        plan: "Pro",
        account: "Truman-fan",
        rows: [
          { q: { id: "r5", title: "session", left: 100, reset: "4h 59m", ticks: 4 }, focused: true },
          { q: { id: "rw", title: "weekly", left: 57, reset: "5d 2h", ticks: 6 }, focused: false },
        ],
      },
    },
  },
  {
    at: NOTIFY[2].beat * B,
    spec: {
      id: "login",
      kind: "info",
      title: "Claude login expires soon",
      body: "It stops renewing on Oct 14. Open Minibar and choose Sign in again.",
      bodyLines: 2,
    },
  },
  {
    // From the public reset feed, which follows Tibo's announcements.
    at: NOTIFY[3].beat * B,
    spec: {
      id: "feed",
      kind: "reset",
      title: "New Codex reset info",
      body: "A possible Codex reset is scheduled for Oct 14, 12:00 (in 2d 3h)",
      bodyLines: 2,
    },
  },
];

const Notifications: React.FC<{ t: number }> = ({ t }) => {
  const p = fluent(true);
  const timeline = React.useMemo(beatTimeline, []);
  return (
    <AbsoluteFill>
      <Headline
        side={1}
        t={t}
        lines={[
          { text: "Alerts before", beat: 0.2 },
          { text: "you hit the wall.", beat: 1.5, accent: ["wall."] },
          { text: "Low limits, fresh windows and logins, with sound.", beat: 3, size: 34, dim: true },
        ]}
      />
      <Camera
        t={t}
        side={1}
        width={392}
        height={560}
        punch={0.01}
        keys={[
          { beat: 0, s: 1.5, x: 196, y: 420, r: 2 },
          { beat: 4, s: 1.65, x: 196, y: 340, r: 0 },
          { beat: 8, s: 1.7, x: 196, y: 320, r: -1 },
        ]}
      >
        <ToastStack toasts={timeline} time={t} p={p} s={STRINGS.en.s} height={560} />
      </Camera>
    </AbsoluteFill>
  );
};

/* ----------------------------------------------------------------- cursor */

type CursorKey = { beat: number; x: number; y: number; click?: boolean };

/// A mouse pointer gliding between keyframes (scene beats, clip pixels) and
/// clicking where asked.
const Cursor: React.FC<{ t: number; keys: CursorKey[] }> = ({ t, keys }) => {
  const beat = t / B;
  const i = Math.max(0, keys.filter((key) => key.beat <= beat).length - 1);
  const a = keys[i];
  const b = keys[Math.min(i + 1, keys.length - 1)];
  const k = a === b ? 0 : ease.inOut((beat - a.beat) / Math.min(0.7, b.beat - a.beat));
  const x = lerp(a.x, b.x, k);
  const y = lerp(a.y, b.y, k);
  const click = keys.filter((key) => key.click && beat >= key.beat).pop();
  const since = click ? t - click.beat * B : 10;
  const press = since < 0.18 ? Math.sin((since / 0.18) * Math.PI) : 0;
  return (
    <>
      {since < 0.45 && click && (
        <div
          style={{
            position: "absolute",
            left: click.x - 22,
            top: click.y - 22,
            width: 44,
            height: 44,
            borderRadius: "50%",
            border: `2px solid ${ORANGE}`,
            opacity: 1 - since / 0.45,
            transform: `scale(${0.4 + since * 2})`,
          }}
        />
      )}
      <svg
        width={22}
        height={28}
        viewBox="0 0 22 28"
        style={{
          position: "absolute",
          left: x - 3,
          top: y - 2,
          transform: `scale(${1 - press * 0.15})`,
          transformOrigin: "3px 2px",
          filter: "drop-shadow(0 2px 3px rgba(0,0,0,0.45))",
        }}
      >
        <path d="M3 2 L3 22 L8 17.5 L11.5 25 L15 23.5 L11.6 16 L18 16 Z" fill="#fff" stroke="#111" strokeWidth={1.4} strokeLinejoin="round" />
      </svg>
    </>
  );
};

/// Text typed between two beats, with a caret while focused.
const typed = (text: string, t: number, from: number, to: number) =>
  text.slice(0, Math.round(interpolate(t / B, [from, to], [0, text.length], clamp)));

/* -------------------------------------------------------------- openrouter */

const OR_HEIGHT = 520;

type OrKey = { name: string; badge?: string; spent: string; percent?: number; summary: string; today: string };

const OR_KEYS: OrKey[] = [
  { name: "minibar", badge: "This app", spent: "$4.12", summary: "No limit · expires Dec 31", today: "today $0.84" },
  { name: "cline-work", spent: "$18.40", percent: 74, summary: "$18.40 of $25.00 · resets monthly", today: "today $2.10" },
  { name: "ci-evals", spent: "$46.00", percent: 92, summary: "$46.00 of $50.00 · resets weekly", today: "today $6.95" },
  { name: "playground", spent: "$0.62", summary: "No limit", today: "today $0.00" },
];

const SmallButton: React.FC<{ label: string; p: Palette; icon?: string; accent?: boolean; pressed?: number; wide?: boolean }> = ({
  label,
  p,
  icon,
  accent,
  pressed = 0,
  wide,
}) => (
  <div
    style={{
      height: 32,
      padding: "0 12px",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      gap: 6,
      borderRadius: p.controlRadius,
      background: accent ? p.accent : p.controlFill,
      border: accent ? "none" : `1px solid ${p.cardStroke}`,
      color: accent ? p.textOnAccent : p.textPrimary,
      fontSize: 14,
      fontWeight: 600,
      whiteSpace: "nowrap",
      transform: `scale(${1 - pressed * 0.06})`,
      flex: wide ? 1 : "none",
    }}
  >
    {icon && <Icon name={icon} size={16} color={accent ? p.textOnAccent : p.textPrimary} />}
    {label}
  </div>
);

/// 0..1..0 bump right after a click at `beat`.
const pressAt = (t: number, beat: number) => {
  const since = t - beat * B;
  return since >= 0 && since < 0.2 ? Math.sin((since / 0.2) * Math.PI) : 0;
};

const Field: React.FC<{ label: string; value: string; p: Palette; prefix?: string; focused: boolean; hint?: string; t: number }> = ({
  label,
  value,
  p,
  prefix,
  focused,
  hint,
  t,
}) => (
  <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
    <T size={12} color={p.textSecondary}>
      {label}
    </T>
    <div
      style={{
        height: 32,
        borderRadius: p.controlRadius,
        background: p.controlFill,
        border: `1px solid ${p.cardStroke}`,
        borderBottom: `2px solid ${focused ? p.accent : p.cardStroke}`,
        display: "flex",
        alignItems: "center",
        padding: "0 10px",
        gap: 4,
        fontSize: 14,
        color: p.textPrimary,
        boxSizing: "border-box",
      }}
    >
      {prefix && <span style={{ color: p.textTertiary }}>{prefix}</span>}
      <span>{value}</span>
      {focused && <span style={{ width: 1.5, height: 16, background: p.textPrimary, opacity: Math.floor(t * 3) % 2 ? 0 : 1 }} />}
      {!value && !focused && <span style={{ color: p.textTertiary }}>{label === "Name" ? "e.g. laptop-cursor" : "No limit"}</span>}
    </div>
    {hint && (
      <T size={12} color={p.textTertiary}>
        {hint}
      </T>
    )}
  </div>
);

const Switch: React.FC<{ label: string; on: boolean; p: Palette }> = ({ label, on, p }) => (
  <div style={{ display: "flex", alignItems: "center", height: 28 }}>
    <T size={14} color={p.textPrimary} style={{ flex: 1 }}>
      {label}
    </T>
    <div
      style={{
        width: 40,
        height: 20,
        borderRadius: 10,
        background: on ? p.accent : "transparent",
        border: on ? "none" : `1px solid ${p.textSecondary}`,
        boxSizing: "border-box",
        position: "relative",
      }}
    >
      <div
        style={{
          position: "absolute",
          top: on ? 4 : 3,
          left: on ? 24 : 4,
          width: on ? 12 : 12,
          height: 12,
          borderRadius: 6,
          background: on ? p.textOnAccent : p.textSecondary,
        }}
      />
    </div>
  </div>
);

const PageHeader: React.FC<{ title: string; p: Palette; back?: boolean }> = ({ title, p, back }) => (
  <div style={{ height: 32, display: "flex", alignItems: "center", gap: 8, padding: "0 4px" }}>
    {back && <Icon name="fluent-arrow-left-16-regular" size={16} color={p.textSecondary} />}
    <T size={14} color={p.textPrimary} weight={600}>
      {title}
    </T>
  </div>
);

const KeysList: React.FC<{ p: Palette; t: number }> = ({ p, t }) => (
  <div style={{ padding: "4px 16px 16px", display: "flex", flexDirection: "column", gap: 6 }}>
    <div style={{ display: "flex", alignItems: "center", height: 28, marginTop: 8, padding: "0 4px" }}>
      <ProviderMark provider="openrouter" size={16} p={p} />
      <div style={{ width: 8 }} />
      <T size={14} color={p.textPrimary} weight={600}>
        OpenRouter
      </T>
      <div style={{ flex: 1 }} />
      <T size={12} color={p.textTertiary}>
        Personal
      </T>
    </div>
    {/* Credits left on the account. */}
    <div style={{ ...cardStyle(p), height: 58, overflow: "hidden" }}>
      <div style={{ position: "absolute", inset: 0, width: "57.8%", background: `color-mix(in srgb, ${p.accent} 20%, transparent)` }} />
      <div style={{ position: "relative", height: "100%", padding: "0 12px", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <T size={12} color={p.textSecondary}>
            CREDITS
          </T>
          <T size={14} color={p.accent} weight={600}>
            $57.82 left
          </T>
        </div>
        <T size={12} color={p.textTertiary}>
          of $100.00
        </T>
      </div>
    </div>
    <div style={{ display: "flex", alignItems: "center", marginTop: 8, padding: "0 4px", height: 32 }}>
      <T size={14} color={p.textSecondary} weight={600}>
        Keys
      </T>
      <T size={12} color={p.textTertiary} style={{ marginLeft: 6 }}>
        · 4
      </T>
      <div style={{ flex: 1 }} />
      <SmallButton label="New key" icon="fluent-add-16-regular" p={p} pressed={pressAt(t, 1.6)} />
    </div>
    <div style={{ ...cardStyle(p), overflow: "hidden" }}>
      {OR_KEYS.map((key, i) => (
        <div key={key.name} style={{ padding: "10px 12px", borderTop: i ? `1px solid ${p.divider}` : "none", display: "flex", flexDirection: "column", gap: 4 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <T size={14} color={p.textPrimary} weight={600} style={{ flex: 1 }}>
              {key.name}
            </T>
            {key.badge && (
              <div style={{ padding: "1px 6px", borderRadius: 4, fontSize: 11, fontWeight: 600, color: p.accent, background: `color-mix(in srgb, ${p.accent} 16%, transparent)` }}>
                {key.badge}
              </div>
            )}
            {key.percent !== undefined && key.percent >= 80 && (
              <div style={{ padding: "1px 6px", borderRadius: 4, fontSize: 11, fontWeight: 600, color: p.caution, background: `color-mix(in srgb, ${p.caution} 16%, transparent)` }}>
                {key.percent}%
              </div>
            )}
            <T size={14} color={p.accent} weight={600}>
              {key.spent}
            </T>
            <Icon name="fluent-chevron-down-16-regular" size={14} color={p.textTertiary} />
          </div>
          {key.percent !== undefined && (
            <div style={{ height: 6, borderRadius: 3, background: p.controlFill, overflow: "hidden" }}>
              <div style={{ width: `${100 - key.percent}%`, height: "100%", borderRadius: 3, background: key.percent >= 80 ? p.caution : p.accent }} />
            </div>
          )}
          <div style={{ display: "flex" }}>
            <T size={12} color={p.textTertiary} style={{ flex: 1, whiteSpace: "nowrap" }}>
              {key.summary}
            </T>
            <T size={12} color={p.textTertiary}>
              {key.today}
            </T>
          </div>
        </div>
      ))}
    </div>
  </div>
);

const RESETS = ["Never", "Daily", "Weekly", "Monthly"];

const CreateKey: React.FC<{ p: Palette; t: number }> = ({ p, t }) => {
  const beat = t / B;
  const name = typed("laptop-cursor", t, 2.1, 2.85);
  const limit = typed("25", t, 3.1, 3.3);
  const reset = beat < 3.7 ? 0 : 3 * ease.outExpo((beat - 3.7) / 0.5);
  const creating = beat >= 4.5;
  return (
    <div style={{ padding: "4px 16px 16px", display: "flex", flexDirection: "column", gap: 12 }}>
      <PageHeader title="New key" p={p} back />
      <div style={{ ...cardStyle(p), padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
        <Field label="Name" value={name} p={p} focused={beat >= 2 && beat < 3} t={t} />
        <Field label="Spending limit" value={limit} prefix="$" p={p} focused={beat >= 3 && beat < 3.6} hint="Leave empty for no limit." t={t} />
        <div style={{ display: "flex", flexDirection: "column", gap: 4, opacity: limit ? 1 : 0.55 }}>
          <T size={12} color={p.textSecondary}>
            Resets
          </T>
          <Segmented items={RESETS} selected={Math.round(reset)} glide={reset} p={p} />
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <T size={12} color={p.textSecondary}>
            Expires
          </T>
          <div
            style={{
              height: 32,
              borderRadius: p.controlRadius,
              background: p.controlFill,
              border: `1px solid ${p.cardStroke}`,
              display: "flex",
              alignItems: "center",
              padding: "0 10px",
              boxSizing: "border-box",
            }}
          >
            <T size={14} color={p.textPrimary} style={{ flex: 1 }}>
              30 days
            </T>
            <Icon name="fluent-chevron-down-16-regular" size={14} color={p.textSecondary} />
          </div>
        </div>
        <Switch label="Count BYOK usage toward limit" on={false} p={p} />
        <Switch label="Track this key in Minibar" on p={p} />
      </div>
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 6 }}>
        <SmallButton label="Cancel" p={p} />
        <SmallButton label={creating ? "Creating…" : "Create key"} p={p} accent pressed={pressAt(t, 4.5)} />
      </div>
    </div>
  );
};

const RevealKey: React.FC<{ p: Palette; t: number }> = ({ p, t }) => {
  const copied = t / B >= 6.1;
  return (
    <div style={{ padding: "4px 16px 16px", display: "flex", flexDirection: "column", gap: 12 }}>
      <PageHeader title="Key created" p={p} />
      <div style={{ ...cardStyle(p), padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
        <div style={{ display: "flex", alignItems: "center", height: 20 }}>
          <T size={14} color={p.textPrimary} weight={600} style={{ flex: 1 }}>
            laptop-cursor
          </T>
          <T size={12} color={p.textTertiary}>
            $25.00 limit · resets monthly
          </T>
        </div>
        <div
          style={{
            padding: 10,
            borderRadius: 6,
            background: "rgba(0,0,0,0.25)",
            border: `1px solid ${p.cardStroke}`,
            fontFamily: "Consolas, monospace",
            fontSize: 12.5,
            lineHeight: "18px",
            color: p.textPrimary,
          }}
        >
          sk-or-v1-3f9c2a71d0e84b5c96a2…e81b
        </div>
        <div style={{ display: "flex" }}>
          <SmallButton
            label={copied ? "Copied" : "Copy key"}
            icon={copied ? "fluent-checkmark-circle-16-filled" : "fluent-copy-16-regular"}
            p={p}
            accent
            wide
            pressed={pressAt(t, 6.1)}
          />
        </div>
      </div>
      <div
        style={{
          ...cardStyle(p),
          padding: 12,
          display: "flex",
          gap: 10,
          background: `color-mix(in srgb, ${p.caution} 12%, transparent)`,
        }}
      >
        <Icon name="fluent-warning-16-filled" size={16} color={p.caution} />
        <div>
          <T size={14} color={p.textPrimary} weight={600}>
            You won't see this key again
          </T>
          <T size={12} color={p.textSecondary}>
            OpenRouter shows a new key only once. Copy it now.
          </T>
        </div>
      </div>
      <div style={{ display: "flex", alignItems: "center", padding: "0 4px" }}>
        <T size={12} color={p.textTertiary} style={{ flex: 1 }}>
          The popup stays open until you press Done.
        </T>
        <SmallButton label="Done" p={p} accent={copied} />
      </div>
    </div>
  );
};

/// OpenRouter tab: credits and keys, then a new key is created and copied.
const OpenRouterScene: React.FC<{ t: number }> = ({ t }) => {
  const p = fluent(true);
  const beat = t / B;
  // Pages slide like the popup's own page transitions.
  const toCreate = ease.outExpo((beat - 1.75) / 0.5);
  const toReveal = ease.outExpo((beat - 4.9) / 0.5);
  const page = (x: number, node: React.ReactNode) =>
    x > -1 && x < 1 ? (
      <div style={{ position: "absolute", inset: 0, transform: `translateX(${x * POPUP_WIDTH}px)`, opacity: 1 - Math.abs(x) * 0.6 }}>{node}</div>
    ) : null;
  return (
    <AbsoluteFill>
      <Headline
        side={-1}
        t={t}
        lines={[
          { text: "First-class", beat: 0.2 },
          { text: "OpenRouter.", beat: 1, accent: ["OpenRouter."] },
          { text: "Credits, key spend and new keys from the tray.", beat: 2.5, size: 34, dim: true },
        ]}
      />
      <Camera
        t={t}
        side={-1}
        width={POPUP_WIDTH}
        height={OR_HEIGHT}
        keys={[
          { beat: 0, s: 1.3, x: 190, y: 250, r: -1 },
          { beat: 2, s: 1.45, x: 190, y: 200 },
          { beat: 4.2, s: 1.4, x: 190, y: 300 },
          { beat: 6, s: 1.5, x: 190, y: 200, r: 1 },
          { beat: 8, s: 1.45, x: 190, y: 220 },
        ]}
      >
        <PopupFrame p={p} height={OR_HEIGHT} footer={<Footer p={p} active={5} />}>
          {page(-toCreate, <KeysList p={p} t={t} />)}
          {page(1 - toCreate - toReveal, <CreateKey p={p} t={t} />)}
          {page(1 - toReveal, <RevealKey p={p} t={t} />)}
        </PopupFrame>
        <Cursor
          t={t}
          keys={[
            { beat: 0, x: 250, y: 470 },
            { beat: 1.1, x: 300, y: 128 },
            { beat: 1.6, x: 300, y: 128, click: true },
            { beat: 1.95, x: 190, y: 96, click: true },
            { beat: 2.9, x: 190, y: 160 },
            { beat: 3.05, x: 190, y: 160, click: true },
            { beat: 3.5, x: 253, y: 244 },
            { beat: 3.7, x: 253, y: 244, click: true },
            { beat: 4.2, x: 316, y: 444 },
            { beat: 4.5, x: 316, y: 444, click: true },
            { beat: 5.6, x: 190, y: 154 },
            { beat: 6.1, x: 190, y: 154, click: true },
            { beat: 8, x: 230, y: 230 },
          ]}
        />
      </Camera>
    </AbsoluteFill>
  );
};

/* -------------------------------------------------------------- activation */

const ACTIVATION_CUES = [
  { beat: 3.2, sound: "success" },
  { beat: 5.6, sound: "reset" },
];

/// A schedule rule row: provider, days, time and a switch.
const RuleRow: React.FC<{ provider: string; days: string; time: string; p: Palette; at: number; hit: number; t: number }> = ({
  provider,
  days,
  time,
  p,
  at,
  hit,
  t,
}) => {
  const k = useSpringAt(at, { damping: 16, stiffness: 200, mass: 0.6 });
  const glow = Math.max(0, 1 - Math.abs(t - hit) / 0.6);
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        height: 48,
        padding: "0 12px",
        borderRadius: 8,
        background: glow > 0 ? `color-mix(in srgb, ${p.accent} ${glow * 18}%, ${p.cardBackground})` : p.cardBackground,
        border: `1px solid ${glow > 0 ? p.accent : p.cardStroke}`,
        opacity: Math.min(1, k * 2),
        transform: `translateX(${(1 - k) * 60}px)`,
      }}
    >
      <ProviderMark provider={provider} size={18} p={p} />
      <T size={14} color={p.textPrimary} weight={600} style={{ flex: 1 }}>
        {days}
      </T>
      <T size={14} color={p.textPrimary} style={{ fontVariantNumeric: "tabular-nums" }}>
        {time}
      </T>
      <Switch label="" on p={p} />
    </div>
  );
};

const session = (q: Quota, started: boolean): Quota =>
  started ? { ...q, left: 100, reset: "4h 59m", notStarted: false, pace: undefined } : { ...q, left: 0, reset: "", notStarted: true, pace: undefined };

/// Scheduled and automatic activation: 09:00 starts Codex, a reset restarts Claude.
const ActivationScene: React.FC<{ t: number }> = ({ t }) => {
  const p = fluent(true);
  const beat = t / B;
  const card = useSpringAt(0, { damping: 18, stiffness: 150, mass: 0.8 });
  const auto = beat >= 0.9;
  const codexOn = beat >= 3.1;
  const claudeOn = beat >= 5.5;
  const fill = (from: number) => 100 * ease.outExpo((beat - from) / 1.2);
  const clock = beat < 2 ? "08:59:57" : beat < 2.5 ? "08:59:58" : beat < 3 ? "08:59:59" : "09:00:00";
  const codex: Section = {
    ...sections()[1],
    layout: "cards",
    resets: undefined,
    quotas: [session(sections()[1].quotas[0], codexOn), sections()[1].quotas[1]],
  };
  const claude: Section = {
    ...sections()[0],
    resets: undefined,
    quotas: [session(sections()[0].quotas[0], claudeOn), sections()[0].quotas[1]],
  };
  const toasts: Timed[] = [
    { ...notificationTimeline()[1], at: ACTIVATION_CUES[0].beat * B, leave: undefined },
    {
      at: ACTIVATION_CUES[1].beat * B,
      spec: {
        id: "activated",
        kind: "reset",
        title: "5-hour limit reset and activated",
        alert: {
          provider: "claude",
          name: "Claude",
          plan: "Max",
          account: "Personal",
          badge: { text: "P", color: "#E07A3F" },
          rows: [{ q: { id: "a5", title: "session", left: 100, reset: "4h 59m", ticks: 4 }, focused: true }],
        },
      },
    },
  ];
  const sway = Math.sin(t * 1.1) * 2;
  return (
    <AbsoluteFill>
      <Headline
        side={1}
        t={t}
        lines={[
          { text: "Never waste", beat: 0.2 },
          { text: "a 5-hour window.", beat: 1, accent: ["window."] },
          { text: "On a schedule, or the moment a limit resets.", beat: 2.5, size: 34, dim: true },
        ]}
      />
      <AbsoluteFill style={{ perspective: 2000 }}>
        <div style={{ position: "absolute", inset: 0, transform: `rotateY(${-3 + sway}deg)`, transformOrigin: "1440px 540px" }}>
          {/* Activation rules. */}
          <div
            style={{
              position: "absolute",
              left: 1000,
              top: 160,
              width: 460,
              padding: 18,
              borderRadius: 12,
              background: "rgba(32,32,38,0.88)",
              border: "1px solid rgba(255,255,255,0.1)",
              boxShadow: "0 24px 60px rgba(0,0,0,0.45)",
              backdropFilter: "blur(30px)",
              fontFamily: p.font,
              display: "flex",
              flexDirection: "column",
              gap: 10,
              opacity: Math.min(1, card * 2),
              transform: `translateY(${(1 - card) * 60}px)`,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <div style={{ flex: 1 }}>
                <T size={14} color={p.textPrimary} weight={600}>
                  Start 5-hour sessions automatically
                </T>
                <T size={12} color={p.textSecondary}>
                  As soon as a window is available.
                </T>
              </div>
              <Switch label="" on={auto} p={p} />
            </div>
            <div style={{ height: 1, background: p.divider, margin: "4px 0" }} />
            <T size={14} color={p.textPrimary} weight={600}>
              Scheduled activations
            </T>
            <RuleRow provider="codex" days="Weekdays" time="09:00" p={p} at={1.2 * B} hit={3 * B} t={t} />
            <RuleRow provider="claude" days="Mon, Wed, Fri" time="13:30" p={p} at={1.6 * B} hit={-9} t={t} />
          </div>
          {/* Clock ticking into the scheduled minute. */}
          <div
            style={{
              position: "absolute",
              left: 1000,
              top: 470,
              fontFamily: SEGOE,
              fontSize: 92,
              fontWeight: 700,
              letterSpacing: -2,
              color: beat >= 3 ? ORANGE : "#fff",
              fontVariantNumeric: "tabular-nums",
              opacity: beat >= 1.7 ? 1 : 0,
              transform: `scale(${1 + pressAt(t, 3) * 0.06})`,
              transformOrigin: "0% 50%",
            }}
          >
            {clock}
          </div>
          {/* The popup picks up both sessions. */}
          <div style={{ position: "absolute", left: 1510, top: 110 }}>
            <PopupFrame p={p} height={420} footer={<Footer p={p} active={0} />}>
              <div style={{ padding: "4px 16px 16px", display: "flex", flexDirection: "column", gap: p.cardGap }}>
                <ActivationBlock section={codex} p={p} fills={{ [codex.quotas[0].id]: codexOn ? fill(3.1) : 0 }} />
                <ActivationBlock section={claude} p={p} fills={{ [claude.quotas[0].id]: claudeOn ? fill(5.5) : 0 }} />
              </div>
            </PopupFrame>
          </div>
          <div style={{ position: "absolute", left: 1498, top: 540, width: 392, height: 440 }}>
            <ToastStack toasts={toasts} time={t} p={p} s={STRINGS.en.s} height={440} />
          </div>
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};

const ActivationBlock: React.FC<{ section: Section; p: Palette; fills: Record<string, number> }> = ({ section, p, fills }) => (
  <div style={{ display: "flex", flexDirection: "column", gap: p.cardGap }}>
    <div style={{ display: "flex", alignItems: "center", height: 28, marginTop: 8, padding: "0 4px" }}>
      <ProviderMark provider={section.provider} size={16} p={p} badge={section.badge} />
      <div style={{ width: section.badge ? 14 : 8 }} />
      <T size={14} color={p.textPrimary} weight={600}>
        {section.name}
      </T>
      <T size={14} color={p.textTertiary} style={{ marginLeft: 4 }}>
        {section.plan}
      </T>
      <div style={{ flex: 1 }} />
      <T size={12} color={p.textTertiary}>
        {section.account}
      </T>
    </div>
    <QuotaGroup quotas={section.quotas} layout="cards" p={p} s={STRINGS.en.s} fills={fills} />
  </div>
);

/* ------------------------------------------------------------- the phrases */

const Scene: React.FC<{ speed: number; children: React.ReactNode }> = ({ speed, children }) => (
  <Speed.Provider value={speed}>{children}</Speed.Provider>
);

const UsageScene: React.FC<{ t: number }> = ({ t }) => (
  <AbsoluteFill>
    <Headline
      side={1}
      t={t}
      lines={[
        { text: "Know what", beat: 0.2 },
        { text: "you spend.", beat: 1.2, accent: ["spend."] },
        { text: "Cost and tokens per model, from local logs.", beat: 3, size: 34, dim: true },
      ]}
    />
    <Camera
      t={t}
      side={1}
      width={520}
      height={800}
      keys={[
        { beat: 0, s: 1.05, x: 260, y: 400, r: 2 },
        { beat: 4, s: 1.6, x: 260, y: 420, r: 0 },
        { beat: 8, s: 1.45, x: 260, y: 470, r: -1 },
      ]}
    >
      <Scene speed={1.25}>
        <Usage bare scrollTo={180} />
      </Scene>
    </Camera>
  </AbsoluteFill>
);

const CAROUSEL: { name: string; p: () => Palette }[] = [
  { name: "Fluent Dark", p: () => fluent(true) },
  { name: "Fluent Light", p: () => fluent(false) },
  { name: "Vercel", p: () => vercel(true) },
  { name: "Dracula", p: () => dracula() },
  { name: "Solarized Light", p: () => solarized() },
];
const THEME_HEIGHT = 836;
/// Beats per carousel step.
const THEME_BEATS = 2;

/// Three differently themed popups; every two beats the row moves right, the
/// left one takes focus and a new theme enters on the left.
const ThemesScene: React.FC<{ t: number }> = ({ t }) => {
  const step = Math.max(0, Math.floor(t / (THEME_BEATS * B)));
  const phase = step + ease.outExpo((t - step * THEME_BEATS * B) / 0.5);
  // Item k sits at position phase - k: 0 left, 1 focused, 2 right.
  const items = [];
  for (let k = Math.floor(phase) - 3; k <= Math.ceil(phase) + 1; k++) {
    const pos = phase - k;
    if (pos <= -1 || pos >= 3) continue;
    items.push({ k, pos });
  }
  const focused = Math.round(phase - 1);
  const theme = (k: number) => CAROUSEL[(((k + 1) % CAROUSEL.length) + CAROUSEL.length) % CAROUSEL.length];
  const centerX = 1300;
  const spacing = 330;
  return (
    <AbsoluteFill>
      <Headline
        side={1}
        t={t}
        lines={[
          { text: "Any theme.", beat: 0.2, accent: ["theme."] },
          { text: "Use your favorite VSCode theme!", beat: 2, size: 34, dim: true },
        ]}
      />
      <div style={{ position: "absolute", left: 130, top: 385, fontFamily: SEGOE, color: ORANGE }}>
        <PopWord key={focused} word={theme(focused).name} at={step * THEME_BEATS * B} t={t} size={54} />
      </div>
      <AbsoluteFill style={{ perspective: 1800 }}>
        {items
          .sort((a, b) => Math.abs(b.pos - 1) - Math.abs(a.pos - 1))
          .map(({ k, pos }) => {
            const p = theme(k).p();
            const off = pos - 1;
            const near = Math.min(1, Math.abs(off));
            const scale = lerp(0.9, 0.7, near);
            const fade = pos < 0 ? pos + 1 : pos > 2 ? 3 - pos : 1;
            return (
              <div
                key={k}
                style={{
                  position: "absolute",
                  left: centerX + off * spacing - POPUP_WIDTH / 2,
                  top: 540 - THEME_HEIGHT / 2,
                  transform: `scale(${scale}) rotateY(${-Math.max(-1, Math.min(1, off)) * 22}deg)`,
                  opacity: fade,
                }}
              >
                <PopupFrame p={p} height={THEME_HEIGHT} footer={<Footer p={p} active={0} />}>
                  <HomePage p={p} s={STRINGS.en.s} sections={sections()} limit={3} />
                </PopupFrame>
                {/* Side popups sit back in the dark. */}
                <div
                  style={{
                    position: "absolute",
                    inset: 0,
                    borderRadius: 8,
                    background: "#05050c",
                    opacity: near * 0.45,
                  }}
                />
              </div>
            );
          })}
      </AbsoluteFill>
    </AbsoluteFill>
  );
};

const HELLO = ["Hello", "Привет", "Hallo", "Bonjour", "Hola", "Olá", "こんにちは", "你好"];

const LANG_HEIGHT = 720;

const LanguagesScene: React.FC<{ t: number }> = ({ t }) => {
  const step = Math.min(7, Math.max(0, Math.floor(t / B)));
  const fade = ease.outExpo((t - step * B) / 0.2);
  const p = fluent(true);
  return (
    <AbsoluteFill>
      <div
        style={{
          position: "absolute",
          top: 0,
          bottom: 0,
          left: 130,
          width: 800,
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          fontFamily: SEGOE,
          color: "#fff",
        }}
      >
        <PopWord key={step} word={HELLO[step]} at={step * B} t={t} size={140} />
        <div style={{ fontSize: 60, fontWeight: 800, letterSpacing: -2, marginTop: 10 }}>
          <span style={{ color: ORANGE }}>8</span> languages.
        </div>
        <div style={{ fontSize: 34, color: "rgba(255,255,255,0.7)", marginTop: 18 }}>Follows your Windows language.</div>
      </div>
      <Camera
        t={t}
        side={1}
        width={POPUP_WIDTH}
        height={LANG_HEIGHT}
        punch={0.01}
        keys={[
          { beat: 0, s: 1.2, x: 190, y: 360, r: -2 },
          { beat: 8, s: 1.1, x: 190, y: 380, r: 2 },
        ]}
      >
        <PopupFrame p={p} height={LANG_HEIGHT} footer={<Footer p={p} active={0} />}>
          {/* Same beat as the greeting: a clean swap that settles into place. */}
          <div style={{ position: "absolute", inset: 0, transform: `translateY(${step > 0 ? (1 - fade) * 8 : 0}px)` }}>
            <HomePage p={p} s={STRINGS[LANGS[step]].s} sections={sections()} limit={2} />
          </div>
        </PopupFrame>
      </Camera>
    </AbsoluteFill>
  );
};

/// A word that pops in on its beat; the caller keys it per beat.
const PopWord: React.FC<{ word: string; at: number; t: number; size: number; glow?: boolean }> = ({
  word,
  at,
  t,
  size,
  glow,
}) => {
  const k = useSpringAt(at, { damping: 12, stiffness: 260, mass: 0.5 });
  return (
    <div
      style={{
        fontSize: size,
        fontWeight: 800,
        letterSpacing: -size / 35,
        lineHeight: 1.1,
        transform: `translateY(${(1 - k) * 70}px) scale(${lerp(1.25, 1, Math.min(1, k))})`,
        transformOrigin: "0% 80%",
        opacity: Math.min(1, k * 2),
        filter: k < 0.8 ? `blur(${(1 - k) * 12}px)` : undefined,
        textShadow: glow ? `0 0 ${40 * pulse(t)}px rgba(251,154,68,0.6)` : undefined,
      }}
    >
      {word}
    </div>
  );
};

/// Last showcase phrase: a tilted wall of popups sliding past, light and
/// dark themes alternating like a checkerboard.
const WALL_COLS = 10;
const WALL_ROWS = 4;
const WALL_LIGHT = [() => fluent(false), () => solarized()];
const WALL_DARK = [() => fluent(true), () => vercel(true), () => dracula()];
const WALL = Array.from({ length: WALL_COLS * WALL_ROWS }, (_, i) => {
  const row = Math.floor(i / WALL_COLS);
  const col = i % WALL_COLS;
  const light = (row + col) % 2 === 0;
  return {
    p: light ? WALL_LIGHT[(row + Math.floor(col / 2)) % 2] : WALL_DARK[(row + Math.floor(col / 2)) % 3],
    page: (row * 2 + col) % 3 === 0 ? ("usage" as const) : ("home" as const),
    lang: LANGS[(row * 3 + col * 5) % LANGS.length],
  };
});

const Wall: React.FC<{ t: number }> = ({ t }) => {
  const k = useSpringAt(0, { damping: 18, stiffness: 90 });
  const travel = t * 120;
  return (
    <AbsoluteFill>
      <AbsoluteFill style={{ perspective: 1400 }}>
        <div
          style={{
            position: "absolute",
            // Wide enough that the drift never uncovers a corner.
            left: -1100,
            top: -1000,
            width: 4400,
            height: 3400,
            transform: `rotateX(28deg) rotateZ(-16deg) translate(${-travel}px, ${-travel * 0.4}px) scale(${lerp(1.4, 0.95, k) * (1 + pulse(t) * 0.01)})`,
            transformOrigin: "50% 40%",
            display: "grid",
            gridTemplateColumns: `repeat(${WALL_COLS}, ${POPUP_WIDTH}px)`,
            gap: 40,
            alignContent: "start",
          }}
        >
          {WALL.map((item, i) => {
            const p = item.p();
            return (
              <div key={i} style={{ transform: `translateY(${Math.sin(t * 1.6 + i) * 16}px)` }}>
                <PopupFrame p={p} height={740} footer={<Footer p={p} active={item.page === "home" ? 0 : 1} />}>
                  {item.page === "home" ? (
                    <HomePage p={p} s={STRINGS[item.lang].s} sections={sections()} limit={3} />
                  ) : (
                    <UsagePage p={p} />
                  )}
                </PopupFrame>
              </div>
            );
          })}
        </div>
      </AbsoluteFill>
      <AbsoluteFill style={{ background: "radial-gradient(circle at 50% 50%, rgba(8,8,18,0.82) 0%, rgba(8,8,18,0.35) 45%, transparent 75%)" }} />
      <AbsoluteFill style={{ alignItems: "center", justifyContent: "center", flexDirection: "column", fontFamily: SEGOE, color: "#fff" }}>
        <BigLine text="All your AI." at={0.2} t={t} />
        <BigLine text="One tray icon." at={2 * B} t={t} accent />
      </AbsoluteFill>
    </AbsoluteFill>
  );
};

const BigLine: React.FC<{ text: string; at: number; t: number; accent?: boolean }> = ({ text, at, t, accent }) => {
  const k = useSpringAt(at, { damping: 12, stiffness: 220, mass: 0.6 });
  if (t < at) return <div style={{ height: 150 }} />;
  return (
    <div
      style={{
        fontSize: 140,
        fontWeight: 800,
        letterSpacing: -5,
        lineHeight: 1.05,
        color: accent ? ORANGE : "#fff",
        transform: `scale(${lerp(1.6, 1, Math.min(1, k)) * (1 + pulse(t) * 0.02)})`,
        opacity: Math.min(1, k * 2),
        filter: k < 0.8 ? `blur(${(1 - k) * 16}px)` : undefined,
        textShadow: "0 10px 50px rgba(0,0,0,0.6)",
      }}
    >
      {text}
    </div>
  );
};

/* ------------------------------------------------------------------ ending */

const URL = "github.com/Truman-fan/agent-minibar";

/// A short end card while the music falls into its breakdown.
const EndCard: React.FC<{ t: number }> = ({ t }) => {
  const rise = (at: number) => useSpringAt(at, { damping: 20, stiffness: 140, mass: 0.8 });
  const icon = rise(0);
  const name = rise(0.08);
  const tagline = rise(0.16);
  const link = rise(0.24);
  const fade = 1 - ease.inCubic((t / B - (END - BREAKDOWN - 1.6)) / 1.6);
  const up = (k: number) => ({ opacity: Math.min(1, k * 1.5), transform: `translateY(${(1 - k) * 40}px)` });
  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        flexDirection: "column",
        fontFamily: SEGOE,
        color: "#fff",
        opacity: fade,
      }}
    >
      <Img src={staticFile("app-icon.png")} style={{ width: 170, height: 170, ...up(icon) }} />
      <div style={{ fontSize: 112, fontWeight: 800, letterSpacing: -3.5, marginTop: 24, ...up(name) }}>Agent Minibar</div>
      <div style={{ fontSize: 38, color: "rgba(255,255,255,0.72)", marginTop: 4, ...up(tagline) }}>
        Free and open source for Windows 10 and 11
      </div>
      <div
        style={{
          marginTop: 40,
          height: 76,
          padding: "0 32px",
          borderRadius: 18,
          display: "flex",
          alignItems: "center",
          gap: 16,
          background: "rgba(255,255,255,0.08)",
          border: "1px solid rgba(255,255,255,0.16)",
          fontSize: 34,
          fontWeight: 600,
          ...up(link),
        }}
      >
        <Icon name="ph-github-logo-fill" size={36} color="#fff" />
        {URL}
      </div>
    </AbsoluteFill>
  );
};

/* ------------------------------------------------------------------ video */

const MUSIC = staticFile("music/electro-dreams.mp3");
const VOLUME = 0.75;

const Music: React.FC = () => {
  const { fps } = useVideoConfig();
  const loop = MUSIC_LOOP_AT * B;
  const end = END * B;
  // A short crossfade hides the jump at the phrase boundary.
  const xfade = 0.04;
  return (
    <>
      <Sequence durationInFrames={Math.round((loop + xfade) * fps)} layout="none">
        <Audio
          src={MUSIC}
          trimBefore={Math.round((TRACK_PHASE + TRACK_START * B) * fps)}
          volume={(f) => interpolate(f / fps, [0, 0.1, loop, loop + xfade], [0, VOLUME, VOLUME, 0], clamp)}
        />
      </Sequence>
      <Sequence from={Math.round(loop * fps)} layout="none">
        <Audio
          src={MUSIC}
          trimBefore={Math.round((TRACK_PHASE + (TRACK_START + MUSIC_LOOP_AT - MUSIC_LOOP_BACK) * B) * fps)}
          volume={(f) => interpolate(f / fps, [0, xfade, end - loop - 2.4, end - loop], [0, VOLUME, VOLUME, 0], clamp)}
        />
      </Sequence>
    </>
  );
};

const LaunchBody: React.FC = () => {
  const t = useSeconds();
  const beat = t / B;
  const { fps } = useVideoConfig();
  const energy = beat < DROP ? 0.4 : beat < BREAKDOWN ? 1 : 0.25;
  const tint = beat < WALL_AT ? color(PROVIDERS[Math.max(0, FLASH_BEATS.filter((b) => b <= beat).length - 1)].id) : undefined;
  const sh = shake(t, DROP * B, 6);
  const showcase: { start: number; side: 1 | -1; render: (t: number) => React.ReactNode }[] = [
    { start: DROP + 8, side: -1, render: (t) => <Accounts t={t} /> },
    { start: OPENROUTER_AT, side: 1, render: (t) => <OpenRouterScene t={t} /> },
    { start: ACTIVATION_AT, side: -1, render: (t) => <ActivationScene t={t} /> },
    { start: NOTIFICATIONS_AT, side: 1, render: (t) => <Notifications t={t} /> },
    { start: DROP + 40, side: -1, render: (t) => <UsageScene t={t} /> },
    { start: DROP + 48, side: 1, render: (t) => <ThemesScene t={t} /> },
    { start: DROP + 56, side: -1, render: (t) => <LanguagesScene t={t} /> },
    { start: DROP + 64, side: 1, render: (t) => <Wall t={t} /> },
  ];
  return (
    <AbsoluteFill style={{ background: "#000" }}>
      <Music />
      <Backdrop t={t} tint={tint} energy={energy} />
      <AbsoluteFill style={{ transform: `translate(${sh.x}px, ${sh.y}px) rotate(${sh.r}deg)` }}>
        <Sequence durationInFrames={Math.round(DROP * B * fps)}>
          <Hook t={t} />
        </Sequence>
        <Phrase start={DROP} side={1} slam>
          {(t) => <OpenApp t={t} />}
        </Phrase>
        {showcase.map((scene) => (
          <Phrase key={scene.start} start={scene.start} side={scene.side}>
            {scene.render}
          </Phrase>
        ))}
        <Sequence from={Math.round(BREAKDOWN * B * fps)}>
          <EndCardAt />
        </Sequence>
        {/* UI sounds land with their toasts. */}
        <Cues start={NOTIFICATIONS_AT} cues={NOTIFY} />
        <Cues start={ACTIVATION_AT} cues={ACTIVATION_CUES} />
      </AbsoluteFill>
      <Flash t={t} at={DROP * B} strength={0.4} />
    </AbsoluteFill>
  );
};

export const Launch: React.FC = () => (
  <FontGate>
    <LaunchBody />
  </FontGate>
);

const EndCardAt: React.FC = () => <EndCard t={useSeconds()} />;

/// UI sounds at scene-beat offsets from `start` (video beats).
const Cues: React.FC<{ start: number; cues: { beat: number; sound: string }[] }> = ({ start, cues }) => {
  const { fps } = useVideoConfig();
  return (
    <>
      {cues.map(({ beat, sound }) => (
        <Sequence key={beat} from={Math.round((start + beat) * B * fps)} layout="none">
          <Audio src={staticFile(`sounds/${sound}.wav`)} volume={0.45} />
        </Sequence>
      ))}
    </>
  );
};

/* -------------------------------------------------- patch notes: 16:9 media */

/// Chapters of the video rendered on their own for the release notes.
export const CHAPTERS = {
  accounts: Accounts,
  openrouter: OpenRouterScene,
  activation: ActivationScene,
  notifications: Notifications,
  usage: UsageScene,
  themes: ThemesScene,
  languages: LanguagesScene,
} as const;
export type ChapterId = keyof typeof CHAPTERS;
/// One phrase plus a beat to read the last state.
export const CHAPTER_SECONDS = 9 * B;

const ChapterBody: React.FC<{ id: ChapterId }> = ({ id }) => {
  const t = useSeconds();
  const Scene = CHAPTERS[id];
  return (
    <AbsoluteFill style={{ background: "#000" }}>
      <Backdrop t={t} energy={0.6} />
      <Scene t={t} />
    </AbsoluteFill>
  );
};

export const Chapter: React.FC<{ id: ChapterId }> = ({ id }) => (
  <FontGate>
    <ChapterBody id={id} />
  </FontGate>
);

/// Release-notes hero: Usage, Home and the Claude tab side by side.
const HeroBody: React.FC = () => {
  const p = fluent(true);
  const side = (rotate: number): React.CSSProperties => ({
    position: "absolute",
    transform: `perspective(1800px) rotateY(${rotate}deg) scale(0.94)`,
  });
  return (
    <AbsoluteFill style={{ background: "#000" }}>
      <Backdrop t={3} energy={0} />
      <div style={{ ...side(16), left: 340, top: 190 }}>
        <PopupFrame p={p} height={712} footer={<Footer p={p} active={1} />}>
          <UsagePage p={p} />
        </PopupFrame>
      </div>
      <div style={{ ...side(-16), left: 1200, top: 230 }}>
        <PopupFrame p={p} height={CLAUDE_TAB_HEIGHT} footer={<Footer p={p} active={3} />}>
          <ClaudeTab p={p} draw={1} />
        </PopupFrame>
      </div>
      <div style={{ position: "absolute", left: 770, top: 65 }}>
        <PopupFrame p={p} height={HOME_POPUP_HEIGHT} footer={<Footer p={p} active={0} />}>
          <HomePage p={p} s={STRINGS.en.s} sections={sections()} />
        </PopupFrame>
      </div>
      <div style={{ position: "absolute", left: 1290, top: 700, width: 392, height: 300 }}>
        <ToastStack toasts={[beatTimeline()[1]]} time={5} p={p} s={STRINGS.en.s} height={300} />
      </div>
    </AbsoluteFill>
  );
};

export const Hero: React.FC = () => (
  <FontGate>
    <HeroBody />
  </FontGate>
);
