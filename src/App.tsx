// App.tsx is the root component of the entire frontend.
// Right now it just renders a placeholder so we can verify
// the app window opens and React is working correctly.
// We will replace this with the full tabbed interface in Phase 7.

export default function App() {
  return (
    <div style={{
      display:        "flex",
      alignItems:     "center",
      justifyContent: "center",
      height:         "100vh",
      background:     "#0f172a",
      color:          "#e2e8f0",
      fontFamily:     "system-ui, sans-serif",
      flexDirection:  "column",
      gap:            "12px",
    }}>
      <div style={{ fontSize: "32px" }}>⚡</div>
      <div style={{ fontSize: "22px", fontWeight: 600 }}>SISU</div>
      <div style={{ fontSize: "14px", color: "#64748b" }}>
        Phase 2 — skeleton running correctly
      </div>
    </div>
  );
}