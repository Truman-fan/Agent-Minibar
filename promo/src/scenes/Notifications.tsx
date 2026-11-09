import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Fonts } from "../ui/Desktop";
import { CARD_WIDTH, EDGE, Timed, ToastSpec, ToastStack } from "../ui/Toast";
import { Palette, fluent } from "../ui/palette";
import { STRINGS } from "../data";

export const NOTIFICATIONS_SIZE = { width: CARD_WIDTH + EDGE * 2, height: 560 };
export const NOTIFICATIONS_SECONDS = 9;

const TOTAL_MB = 22.7;

const download = (fraction: number): ToastSpec => ({
  id: "update",
  kind: "update",
  title: "Downloading update",
  body: `${(TOTAL_MB * fraction).toFixed(1)} MB of ${TOTAL_MB.toFixed(1)} MB`,
  progress: fraction,
});

export const notificationTimeline = (offset = 0): Timed[] => [
  {
    at: offset + 0.3,
    leave: offset + 8.0,
    spec: {
      id: "low",
      kind: "warning",
      title: "Weekly limit is low",
      alert: {
        provider: "claude",
        name: "Claude",
        plan: "Max",
        account: "Personal",
        rows: [
          { q: { id: "a", title: "session", left: 41, reset: "1h 12m", ticks: 4 }, focused: false },
          { q: { id: "b", title: "weekly", left: 5, reset: "2d 3h", ticks: 6 }, focused: true },
        ],
      },
    },
  },
  {
    at: offset + 1.6,
    leave: offset + 8.12,
    spec: {
      id: "started",
      kind: "success",
      title: "5-hour limit started",
      alert: {
        provider: "codex",
        name: "Codex",
        plan: "Pro",
        account: "Truman-fan",
        rows: [
          { q: { id: "c", title: "session", left: 100, reset: "4h 59m", ticks: 4 }, focused: true },
          { q: { id: "d", title: "weekly", left: 57, reset: "5d 2h", ticks: 6 }, focused: false },
        ],
      },
    },
  },
  {
    at: offset + 3.0,
    leave: offset + 8.24,
    spec: {
      id: "update",
      kind: "update",
      title: "Update Available",
      body: "Agent Minibar 1.0.0. A new release is ready to install.",
      bodyLines: 2,
      actions: ["Update now", "What's new"],
    },
    replaces: [
      {
        at: offset + 4.9,
        spec: { id: "update", kind: "update", title: "Downloading update", body: "Agent Minibar 1.0.0", progress: "sweep" },
      },
      ...Array.from({ length: 25 }, (_, i) => ({
        at: offset + 5.7 + i * 0.06,
        spec: download(Math.min(1, ((i + 1) / 25) ** 0.9)),
      })),
      {
        at: offset + 7.3,
        spec: {
          id: "update",
          kind: "success",
          title: "Restarting Agent Minibar",
          body: "The app will reopen in a moment.",
          progress: 1,
        },
      },
    ],
  },
];

export const NotificationSounds: React.FC<{ offset?: number; speed?: number }> = ({ offset = 0, speed = 1 }) => {
  const { fps } = useVideoConfig();
  return (
    <>
      {[
        { at: 0.3, sound: "warning" },
        { at: 1.6, sound: "success" },
        { at: 3.0, sound: "update" },
        { at: 7.3, sound: "success" },
      ].map(({ at, sound }) => (
        <Sequence key={at} from={Math.round(((offset + at) / speed) * fps)} layout="none">
          <Audio src={staticFile(`sounds/${sound}.wav`)} volume={0.6} />
        </Sequence>
      ))}
    </>
  );
};

/// Transparent-background notification stack for patch notes.
export const Notifications: React.FC<{ dark?: boolean; palette?: Palette }> = ({ dark = true, palette }) => {
  const frame = useCurrentFrame();
  const { fps, height } = useVideoConfig();
  const p = palette ?? fluent(dark);
  return (
    <AbsoluteFill>
      <Fonts />
      <ToastStack toasts={notificationTimeline()} time={frame / fps} p={p} s={STRINGS.en.s} height={height} />
      <NotificationSounds />
    </AbsoluteFill>
  );
};
