import {
  FieldError,
  Input,
  Label,
  TextField as HeroTextField,
} from "@heroui/react";
import { useId, type ComponentPropsWithRef } from "react";

type TextFieldProps = Omit<ComponentPropsWithRef<"input">, "id"> & {
  label: string;
  error?: string;
  inputId?: string;
};

export function TextField({
  disabled,
  error,
  inputId,
  label,
  required,
  ...props
}: TextFieldProps) {
  const generatedId = useId();
  const id = inputId ?? generatedId;

  return (
    <HeroTextField
      className="ui-field gap-2"
      isDisabled={disabled}
      isInvalid={Boolean(error)}
      isRequired={required}
    >
      <Label className="ui-field__label" htmlFor={id}>
        {label}
      </Label>
      <Input
        {...props}
        className="ui-field__input min-h-11 w-full rounded-(--radius-control) border border-(--color-hairline) bg-(--color-canvas) px-3 py-2.5 text-(--color-ink)"
        disabled={disabled}
        id={id}
        required={required}
        variant="secondary"
      />
      {error ? (
        <div role="alert">
          <FieldError className="ui-field__message">Error: {error}</FieldError>
        </div>
      ) : null}
    </HeroTextField>
  );
}
