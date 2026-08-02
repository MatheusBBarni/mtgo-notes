import { Label, Radio, RadioGroup } from "@heroui/react";

import type { SelectOption } from "./SelectField";

type RadioGroupFieldProps<Value extends string> = {
  className?: string;
  label: string;
  name: string;
  onChange: (value: Value) => void;
  options: readonly SelectOption<Value>[];
  value: Value;
};

export function RadioGroupField<Value extends string>({
  className = "",
  label,
  name,
  onChange,
  options,
  value,
}: RadioGroupFieldProps<Value>) {
  return (
    <RadioGroup
      className={className}
      name={name}
      value={value}
      variant="secondary"
      onChange={(nextValue) => onChange(nextValue as Value)}
    >
      <Label>{label}</Label>
      {options.map((option) => (
        <Radio key={option.value} value={option.value}>
          <Radio.Content className="min-h-8 items-center gap-3">
            <Radio.Control>
              <Radio.Indicator />
            </Radio.Control>
            {option.label}
          </Radio.Content>
        </Radio>
      ))}
    </RadioGroup>
  );
}
