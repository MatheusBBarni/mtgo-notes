import {
  Button as HeroButton,
  Spinner,
  type ButtonProps as HeroButtonProps,
} from "@heroui/react";
import type { ReactNode } from "react";

type ButtonVariant = "primary" | "secondary" | "destructive";

type ButtonProps = Omit<
  HeroButtonProps,
  "children" | "isDisabled" | "isPending" | "variant"
> & {
  children: ReactNode;
  variant?: ButtonVariant;
  busy?: boolean;
  disabled?: boolean;
};

export function Button({
  busy = false,
  children,
  className = "",
  disabled,
  type = "button",
  variant = "primary",
  ...props
}: ButtonProps) {
  const heroVariant: HeroButtonProps["variant"] =
    variant === "destructive" ? "danger" : variant;
  const classes = [
    "ui-button",
    `ui-button--${variant}`,
    "min-h-10 min-w-8 rounded-(--radius-control) px-4 text-base font-medium",
    "motion-reduce:transition-none",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <HeroButton
      {...props}
      aria-busy={busy || undefined}
      className={classes}
      isDisabled={disabled || busy}
      isPending={busy}
      render={(renderProps) => (
        <button {...renderProps} aria-busy={busy || undefined} />
      )}
      type={type}
      variant={heroVariant}
    >
      {busy ? <Spinner color="current" size="sm" /> : null}
      {children}
    </HeroButton>
  );
}
