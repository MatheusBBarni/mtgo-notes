import { Checkbox as HeroCheckbox } from "@heroui/react";
import type { ReactNode } from "react";

type CheckboxProps = {
  checked: boolean;
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  name?: string;
  onChange: (checked: boolean) => void;
};

export function Checkbox({
  checked,
  children,
  className = "",
  disabled = false,
  name,
  onChange,
}: CheckboxProps) {
  return (
    <HeroCheckbox
      className={className}
      isDisabled={disabled}
      isSelected={checked}
      name={name}
      onChange={onChange}
      variant="secondary"
    >
      <HeroCheckbox.Content className="min-h-8 items-start gap-3">
        <HeroCheckbox.Control className="mt-0.5 rounded-xs">
          <HeroCheckbox.Indicator />
        </HeroCheckbox.Control>
        {children}
      </HeroCheckbox.Content>
    </HeroCheckbox>
  );
}
