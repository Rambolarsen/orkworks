interface CenterPanelProps {
  backendStatus: string;
}

function CenterPanel({ backendStatus }: CenterPanelProps) {
  return (
    <>
      <p style={{ color: "#666", fontSize: 14, marginBottom: 8 }}>OrkWorks</p>
      <p style={{ color: "#555", fontSize: 12 }}>
        Mission Control for AI Agents
      </p>
      <p style={{ color: "#444", fontSize: 11, marginTop: 12 }}>
        backend: {backendStatus}
      </p>
    </>
  );
}

export default CenterPanel;
