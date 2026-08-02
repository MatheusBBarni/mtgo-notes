import { Card } from "@heroui/react";
import type { ReactNode } from "react";

export function Panel({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}) {
  return (
    <Card
      aria-label={label}
      className="ui-panel rounded-(--radius-card) border border-(--color-hairline) bg-(--color-canvas) p-6 shadow-(--shadow-minimal)"
      render={(props) => <section {...props} />}
    >
      {children}
    </Card>
  );
}
