import brFlagUrl from "../assets/flags/br.svg";
import usFlagUrl from "../assets/flags/us.svg";
import { AppLocale } from "../i18n/types";

interface LanguageFlagProps {
  locale: AppLocale;
  className?: string;
}

export function LanguageFlag({ locale, className = "" }: LanguageFlagProps) {
  const src = locale === "pt_BR" ? brFlagUrl : usFlagUrl;
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
