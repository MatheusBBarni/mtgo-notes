import {
  FieldError,
  Label,
  TextArea,
  TextField as HeroTextField,
} from "@heroui/react";
import { useId, type ComponentPropsWithRef } from "react";

type TextAreaFieldProps = Omit<ComponentPropsWithRef<"textarea">, "id"> & {
  error?: string;
  inputId?: string;
  label: string;
};

export function TextAreaField({
  className = "",
  disabled,
  error,
  inputId,
  label,
  required,
  ...props
}: TextAreaFieldProps) {
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
      <TextArea
        {...props}
        className={[
          "ui-field__input min-h-24 w-full resize-y",
          "rounded-(--radius-control) border border-(--color-hairline)",
          "bg-(--color-canvas) px-3 py-2.5 text-(--color-ink)",
          className,
        ]
          .filter(Boolean)
          .join(" ")}
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
