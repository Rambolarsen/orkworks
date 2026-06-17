function RecommendationsPanel() {
  return (
    <div style={{ padding: "12px", height: "100%", display: "flex", flexDirection: "column" }}>
      <div style={{
        fontSize: "11px", fontWeight: 600, textTransform: "uppercase",
        letterSpacing: "0.5px", color: "#999", marginBottom: "12px"
      }}>
        Start Next Task
      </div>
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <p style={{ color: "#666", fontSize: 12, fontStyle: "italic" }}>
          Recommendations coming in M9
        </p>
      </div>
    </div>
  );
}

export default RecommendationsPanel;
