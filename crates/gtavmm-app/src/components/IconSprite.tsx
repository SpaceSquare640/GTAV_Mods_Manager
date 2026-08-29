// SVG icon sprite, carried over verbatim from the design mockup (the same source of
// truth used throughout this session's HTML Artifact) — see `styles/mockup.css`'s
// `.icon` rule for how `<use>` references get sized/colored.
const SPRITE_MARKUP = `
  <symbol id="i-lock" viewBox="0 0 24 24"><rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></symbol>
  <symbol id="i-settings" viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></symbol>
  <symbol id="i-folder" viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/></symbol>
  <symbol id="i-bot" viewBox="0 0 24 24"><rect x="4" y="9" width="16" height="11" rx="2"/><path d="M12 9V5m-3-1h6"/><circle cx="9" cy="14" r="1"/><circle cx="15" cy="14" r="1"/><path d="M2 14h2m18 0h-2"/></symbol>
  <symbol id="i-shield" viewBox="0 0 24 24"><path d="M12 3 4 6v6c0 4.5 3.2 7.7 8 9 4.8-1.3 8-4.5 8-9V6Z"/></symbol>
  <symbol id="i-alert-triangle" viewBox="0 0 24 24"><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z"/><path d="M12 9v4m0 4h.01"/></symbol>
  <symbol id="i-check-circle" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="m8.5 12.5 2.3 2.3L16 10"/></symbol>
  <symbol id="i-trash" viewBox="0 0 24 24"><path d="M4 7h16M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2m-8 0v12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V7"/></symbol>
  <symbol id="i-bar-chart" viewBox="0 0 24 24"><path d="M4 20V10m7 10V4m7 16v-7"/></symbol>
  <symbol id="i-x-octagon" viewBox="0 0 24 24"><path d="M8 2h8l6 6v8l-6 6H8l-6-6V8Z"/><path d="m9.5 9.5 5 5m0-5-5 5"/></symbol>
  <symbol id="i-monitor" viewBox="0 0 24 24"><rect x="2" y="4" width="20" height="13" rx="2"/><path d="M8 21h8M12 17v4"/></symbol>
  <symbol id="i-moon" viewBox="0 0 24 24"><path d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8Z"/></symbol>
  <symbol id="i-sun" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></symbol>
  <symbol id="i-cloud" viewBox="0 0 24 24"><path d="M7 18a4.5 4.5 0 0 1-.4-9A5.5 5.5 0 0 1 17.3 8H18a3.5 3.5 0 0 1 0 7H7Z"/></symbol>
  <symbol id="i-palette" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><circle cx="8" cy="11" r="1"/><circle cx="12" cy="8" r="1"/><circle cx="16" cy="11" r="1"/><path d="M12 21a1.5 1.5 0 0 1 0-3 2 2 0 0 0 0-4h-1a4 4 0 0 1 0-8"/></symbol>
  <symbol id="i-globe" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18Z"/></symbol>
  <symbol id="i-x" viewBox="0 0 24 24"><path d="M18 6 6 18M6 6l12 12"/></symbol>
  <symbol id="i-search" viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></symbol>
  <symbol id="i-download" viewBox="0 0 24 24"><path d="M12 3v12m0 0 4-4m-4 4-4-4M4 19h16"/></symbol>
  <symbol id="i-edit" viewBox="0 0 24 24"><path d="M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/></symbol>
  <symbol id="i-copy" viewBox="0 0 24 24"><rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></symbol>
  <symbol id="i-check" viewBox="0 0 24 24"><path d="m5 12 5 5L20 7"/></symbol>
  <symbol id="i-info" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-9h.01"/></symbol>
  <symbol id="i-file-text" viewBox="0 0 24 24"><path d="M6 2h9l5 5v13a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Z"/><path d="M14 2v6h6M8 13h8M8 17h5"/></symbol>
  <symbol id="i-layers" viewBox="0 0 24 24"><path d="m12 2 9 5-9 5-9-5Z"/><path d="m3 12 9 5 9-5M3 17l9 5 9-5"/></symbol>
  <symbol id="i-translate" viewBox="0 0 24 24"><path d="M4 5h9M8 3v2m0 0c0 4-2.5 7-6 8m3-2c1.5 1.5 3.5 2.5 5.5 3M13 21l4-9 4 9m-6.5-3h5"/></symbol>
`;

export function IconSprite() {
  return (
    <svg
      style={{ display: "none" }}
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: SPRITE_MARKUP }}
    />
  );
}

export function Icon({ name, className }: { name: string; className?: string }) {
  return (
    <svg className={className ? `icon ${className}` : "icon"}>
      <use href={`#i-${name}`} />
    </svg>
  );
}
