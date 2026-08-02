import { readFileSync } from "node:fs";
import { resolve } from "node:path";

describe("Task 07 packaged Windows release contract", () => {
  const workflow = readFileSync(
    resolve(process.cwd(), ".github/workflows/windows.yml"),
    "utf8",
  );
  const validator = readFileSync(
    resolve(process.cwd(), "tests/release/validate-windows.ps1"),
    "utf8",
  );
  const evidence = readFileSync(
    resolve(process.cwd(), "tests/release/MANUAL_EVIDENCE.md"),
    "utf8",
  );

  test("Windows 10 22H2 and Windows 11 x64 require separate packaged evidence", () => {
    expect(workflow).toContain("workflow_dispatch:");
    expect(workflow).toContain("windows-10-22h2");
    expect(workflow).toContain("windows-11");
    expect(workflow).toContain("npm run verify");
    expect(workflow).toContain("npm run build:windows");
    expect(workflow).toContain("validate-windows.ps1");
  });

  test("release validation fails closed on signing and manual evidence", () => {
    expect(validator).toContain("MTGO_NOTES_REQUIRE_PRODUCTION_SIGNATURE");
    expect(validator).toContain("MTGO_NOTES_MANUAL_EVIDENCE_COMPLETE");
    expect(validator).toContain("Get-AuthenticodeSignature");
    expect(evidence).toContain("Status: BLOCKED");
    expect(evidence).toContain("production Authenticode identity");
  });
});
