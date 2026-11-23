interface WorkspaceAlert {
  key: string;
  text: string;
  action?: { label: string; onClick: () => void };
}

export default function WorkspaceAlerts({ alerts }: { alerts: readonly WorkspaceAlert[] }) {
  return alerts.map(({ key, text, action }) => (
    <div className="shell-preferences-notice workspace-alert" key={key}>
      <div role="status">{text}</div>
      {action && <button type="button" onClick={action.onClick}>{action.label}</button>}
    </div>
  ));
}
