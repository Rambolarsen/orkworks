import { generateKeyPairSync, createHash, sign } from "node:crypto";
import { createKnowledgeActivation, verifyKnowledgeActivation } from "../electron/knowledgeProof.ts";
export function signer() {
  const keys = generateKeyPairSync("ed25519");
  const publicKey = keys.publicKey.export({ type: "spki", format: "pem" }).toString();
  const signed = (value: unknown) => {
    const payload = JSON.stringify(value);
    return Buffer.from(JSON.stringify({ payload, signature: sign(null, Buffer.from(payload), keys.privateKey).toString("base64") }));
  };
  function proof(sequence: number, version = String(sequence), content = "Reviewed synthetic guidance.") {
    const bundle = signed({ formatVersion: 1, privacyPolicyVersion: 1, capabilities: [], version, sequence,
      publishedAt: "2026-09-09T00:00:00.000Z", pages: [{ id: "guide.md", title: "Guide", type: "concept", status: "active", content,
        sha256: createHash("sha256").update(content).digest("hex"), relatedIds: [] }] });
    const entry = { formatVersion: 1, privacyPolicyVersion: 1, version, sequence, path: `bundles/${version}.json`, sha256: createHash("sha256").update(bundle).digest("hex") };
    const manifest = signed({ formatVersion: 1, bundles: [entry] });
    const activation = createKnowledgeActivation(bundle, manifest);
    return { verified: verifyKnowledgeActivation(Buffer.from(JSON.stringify(activation)), publicKey), bundle, manifest, entry };
  }
  return { publicKey, signed, proof };
}
