import { Label, ListBox, Select } from "@heroui/react";

export type SelectOption<Value extends string = string> = {
  label: string;
  value: Value;
};

type SelectFieldProps<Value extends string> = {
  disabled?: boolean;
  label: string;
  name?: string;
  onChange: (value: Value) => void;
  options: readonly SelectOption<Value>[];
  required?: boolean;
  value: Value;
};

export function SelectField<Value extends string>({
  disabled = false,
  label,
  name,
  onChange,
  options,
  required = false,
  value,
}: SelectFieldProps<Value>) {
  return (
    <Select
      fullWidth
      isDisabled={disabled}
      isRequired={required}
      name={name}
      value={value}
      variant="secondary"
      onChange={(key) => {
        if (key !== null && !Array.isArray(key)) {
          onChange(String(key) as Value);
        }
      }}
    >
      <Label>{label}</Label>
      <Select.Trigger className="min-h-11 rounded-(--radius-control) border border-(--color-hairline) bg-(--color-canvas) px-3">
        <Select.Value />
        <Select.Indicator />
      </Select.Trigger>
      <Select.Popover className="rounded-(--radius-card) border border-(--color-hairline) bg-(--color-canvas) p-1 shadow-(--shadow-minimal)">
        <ListBox>
          {options.map((option) => (
            <ListBox.Item
              id={option.value}
              key={option.value}
              textValue={option.label}
            >
              {option.label}
              <ListBox.ItemIndicator />
            </ListBox.Item>
          ))}
        </ListBox>
      </Select.Popover>
    </Select>
  );
}
