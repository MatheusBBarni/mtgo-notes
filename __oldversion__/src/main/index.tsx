import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../ui/global.css";
import { MainApp } from "./MainApp";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Main window root is missing");
}

createRoot(root).render(
  <StrictMode>
    <MainApp />
  </StrictMode>,
);
