import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../ui/global.css";
import { OverlayApp } from "./OverlayApp";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Overlay window root is missing");
}

createRoot(root).render(
  <StrictMode>
    <OverlayApp />
  </StrictMode>,
);
