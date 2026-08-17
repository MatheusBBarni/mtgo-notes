import { Chip, type ChipProps } from "@heroui/react";

type StatusKind = "phase" | "certainty" | "source" | "error" | "incomplete";

const STATUS_METADATA: Record<
  StatusKind,
  { icon: string; prefix: string; role?: "alert" | "status" }
> = {
  phase: { icon: "◷", prefix: "Phase", role: "status" },
  certainty: { icon: "✓", prefix: "Certainty", role: "status" },
  source: { icon: "↗", prefix: "Source", role: "status" },
  error: { icon: "!", prefix: "Error", role: "alert" },
  incomplete: { icon: "…", prefix: "Encounter", role: "status" },
};

export function StatusLabel({
  kind,
  label,
}: {
  kind: StatusKind;
  label: string;
}) {
  const metadata = STATUS_METADATA[kind];
  let color: ChipProps["color"] = "default";
  if (kind === "error") {
    color = "danger";
  } else if (kind === "certainty") {
    color = "success";
  }

  return (
    <Chip
      className="ui-status w-fit rounded-(--radius-control) text-[13px] font-medium"
      color={color}
      data-kind={kind}
      role={metadata.role}
      size="sm"
      variant={kind === "incomplete" ? "tertiary" : "secondary"}
    >
      <span aria-hidden="true">{metadata.icon}</span>
      <Chip.Label>
        {metadata.prefix}: {label}
      </Chip.Label>
    </Chip>
  );
}
