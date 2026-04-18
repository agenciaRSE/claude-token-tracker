/** Procedurally-generated notification sounds using the Web Audio API.
 *  Keeping the sounds as tone patterns (rather than bundled MP3/WAV files)
 *  avoids shipping binary assets in the installer and lets us tune the
 *  feel of each alert in code. */

export type SoundId =
  | "none"
  | "chime"
  | "bell"
  | "ping"
  | "alert"
  | "pulse"
  | "success"
  | "warning";

export const SOUND_LABELS: Record<SoundId, string> = {
  none: "None (silent)",
  chime: "Chime (two notes)",
  bell: "Bell (long decay)",
  ping: "Ping (short)",
  alert: "Alert (double beep)",
  pulse: "Pulse (ascending)",
  success: "Success (up arpeggio)",
  warning: "Warning (descending)",
};

/** Ordered list for rendering in dropdowns. */
export const SOUND_IDS: SoundId[] = [
  "chime",
  "bell",
  "ping",
  "alert",
  "pulse",
  "success",
  "warning",
  "none",
];

/** Lazy-init a single shared AudioContext so we don't leak one per play().
 *  Browsers / Tauri webview gate AudioContext creation behind a user gesture
 *  on some platforms — first play() may fail silently if no interaction has
 *  happened yet. Settings panel's Preview button resolves that for picks. */
let ctx: AudioContext | null = null;
function getCtx(): AudioContext | null {
  if (typeof window === "undefined") return null;
  if (ctx && ctx.state !== "closed") return ctx;
  try {
    // @ts-expect-error webkit fallback for older engines
    const Ctor: typeof AudioContext = window.AudioContext || window.webkitAudioContext;
    ctx = new Ctor();
    return ctx;
  } catch {
    return null;
  }
}

interface ToneOptions {
  freq: number;
  startDelay: number; // seconds from now
  duration: number;
  volume: number; // 0-1
  type?: OscillatorType;
  /** If true, apply exponential decay for a "bell"-like envelope. */
  bell?: boolean;
}

function playTone(audioCtx: AudioContext, opts: ToneOptions) {
  const osc = audioCtx.createOscillator();
  const gain = audioCtx.createGain();
  osc.type = opts.type ?? "sine";
  osc.frequency.value = opts.freq;
  osc.connect(gain);
  gain.connect(audioCtx.destination);

  const now = audioCtx.currentTime + opts.startDelay;
  const vol = Math.max(0, Math.min(1, opts.volume));

  // Short attack to avoid click artifacts.
  gain.gain.setValueAtTime(0, now);
  gain.gain.linearRampToValueAtTime(vol, now + 0.005);

  if (opts.bell) {
    // Long exponential decay.
    gain.gain.exponentialRampToValueAtTime(
      0.001,
      now + opts.duration,
    );
  } else {
    // Flat sustain with a quick release at the end.
    gain.gain.setValueAtTime(vol, now + Math.max(0, opts.duration - 0.03));
    gain.gain.linearRampToValueAtTime(0, now + opts.duration);
  }

  osc.start(now);
  osc.stop(now + opts.duration + 0.02);
}

/** Play a named preset at the given volume (0-100 → 0-1). */
export function playSound(id: SoundId, volumePct: number = 70): void {
  if (id === "none") return;
  const audioCtx = getCtx();
  if (!audioCtx) return;

  const vol = Math.max(0, Math.min(100, volumePct)) / 100;

  // If the context was suspended (e.g. no user gesture yet), try to resume.
  // This is best-effort — if it fails, the tone simply won't sound.
  if (audioCtx.state === "suspended") {
    audioCtx.resume().catch(() => {});
  }

  switch (id) {
    case "chime": {
      // Pleasant two-note chime: E5 → C6
      playTone(audioCtx, { freq: 659.25, startDelay: 0, duration: 0.18, volume: vol, type: "sine", bell: true });
      playTone(audioCtx, { freq: 1046.5, startDelay: 0.12, duration: 0.35, volume: vol, type: "sine", bell: true });
      break;
    }
    case "bell": {
      // Single long bell-like tone
      playTone(audioCtx, { freq: 880, startDelay: 0, duration: 0.7, volume: vol, type: "triangle", bell: true });
      playTone(audioCtx, { freq: 1760, startDelay: 0, duration: 0.5, volume: vol * 0.4, type: "sine", bell: true });
      break;
    }
    case "ping": {
      // Single short ping (high frequency)
      playTone(audioCtx, { freq: 1200, startDelay: 0, duration: 0.09, volume: vol, type: "sine", bell: true });
      break;
    }
    case "alert": {
      // Two identical short beeps — attention-grabbing without being harsh
      playTone(audioCtx, { freq: 830, startDelay: 0, duration: 0.12, volume: vol, type: "square" });
      playTone(audioCtx, { freq: 830, startDelay: 0.2, duration: 0.22, volume: vol, type: "square" });
      break;
    }
    case "pulse": {
      // Ascending three-tone pulse
      playTone(audioCtx, { freq: 523.25, startDelay: 0, duration: 0.09, volume: vol, type: "sine" });
      playTone(audioCtx, { freq: 659.25, startDelay: 0.1, duration: 0.09, volume: vol, type: "sine" });
      playTone(audioCtx, { freq: 783.99, startDelay: 0.2, duration: 0.15, volume: vol, type: "sine", bell: true });
      break;
    }
    case "success": {
      // C-E-G major arpeggio
      playTone(audioCtx, { freq: 523.25, startDelay: 0, duration: 0.1, volume: vol, type: "sine" });
      playTone(audioCtx, { freq: 659.25, startDelay: 0.1, duration: 0.1, volume: vol, type: "sine" });
      playTone(audioCtx, { freq: 783.99, startDelay: 0.2, duration: 0.25, volume: vol, type: "sine", bell: true });
      break;
    }
    case "warning": {
      // Descending two-tone warning
      playTone(audioCtx, { freq: 880, startDelay: 0, duration: 0.15, volume: vol, type: "sawtooth" });
      playTone(audioCtx, { freq: 587.33, startDelay: 0.15, duration: 0.3, volume: vol, type: "sawtooth", bell: true });
      break;
    }
  }
}

/** Validate a string coming from a settings store against the known SoundIds. */
export function isValidSoundId(s: string): s is SoundId {
  return (SOUND_IDS as string[]).includes(s) || s === "none";
}
