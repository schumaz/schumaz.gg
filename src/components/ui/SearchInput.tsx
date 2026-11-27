import { useRef } from "react";
import { useTranslation } from "../../i18n";

interface SearchInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  ariaLabel: string;
  clearAriaLabel?: string;
  className?: string;
  testId?: string;
}

export function SearchInput({
  value,
  onChange,
  placeholder,
  ariaLabel,
  clearAriaLabel,
  className = "",
  testId,
}: SearchInputProps) {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement | null>(null);
  const resolvedClearAriaLabel = clearAriaLabel ?? t("common.clearSearch");

  const handleClear = () => {
    onChange("");
    inputRef.current?.focus();
  };

  return (
    <div className={`search-input-wrap ${className}`.trim()} data-testid={testId}>
      <svg
        className="search-input-icon"
        viewBox="0 0 24 24"
        width="14"
        height="14"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        aria-hidden="true"
      >
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input
        ref={inputRef}
        type="text"
        className={`search-input-field ${value ? "has-clear" : ""}`.trim()}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        aria-label={ariaLabel}
      />
      {value.length > 0 && (
        <button
          type="button"
          className="search-input-clear"
          onClick={handleClear}
          aria-label={resolvedClearAriaLabel}
          title={resolvedClearAriaLabel}
        >
          <svg
            viewBox="0 0 24 24"
            width="13"
            height="13"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.2"
            strokeLinecap="round"
            aria-hidden="true"
          >
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      )}
    </div>
  );
}
