import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";

import {
  findCapabilityViolations,
  type CapabilityManifest,
} from "../src/lib/security/capabilities";

const capabilityDirectory = resolve(
  import.meta.dirname,
  "../src-tauri/capabilities",
);

const failures = readdirSync(capabilityDirectory)
  .filter((fileName) => fileName.endsWith(".json"))
  .flatMap((fileName) => {
    const path = resolve(capabilityDirectory, fileName);
    const manifest = JSON.parse(
      readFileSync(path, "utf8"),
    ) as CapabilityManifest;
    return findCapabilityViolations(manifest).map(
      (violation) => `${fileName}: ${violation}`,
    );
  });

if (failures.length > 0) {
  process.stderr.write(`${failures.join("\n")}\n`);
  process.exitCode = 1;
} else {
  process.stdout.write("Capability policy: 3 manifests valid\n");
}
