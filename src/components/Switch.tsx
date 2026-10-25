interface Props {
  checked: boolean;
  label: string;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

export function Switch({ checked, label, disabled, onChange }: Props) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className="switch"
      disabled={disabled}
      onClick={() => {
        onChange(!checked);
      }}
    >
      <span className="switch__track" aria-hidden="true" />
      {label}
    </button>
  );
}
