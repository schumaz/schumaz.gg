import { KeyboardEvent, ReactNode, useEffect, useRef, useState } from "react";

export interface FilterDropdownOption {
  value: string;
  label: string;
  title?: string;
  icon?: ReactNode;
}

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

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled || options.length === 0) return;

    switch (event.key) {
      case "ArrowDown": {
        event.preventDefault();
        if (!isOpen) {
          openDropdown(selectedIndex);
        } else {
          setHighlightedIndex((prev) => Math.min(prev + 1, options.length - 1));
        }
        break;
      }
      case "ArrowUp": {
        event.preventDefault();
        if (!isOpen) {
          openDropdown(selectedIndex);
        } else {
          setHighlightedIndex((prev) => Math.max(prev - 1, 0));
        }
        break;
      }
      case "Home": {
        if (isOpen) {
          event.preventDefault();
          setHighlightedIndex(0);
        }
        break;
      }
      case "End": {
        if (isOpen) {
          event.preventDefault();
          setHighlightedIndex(options.length - 1);
        }
        break;
      }
      case "Enter":
      case " ": {
        event.preventDefault();
        if (!isOpen) {
          openDropdown(selectedIndex);
        } else {
          commitSelection(highlightedIndex);
        }
        break;
      }
      case "Escape": {
        if (isOpen) {
          event.preventDefault();
          closeDropdown();
        }
        break;
      }
      case "Tab": {
        if (isOpen) {
          closeDropdown();
        }
        break;
      }
      default: {
        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
          const query = event.key.toLowerCase();
          const startIdx = isOpen ? (highlightedIndex + 1) % options.length : 0;
          const orderedIndices = [
            ...Array.from({ length: options.length - startIdx }, (_, i) => startIdx + i),
            ...Array.from({ length: startIdx }, (_, i) => i),
          ];
          const matchIdx = orderedIndices.find((idx) =>
            options[idx].label.toLowerCase().startsWith(query)
          );
          if (matchIdx !== undefined) {
            if (isOpen) {
              setHighlightedIndex(matchIdx);
            } else if (options[matchIdx].value !== value) {
              onChange(options[matchIdx].value);
            }
          }
        }
        break;
      }
    }
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
        onKeyDown={handleKeyDown}
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
        <ul
          id={listboxId}
          role="listbox"
          className="filter-dropdown-menu"
          aria-labelledby={labelId}
          tabIndex={-1}
        >
          {options.map((opt, idx) => {
            const isSelected = opt.value === selectedOption?.value;
            const isHighlighted = idx === highlightedIndex;
            return (
              <li
                key={opt.value}
                ref={(el) => {
                  optionRefs.current[idx] = el;
                }}
                id={`${id}-option-${idx}`}
                role="option"
                aria-selected={isSelected}
                data-value={opt.value}
                data-highlighted={isHighlighted ? "true" : undefined}
                title={opt.title}
                className={`filter-dropdown-option ${isSelected ? "selected" : ""} ${
                  isHighlighted ? "highlighted" : ""
                }`.trim()}
                onMouseDown={(e) => e.preventDefault()}
                onMouseEnter={() => setHighlightedIndex(idx)}
                onClick={() => commitSelection(idx)}
              >
                {opt.icon && (
                  <span className="filter-dropdown-icon-slot" aria-hidden="true">
                    {opt.icon}
                  </span>
                )}
                <span className="filter-dropdown-option-label">{opt.label}</span>
                {isSelected && (
                  <svg
                    className="filter-dropdown-check"
                    viewBox="0 0 12 12"
                    width="12"
                    height="12"
                    fill="none"
                    aria-hidden="true"
                  >
                    <path
                      d="M2.5 6.2L4.8 8.5L9.5 3.5"
                      stroke="currentColor"
                      strokeWidth="1.6"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
