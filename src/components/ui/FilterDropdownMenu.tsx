import { ReactNode, MutableRefObject } from "react";

export interface FilterDropdownOption {
  value: string;
  label: string;
  title?: string;
  icon?: ReactNode;
}

interface FilterDropdownMenuProps {
  id: string;
  listboxId: string;
  labelId: string;
  options: FilterDropdownOption[];
  selectedValue: string | undefined;
  highlightedIndex: number;
  optionRefs: MutableRefObject<Array<HTMLLIElement | null>>;
  onHighlight: (index: number) => void;
  onSelect: (index: number) => void;
}

export function FilterDropdownMenu({
  id,
  listboxId,
  labelId,
  options,
  selectedValue,
  highlightedIndex,
  optionRefs,
  onHighlight,
  onSelect,
}: FilterDropdownMenuProps) {
  return (
    <ul
      id={listboxId}
      role="listbox"
      className="filter-dropdown-menu"
      aria-labelledby={labelId}
      tabIndex={-1}
    >
      {options.map((opt, idx) => {
        const isSelected = opt.value === selectedValue;
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
            onMouseEnter={() => onHighlight(idx)}
            onClick={() => onSelect(idx)}
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
  );
}
