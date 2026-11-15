interface LoadingDotsProps {
  variant?: "accent" | "warning" | "inline";
  size?: "sm" | "md";
  className?: string;
  testId?: string;
}

export function LoadingDots({
  variant = "accent",
  size = "md",
  className = "",
  testId,
}: LoadingDotsProps) {
  return (
    <span
      className={`loading-dots variant-${variant} size-${size} ${className}`.trim()}
      aria-hidden="true"
      data-testid={testId}
    >
      <span className="loading-dot" />
      <span className="loading-dot" />
      <span className="loading-dot" />
    </span>
  );
}

interface LoadingBarProps {
  active?: boolean;
  className?: string;
  testId?: string;
}

export function LoadingBar({
  active = true,
  className = "",
  testId,
}: LoadingBarProps) {
  if (!active) return null;

  return (
    <div
      className={`loading-bar-track ${className}`.trim()}
      role="progressbar"
      aria-label="Carregando"
      aria-busy="true"
      data-testid={testId}
    >
      <div className="loading-bar-fill" />
    </div>
  );
}

interface ListSkeletonProps {
  rows?: number;
  variant?: "ranked" | "aram" | "augments" | "compact";
  className?: string;
  testId?: string;
}

const SKELETON_NAME_WIDTHS = ["34%", "26%", "40%", "30%", "36%", "28%"];

export function ListSkeleton({
  rows = 6,
  variant = "ranked",
  className = "",
  testId,
}: ListSkeletonProps) {
  return (
    <div
      className={`skeleton-list variant-${variant} ${className}`.trim()}
      aria-hidden="true"
      data-testid={testId}
    >
      {Array.from({ length: rows }).map((_, idx) => {
        const nameWidth = SKELETON_NAME_WIDTHS[idx % SKELETON_NAME_WIDTHS.length];

        if (variant === "augments") {
          return (
            <div key={idx} className="skeleton-row skeleton-augment-row">
              <div className="skeleton-block skeleton-augment-icon" />
              <div className="skeleton-block skeleton-tier-square" />
              <div className="skeleton-augment-lines">
                <div className="skeleton-augment-top">
                  <div
                    className="skeleton-block skeleton-line"
                    style={{ width: nameWidth }}
                  />
                  <div className="skeleton-block skeleton-pill" />
                </div>
                <div className="skeleton-block skeleton-line is-sub" />
              </div>
            </div>
          );
        }

        if (variant === "compact") {
          return (
            <div key={idx} className="skeleton-row skeleton-compact-row">
              <div className="skeleton-block skeleton-rank" />
              <div className="skeleton-block skeleton-avatar" />
              <div
                className="skeleton-block skeleton-line"
                style={{ width: nameWidth }}
              />
              <div className="skeleton-block skeleton-pill is-end" />
            </div>
          );
        }

        return (
          <div key={idx} className="skeleton-row">
            <div className="skeleton-block skeleton-rank" />
            <div className="skeleton-block skeleton-avatar" />
            <div
              className="skeleton-block skeleton-line"
              style={{ width: nameWidth }}
            />
            <div className="skeleton-stats-group">
              {variant === "ranked" && (
                <div className="skeleton-block skeleton-lane-pill" />
              )}
              <div className="skeleton-block skeleton-stat-pill" />
              <div className="skeleton-block skeleton-stat-pill" />
              {variant === "ranked" && (
                <div className="skeleton-block skeleton-stat-pill" />
              )}
              <div className="skeleton-block skeleton-pill" />
            </div>
          </div>
        );
      })}
    </div>
  );
}
