import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import axe from "axe-core";
import { JSDOM, VirtualConsole } from "jsdom";

const prototypePath = fileURLToPath(new URL("./index.html", import.meta.url));
const html = await readFile(prototypePath, "utf8");
const runtimeErrors = [];
const virtualConsole = new VirtualConsole();

virtualConsole.on("jsdomError", (error) => runtimeErrors.push(error.message));

const dom = new JSDOM(html, {
  pretendToBeVisual: true,
  runScripts: "dangerously",
  url: "https://prototype.local/player",
  virtualConsole,
});

dom.window.eval(axe.source);

const stateResults = [];
const violations = [];
const stateButtons = [...dom.window.document.querySelectorAll("[data-state]")];

for (const button of stateButtons) {
  button.click();
  const state = button.dataset.state;
  const status =
    dom.window.document.querySelector("#workspace-status")?.textContent;
  const results = await dom.window.axe.run(dom.window.document, {
    rules: {
      "color-contrast": { enabled: false },
    },
  });

  stateResults.push({ state, status, axePasses: results.passes.length });
  violations.push(
    ...results.violations.map(({ id, impact, nodes }) => ({
      state,
      id,
      impact,
      targets: nodes.map((node) => node.target),
    })),
  );
}

dom.window.document.querySelector('[data-state="candidates"]').click();
const candidateChecks = [
  ...dom.window.document.querySelectorAll(".candidate-check"),
];
const importButton = dom.window.document.querySelector("#import-button");

for (const checkbox of candidateChecks) {
  checkbox.click();
}

if (!importButton.disabled) {
  runtimeErrors.push("Import remained enabled with no selected candidates.");
}

candidateChecks[0].click();

if (importButton.disabled) {
  runtimeErrors.push("Import remained disabled after selecting a candidate.");
}

importButton.click();

if (
  dom.window.document.querySelector("#workspace-status")?.textContent !==
  "Results imported"
) {
  runtimeErrors.push("Import did not reach the imported state.");
}

if (runtimeErrors.length > 0 || violations.length > 0) {
  console.error(
    JSON.stringify(
      {
        runtimeErrors,
        violations,
      },
      null,
      2,
    ),
  );
  process.exitCode = 1;
} else {
  console.log(
    JSON.stringify({
      statesChecked: stateResults.length,
      minimumAxePasses: Math.min(
        ...stateResults.map((result) => result.axePasses),
      ),
      runtimeErrors: 0,
      violations: 0,
    }),
  );
}

dom.window.close();
