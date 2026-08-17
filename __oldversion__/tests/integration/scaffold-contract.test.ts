import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

type WindowConfig = {
  label: string;
  url: string;
  width: number;
  height: number;
  minWidth?: number;
  minHeight?: number;
  visible: boolean;
  focus: boolean;
  alwaysOnTop?: boolean;
  skipTaskbar?: boolean;
  resizable?: boolean;
};

type TauriConfig = {
  app: {
    windows: WindowConfig[];
    security: {
      csp: string;
      freezePrototype: boolean;
      assetProtocol: { enable: boolean };
    };
  };
  bundle: {
    targets: string[];
    icon: string[];
    resources: string[];
    windows: {
      webviewInstallMode: { type: string; silent: boolean };
      nsis: { installerIcon: string; installMode: string };
    };
  };
};

const root = process.cwd();
const config = JSON.parse(
  readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"),
) as TauriConfig;

test("UT-100: three isolated window entrypoints include a non-activating automatic overlay", () => {
  const windows = Object.fromEntries(
    config.app.windows.map((window) => [window.label, window]),
  );

  expect(windows.main).toMatchObject({
    url: "index.html",
    minWidth: 960,
    minHeight: 680,
    visible: true,
    focus: true,
  });
  expect(windows.overlay).toMatchObject({
    url: "overlay.html",
    width: 360,
    height: 220,
    visible: false,
    focus: false,
    alwaysOnTop: true,
    skipTaskbar: true,
  });
  expect(windows.capture).toMatchObject({
    url: "capture.html",
    width: 420,
    height: 160,
    visible: false,
    focus: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
  });

  for (const html of ["index.html", "overlay.html", "capture.html"]) {
    expect(existsSync(resolve(root, html))).toBe(true);
  }
});

test("packaging and webview security remain local and Windows-scoped", () => {
  expect(config.app.security).toMatchObject({
    freezePrototype: true,
    assetProtocol: { enable: false },
  });
  expect(config.app.security.csp).toContain("default-src 'self'");
  expect(config.app.security.csp).toContain("object-src 'none'");
  expect(config.app.security.csp).toContain("frame-src 'none'");
  expect(config.bundle.targets).toEqual(["nsis"]);
  expect(config.bundle.icon).toContain("../assets/icons/mtgo-notes.ico");
  expect(config.bundle.windows.webviewInstallMode).toEqual({
    type: "downloadBootstrapper",
    silent: true,
  });
  expect(config.bundle.resources).toContain("../THIRD_PARTY_NOTICES.md");
  expect(config.bundle.resources).toContain(
    "../assets/fonts/Inter-OFL-1.1.txt",
  );
});

test("Inter assets, license notice, supplied icons, and Windows CI are present", () => {
  const styles = readFileSync(resolve(root, "src/ui/global.css"), "utf8");
  const notice = readFileSync(resolve(root, "THIRD_PARTY_NOTICES.md"), "utf8");
  const fontLicense = readFileSync(
    resolve(root, "assets/fonts/Inter-OFL-1.1.txt"),
    "utf8",
  );
  const workflow = readFileSync(
    resolve(root, ".github/workflows/windows.yml"),
    "utf8",
  );

  expect(styles).toContain('@import "@fontsource-variable/inter/wght.css"');
  expect(notice).toContain("SIL Open Font License, Version 1.1");
  expect(fontLicense).toContain("SIL OPEN FONT LICENSE Version 1.1");
  expect(existsSync(resolve(root, "assets/icons/mtgo-notes.ico"))).toBe(true);
  expect(workflow).toContain("windows-latest");
  expect(workflow).toContain("x86_64-pc-windows-msvc");
  expect(workflow).toContain("npm ci");
});
