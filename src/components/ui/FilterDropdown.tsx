import { useEffect, useRef, useState } from "react";
import { FilterDropdownMenu, FilterDropdownOption } from "./FilterDropdownMenu";
import { handleFilterDropdownKeyDown } from "./filterDropdownKeyboard";

export type { FilterDropdownOption };

interface FilterDropdownProps {
  id: string;
  labelId: string;
  testId?: string;
  value: string;
  options: FilterDropdownOption[];
  disabled?: boolean;
  onChange: (value: string) => void;
  className?: string;
}

export function FilterDropdown({
  id,
  labelId,
  testId,
  value,
  options,
  disabled = false,
  onChange,
  className = "",
}: FilterDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const selectedIndex = Math.max(
    0,
    options.findIndex((opt) => opt.value === value)
  );
  const [highlightedIndex, setHighlightedIndex] = useState(selectedIndex);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const optionRefs = useRef<Array<HTMLLIElement | null>>([]);

  const selectedOption = options[selectedIndex] ?? options[0];
  const listboxId = `${id}-listbox`;

  useEffect(() => {
    setHighlightedIndex(selectedIndex);
  }, [selectedIndex]);

  useEffect(() => {
    if (disabled && isOpen) {
      setIsOpen(false);
    }
  }, [disabled, isOpen]);

  useEffect(() => {
    if (!isOpen) return;

    const handleOutsideMouseDown = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleOutsideMouseDown);
    return () => {
      document.removeEventListener("mousedown", handleOutsideMouseDown);
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const optionEl = optionRefs.current[highlightedIndex];
    optionEl?.scrollIntoView?.({ block: "nearest" });
  }, [isOpen, highlightedIndex]);

  const openDropdown = (initialIndex = selectedIndex) => {
    if (disabled || options.length === 0) return;
    setHighlightedIndex(initialIndex);
    setIsOpen(true);
  };

  const closeDropdown = () => {
    setIsOpen(false);
  };

  const commitSelection = (index: number) => {
    const opt = options[index];
    if (!opt) return;
    closeDropdown();
    if (opt.value !== value) {
      onChange(opt.value);
    }
    triggerRef.current?.focus();
  };

  return (
    <div
      ref={containerRef}
      className={`filter-dropdown ${isOpen ? "open" : ""} ${className}`.trim()}
    >
      <button
        ref={triggerRef}
        id={id}
        data-testid={testId}
        type="button"
        role="combobox"
        className="filter-dropdown-trigger"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={listboxId}
        aria-labelledby={labelId}
        aria-activedescendant={isOpen ? `${id}-option-${highlightedIndex}` : undefined}
        data-value={selectedOption?.value ?? ""}
        title={selectedOption?.title}
        onClick={() => (isOpen ? closeDropdown() : openDropdown(selectedIndex))}
        onKeyDown={(event) =>
          handleFilterDropdownKeyDown({
            event,
            isOpen,
            disabled,
            options,
            value,
            selectedIndex,
            highlightedIndex,
            openDropdown,
            closeDropdown,
            commitSelection,
            setHighlightedIndex,
            onChange,
          })
        }
      >
        <span className="filter-dropdown-value">
          {selectedOption?.icon && (
            <span className="filter-dropdown-icon-slot" aria-hidden="true">
              {selectedOption.icon}
            </span>
          )}
          <span className="filter-dropdown-label">{selectedOption?.label}</span>
        </span>
        <svg
          className="filter-dropdown-chevron"
          viewBox="0 0 12 12"
          width="12"
          height="12"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M3 4.5L6 7.5L9 4.5"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </button>

      {isOpen && (
        <FilterDropdownMenu
          id={id}
          listboxId={listboxId}
          labelId={labelId}
          options={options}
          selectedValue={selectedOption?.value}
          highlightedIndex={highlightedIndex}
          optionRefs={optionRefs}
          onHighlight={setHighlightedIndex}
          onSelect={commitSelection}
        />
      )}
    </div>
  );
}
