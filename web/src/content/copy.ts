export { githubReleasesHref as primaryDownloadHref } from "../lib/site";

export const copy = {
  pitch:
    "A private, local-first Windows companion that helps you remember MTGO opponents and review verifiable public context — without becoming an MTGO client.",
  contrast:
    "We store your observations. We do not log the board. This is not a board logger.",
  downloadLabel: "Download for Windows",
  requirements:
    "Windows 10 22H2 / Windows 11 x64. Unzip and run MTGONotes.App.exe.",
  liveAttachHint:
    "Live attach is optional. Log into MTGO before launching if you want it.",
  beats: [
    {
      title: "confirm opponent",
      body: "Match the nickname you see before anything is saved.",
    },
    {
      title: "fast capture",
      body: "Write a note in under five seconds between games.",
    },
    {
      title: "recall between games",
      body: "Read history when you are not playing, not during a match.",
    },
  ],
  howItWorks: {
    notebook: "Notes live in a local notebook on your machine.",
    confirm:
      "The companion asks you to confirm before it persists an opponent or note.",
    overlay: "The overlay is click-through so it does not steal the match.",
    hiddenHistory:
      "History stays hidden during possible gameplay and is for review between games.",
    publicPreview:
      "A consented public-result preview is a published snapshot, not a live deck.",
    screenshotCaption: "Screenshot forthcoming",
  },
  liveAttach: {
    off: "Optional. Off means manual notes only.",
    on: "On is a read-only attach to an already-logged-in client.",
    noLogin:
      "It does not call LogOn and does not send a password, chat, queue, or concede.",
    videre:
      "It is the same class of process inspect as Videre; we read less.",
    risk:
      "Unofficial. Daybreak may still terminate accounts under their EULA. Not legal advice. Not affiliated. Not tournament-approved.",
  },
  privacy: {
    noSignup: "There is no signup and no visitor account.",
    telemetry: "There is no telemetry.",
    local: "Notes stay on the machine.",
    backups: "Backups are user-made.",
    exportWarning: "Text export is unencrypted. Treat exported files as plaintext.",
  },
} as const;

const forbidden = [
  "tournament-safe",
  "ban-proof",
  "auto-updater",
  "their current deck",
] as const;

const thirdPartyHosts = [
  "googletagmanager.com",
  "google-analytics.com",
  "fonts.googleapis.com",
  "plausible.io",
  "cloudflareinsights.com",
  "doubleclick.net",
] as const;

export function assertAllowedCopy(sample: string): void {
  const haystack = sample.toLowerCase();
  for (const term of forbidden) {
    if (haystack.includes(term)) {
      throw new Error(`Forbidden copy: ${term}`);
    }
  }
}

export function findForbiddenThirdParty(html: string): string[] {
  const haystack = html.toLowerCase();
  return thirdPartyHosts.filter((host) => haystack.includes(host));
}
