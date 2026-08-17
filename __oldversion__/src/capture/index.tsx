import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../ui/global.css";
import { CaptureApp } from "./CaptureApp";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Capture window root is missing");
}

createRoot(root).render(
  <StrictMode>
    <CaptureApp />
  </StrictMode>,
);
