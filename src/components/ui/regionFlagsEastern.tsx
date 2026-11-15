import type { ReactElement } from "react";

export function renderEasternRegionFlag(code: string): ReactElement | null {
  switch (code) {
    case "kr":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-kr"
        >
          <rect width="24" height="18" fill="#ffffff" />
          <g transform="translate(12 9)">
            <path d="M-3.8,0 A3.8,3.8 0 0,1 3.8,0 A1.9,1.9 0 0,1 0,0 A1.9,1.9 0 0,0 -3.8,0 Z" fill="#cd2e3a" />
            <path d="M3.8,0 A3.8,3.8 0 0,1 -3.8,0 A1.9,1.9 0 0,1 0,0 A1.9,1.9 0 0,0 3.8,0 Z" fill="#0047a0" />
          </g>
          <g stroke="#111827" strokeWidth="1.05" strokeLinecap="butt">
            <line x1="3.4" y1="5.2" x2="6.1" y2="2.6" />
            <line x1="4.4" y1="6.2" x2="7.1" y2="3.6" />
            <line x1="16.9" y1="2.6" x2="19.6" y2="5.2" />
            <line x1="15.9" y1="3.6" x2="18.6" y2="6.2" />
            <line x1="3.4" y1="12.8" x2="6.1" y2="15.4" />
            <line x1="4.4" y1="11.8" x2="7.1" y2="14.4" />
            <line x1="16.9" y1="15.4" x2="19.6" y2="12.8" />
            <line x1="15.9" y1="14.4" x2="18.6" y2="11.8" />
          </g>
        </svg>
      );

    case "tr":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-tr"
        >
          <rect width="24" height="18" fill="#e30a17" />
          <circle cx="10.2" cy="9" r="4.3" fill="#ffffff" />
          <circle cx="11.5" cy="9" r="3.4" fill="#e30a17" />
          <polygon
            points="16.2,6.8 16.8,8.4 18.5,8.4 17.1,9.4 17.6,11 16.2,10 14.8,11 15.3,9.4 13.9,8.4 15.6,8.4"
            fill="#ffffff"
          />
        </svg>
      );

    case "ru":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-ru"
        >
          <rect y="0" width="24" height="6" fill="#ffffff" />
          <rect y="6" width="24" height="6" fill="#0039a6" />
          <rect y="12" width="24" height="6" fill="#d52b1e" />
        </svg>
      );

    case "jp":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-jp"
        >
          <rect width="24" height="18" fill="#ffffff" />
          <circle cx="12" cy="9" r="4.8" fill="#bc002d" />
        </svg>
      );

    case "me":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-me"
        >
          <rect x="6.5" y="0" width="17.5" height="6" fill="#00732f" />
          <rect x="6.5" y="6" width="17.5" height="6" fill="#ffffff" />
          <rect x="6.5" y="12" width="17.5" height="6" fill="#111827" />
          <rect x="0" y="0" width="6.5" height="18" fill="#ef3340" />
        </svg>
      );

    case "sg":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-sg"
        >
          <rect y="0" width="24" height="9" fill="#ef3340" />
          <rect y="9" width="24" height="9" fill="#ffffff" />
          <circle cx="5.8" cy="4.5" r="2.8" fill="#ffffff" />
          <circle cx="6.8" cy="4.5" r="2.4" fill="#ef3340" />
          <circle cx="8.8" cy="2.6" r="0.65" fill="#ffffff" />
          <circle cx="10.4" cy="3.9" r="0.65" fill="#ffffff" />
          <circle cx="9.8" cy="5.9" r="0.65" fill="#ffffff" />
          <circle cx="7.8" cy="5.9" r="0.65" fill="#ffffff" />
          <circle cx="7.2" cy="3.9" r="0.65" fill="#ffffff" />
        </svg>
      );

    case "tw":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-tw"
        >
          <rect width="24" height="18" fill="#fe0000" />
          <rect width="12" height="9" fill="#000095" />
          <circle cx="6" cy="4.5" r="2.1" fill="#ffffff" />
          <circle cx="6" cy="4.5" r="1.4" fill="#000095" />
          <circle cx="6" cy="4.5" r="1.05" fill="#ffffff" />
        </svg>
      );

    case "vn":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-vn"
        >
          <rect width="24" height="18" fill="#da251d" />
          <polygon
            points="12,3.8 13.3,7.5 17.2,7.5 14,9.8 15.2,13.6 12,11.3 8.8,13.6 10,9.8 6.8,7.5 10.7,7.5"
            fill="#ffff00"
          />
        </svg>
      );

    default:
      return null;
  }
}
