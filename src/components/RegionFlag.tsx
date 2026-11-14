interface RegionFlagProps {
  region: string;
}

export function RegionFlag({ region }: RegionFlagProps) {
  const code = region.toLowerCase();

  switch (code) {
    case "global":
      return (
        <svg
          className="filter-globe-svg"
          viewBox="0 0 24 24"
          fill="none"
          aria-hidden="true"
          data-testid="region-icon-global"
        >
          <circle cx="12" cy="12" r="9.25" fill="#1e3a8a" stroke="#60a5fa" strokeWidth="1.5" />
          <ellipse cx="12" cy="12" rx="4.2" ry="9.25" stroke="#93c5fd" strokeWidth="1.3" />
          <line x1="2.75" y1="12" x2="21.25" y2="12" stroke="#93c5fd" strokeWidth="1.3" />
          <path d="M4.6 7.5H19.4" stroke="#93c5fd" strokeWidth="1.15" />
          <path d="M4.6 16.5H19.4" stroke="#93c5fd" strokeWidth="1.15" />
        </svg>
      );

    case "br":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-br"
        >
          <rect width="24" height="18" fill="#009c3b" />
          <polygon points="12,2.2 21.5,9 12,15.8 2.5,9" fill="#ffdf00" />
          <circle cx="12" cy="9" r="3.6" fill="#002776" />
          <path d="M8.6 8.5 Q12 7.2 15.4 9.6" stroke="#ffffff" strokeWidth="0.9" fill="none" />
        </svg>
      );

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

    case "na":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-na"
        >
          <rect width="24" height="18" fill="#b22234" />
          <rect y="2" width="24" height="1.5" fill="#ffffff" />
          <rect y="5" width="24" height="1.5" fill="#ffffff" />
          <rect y="8" width="24" height="1.5" fill="#ffffff" />
          <rect y="11" width="24" height="1.5" fill="#ffffff" />
          <rect y="14" width="24" height="1.5" fill="#ffffff" />
          <rect width="11" height="9.5" fill="#3c3b6e" />
          <circle cx="2.5" cy="2.3" r="0.7" fill="#ffffff" />
          <circle cx="5.5" cy="2.3" r="0.7" fill="#ffffff" />
          <circle cx="8.5" cy="2.3" r="0.7" fill="#ffffff" />
          <circle cx="4" cy="4.7" r="0.7" fill="#ffffff" />
          <circle cx="7" cy="4.7" r="0.7" fill="#ffffff" />
          <circle cx="2.5" cy="7.1" r="0.7" fill="#ffffff" />
          <circle cx="5.5" cy="7.1" r="0.7" fill="#ffffff" />
          <circle cx="8.5" cy="7.1" r="0.7" fill="#ffffff" />
        </svg>
      );

    case "euw":
    case "eune":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid={`region-icon-${code}`}
        >
          <rect width="24" height="18" fill="#003399" />
          <circle cx="12" cy="3.6" r="0.75" fill="#ffcc00" />
          <circle cx="14.7" cy="4.3" r="0.75" fill="#ffcc00" />
          <circle cx="16.7" cy="6.3" r="0.75" fill="#ffcc00" />
          <circle cx="17.4" cy="9" r="0.75" fill="#ffcc00" />
          <circle cx="16.7" cy="11.7" r="0.75" fill="#ffcc00" />
          <circle cx="14.7" cy="13.7" r="0.75" fill="#ffcc00" />
          <circle cx="12" cy="14.4" r="0.75" fill="#ffcc00" />
          <circle cx="9.3" cy="13.7" r="0.75" fill="#ffcc00" />
          <circle cx="7.3" cy="11.7" r="0.75" fill="#ffcc00" />
          <circle cx="6.6" cy="9" r="0.75" fill="#ffcc00" />
          <circle cx="7.3" cy="6.3" r="0.75" fill="#ffcc00" />
          <circle cx="9.3" cy="4.3" r="0.75" fill="#ffcc00" />
        </svg>
      );

    case "lan":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-lan"
        >
          <rect x="0" width="8" height="18" fill="#006847" />
          <rect x="8" width="8" height="18" fill="#ffffff" />
          <rect x="16" width="8" height="18" fill="#ce1126" />
          <circle cx="12" cy="9" r="2.2" fill="#8b5a2b" />
          <circle cx="12" cy="8.4" r="1.1" fill="#d4af37" />
        </svg>
      );

    case "las":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-las"
        >
          <rect y="0" width="24" height="6" fill="#74acdf" />
          <rect y="6" width="24" height="6" fill="#ffffff" />
          <rect y="12" width="24" height="6" fill="#74acdf" />
          <circle cx="12" cy="9" r="2.1" fill="#f6b40e" />
        </svg>
      );

    case "oce":
      return (
        <svg
          className="filter-flag-svg"
          viewBox="0 0 24 18"
          aria-hidden="true"
          data-testid="region-icon-oce"
        >
          <rect width="24" height="18" fill="#012169" />
          <rect width="11" height="9" fill="#012169" />
          <path d="M0 0L11 9M11 0L0 9" stroke="#ffffff" strokeWidth="1.8" />
          <path d="M0 0L11 9M11 0L0 9" stroke="#c8102e" strokeWidth="0.9" />
          <rect x="4.5" width="2" height="9" fill="#ffffff" />
          <rect y="3.5" width="11" height="2" fill="#ffffff" />
          <rect x="5" width="1" height="9" fill="#c8102e" />
          <rect y="4" width="11" height="1" fill="#c8102e" />
          <circle cx="5.5" cy="13.5" r="1.6" fill="#ffffff" />
          <circle cx="18" cy="4.2" r="0.9" fill="#ffffff" />
          <circle cx="21" cy="7.5" r="0.9" fill="#ffffff" />
          <circle cx="15" cy="8.2" r="0.9" fill="#ffffff" />
          <circle cx="18" cy="13.5" r="1.1" fill="#ffffff" />
          <circle cx="19.8" cy="10.2" r="0.6" fill="#ffffff" />
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
      return (
        <svg
          className="filter-globe-svg"
          viewBox="0 0 24 24"
          fill="none"
          aria-hidden="true"
          data-testid={`region-icon-${code}`}
        >
          <circle cx="12" cy="12" r="9" stroke="#9ca3af" strokeWidth="1.5" />
        </svg>
      );
  }
}
