export const ReleaseKeys = {
  versioned: (version: string) =>
    `releases/windows/MTGONotes-${version}-win-x64.zip`,
  latestZip: "releases/windows/latest.zip",
  latestMeta: "releases/windows/latest.json",
} as const;
