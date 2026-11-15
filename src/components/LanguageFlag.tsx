import brFlagUrl from "../assets/flags/br.svg";
import brCompactFlagUrl from "../assets/flags/compact/br.svg";
import usFlagUrl from "../assets/flags/us.svg";
import { AppLocale } from "../i18n/types";

interface LanguageFlagProps {
  locale: AppLocale;
  className?: string;
  variant?: "default" | "compact";
}

export function LanguageFlag({
  locale,
  className = "",
  variant = "default",
}: LanguageFlagProps) {
  const src =
    locale === "pt_BR"
      ? variant === "compact"
        ? brCompactFlagUrl
        : brFlagUrl
      : usFlagUrl;
  return (
    <img
      src={src}
      alt=""
      aria-hidden="true"
      className={`language-flag-img ${className}`.trim()}
      draggable={false}
    />
  );
}
