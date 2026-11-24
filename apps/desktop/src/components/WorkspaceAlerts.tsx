interface WorkspaceAlert {
  key: string;
  text: string;
  action?: { label: string; onClick: () => void };
}

export default function WorkspaceAlerts({ alerts }: { alerts: readonly WorkspaceAlert[] }) {
  if (alerts.length === 0) return null;

  return (
    <div className="workspace-alerts">
      {alerts.map(({ key, text, action }) => (
        <div className="workspace-alert" key={key}>
          <div role="status">{text}</div>
          {action && <button type="button" onClick={action.onClick}>{action.label}</button>}
        </div>
      ))}
    </div>
  );
}
