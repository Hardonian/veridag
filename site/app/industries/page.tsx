const packs = [
  {
    name: "Supply chain",
    standard: "GS1 EPCIS 2.0",
    use: "Anchor event evidence and custody transitions without copying commercial records into shared state.",
  },
  {
    name: "Healthcare",
    standard: "HL7 FHIR R4/R5",
    use: "Commit provenance for bounded FHIR resources while patient data remains in the system of record.",
  },
  {
    name: "Manufacturing",
    standard: "OPC UA",
    use: "Anchor signed production and quality events with tenant and schema separation.",
  },
  {
    name: "Digital identity",
    standard: "W3C Verifiable Credentials 2.0",
    use: "Record credential evidence hashes and lifecycle events without storing the credential payload.",
  },
  {
    name: "Event-driven systems",
    standard: "CloudEvents 1.0",
    use: "Normalize bounded event envelopes for deterministic multi-party workflows.",
  },
  {
    name: "Financial services",
    standard: "ISO 20022",
    use: "Validate payment messages and route approved instructions into deterministic settlement adapters.",
  },
];

export default function IndustriesPage() {
  return (
    <div>
      <h1>Cross-Industry Integration Packs</h1>
      <p className="tagline">
        One deterministic evidence model, versioned profiles for established standards, and no raw business or personal data in shared state by default.
      </p>

      <div className="alert" style={{ margin: "24px 0" }}>
        <div className="alert-title">Privacy-preserving by construction</div>
        <p style={{ margin: 0 }}>
          Each adapter validates a bounded source envelope and creates an evidence anchor containing hashes, tenant scope, profile version, and event time. Source documents stay in the operator&apos;s system of record.
        </p>
      </div>

      <div className="card-grid">
        {packs.map((pack) => (
          <div className="card" key={pack.name}>
            <div className="badge">{pack.standard}</div>
            <h3>{pack.name}</h3>
            <p>{pack.use}</p>
          </div>
        ))}
      </div>

      <h2>Adoption path</h2>
      <div className="card-grid">
        <div className="card">
          <h3>1. Map</h3>
          <p>Select a versioned profile and map only the fields required for deterministic validation and routing.</p>
        </div>
        <div className="card">
          <h3>2. Anchor</h3>
          <p>Hash the source envelope, policy decision, and optional subject identifier into an immutable evidence object.</p>
        </div>
        <div className="card">
          <h3>3. Operate</h3>
          <p>Define retention, key custody, recovery, performance, and regulatory controls for the deployment before production promotion.</p>
        </div>
      </div>

      <p className="muted">
        These packs are technical adapters, not a representation of certification by GS1, HL7, OPC Foundation, W3C, CNCF, ISO, or any regulator.
      </p>
    </div>
  );
}
