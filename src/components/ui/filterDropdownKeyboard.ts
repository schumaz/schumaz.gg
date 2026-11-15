import { KeyboardEvent } from "react";
import type { FilterDropdownOption } from "./FilterDropdownMenu";

interface DropdownKeyboardContext {
  event: KeyboardEvent<HTMLButtonElement>;
  isOpen: boolean;
  disabled: boolean;
  options: FilterDropdownOption[];
  value: string;
  selectedIndex: number;
  highlightedIndex: number;
  openDropdown: (initialIndex?: number) => void;
  closeDropdown: () => void;
  commitSelection: (index: number) => void;
  setHighlightedIndex: (updater: number | ((prev: number) => number)) => void;
  onChange: (value: string) => void;
}

export function handleFilterDropdownKeyDown({
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
}: DropdownKeyboardContext): void {
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
}
