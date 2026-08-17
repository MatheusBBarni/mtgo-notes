export const primaryDownloadHref = "/download/windows";

export const copy = {
  pitch:
    "A private, local-first Windows companion that helps you remember MTGO opponents and review verifiable public context — without becoming an MTGO client.",
  contrast:
    "We store your observations. We do not log the board. This is not a board logger.",
  downloadLabel: "Download for Windows",
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
