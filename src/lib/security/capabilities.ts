export type CapabilityPermission =
  | string
  | {
      identifier: string;
      allow?: unknown[];
      deny?: unknown[];
    };

export type CapabilityManifest = {
  identifier: string;
  windows: string[];
  permissions: CapabilityPermission[];
  remote?: unknown;
};

export const DOCUMENTED_WINDOW_PERMISSIONS = {
  overlay: new Set([
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "core:window:allow-hide",
    "allow-bootstrap",
    "allow-pause-detection",
    "allow-confirm-opponent",
    "allow-correct-phase",
    "allow-open-capture",
    "allow-finish-encounter",
    "allow-undo-transition",
    "allow-set-overlay-interaction",
  ]),
  capture: new Set([
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "core:window:allow-hide",
    "allow-bootstrap",
    "allow-save-observation",
    "allow-discard-draft",
  ]),
} as const;

const FORBIDDEN_PERMISSION =
  /(^|:)(fs|filesystem|sql|shell|process|http|opener|updater)(:|$)|global-shortcut|create-webview-window/i;

export function permissionIdentifier(permission: CapabilityPermission): string {
  return typeof permission === "string" ? permission : permission.identifier;
}

export function findCapabilityViolations(
  manifest: CapabilityManifest,
): string[] {
  const violations: string[] = [];

  if (manifest.windows.length !== 1) {
    violations.push("a capability must target exactly one window label");
  }

  if (manifest.remote !== undefined) {
    violations.push("remote origins are forbidden");
  }

  for (const permission of manifest.permissions) {
    const identifier = permissionIdentifier(permission);
    if (identifier.includes("*")) {
      violations.push(`wildcard permission is forbidden: ${identifier}`);
    }
    if (FORBIDDEN_PERMISSION.test(identifier)) {
      violations.push(`forbidden renderer authority: ${identifier}`);
    }
  }

  const windowLabel = manifest.windows[0];
  if (windowLabel === "overlay" || windowLabel === "capture") {
    const expected = DOCUMENTED_WINDOW_PERMISSIONS[windowLabel];
    const actual = new Set(manifest.permissions.map(permissionIdentifier));
    const unexpected = [...actual].filter((item) => !expected.has(item));
    const missing = [...expected].filter((item) => !actual.has(item));

    for (const item of unexpected) {
      violations.push(`${windowLabel} has undocumented permission: ${item}`);
    }
    for (const item of missing) {
      violations.push(
        `${windowLabel} is missing documented permission: ${item}`,
      );
    }
  }

  return violations;
}
