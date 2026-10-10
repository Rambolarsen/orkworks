interface Props {
  migrationNotice: boolean;
  needsRebuild: boolean;
  dismissMigrationNotice: () => void;
  onCommand: (command: "reset-layout") => void;
}

export default function ShellPreferencesNotice(props: Props) {
  return <>
    {props.migrationNotice && <div className="shell-preferences-notice" role="status">
      <span>Your saved panel arrangement is retained; this version uses a fixed layout.</span>
      <button type="button" onClick={props.dismissMigrationNotice}>Dismiss</button>
    </div>}
    {props.needsRebuild && <div className="shell-preferences-notice" role="alert">
      <span>Saved layout preferences could not be read. Defaults are in use. Reset Layout offers a confirmed rebuild.</span>
      <button type="button" onClick={() => props.onCommand("reset-layout")}>Reset Layout…</button>
    </div>}
  </>;
}
