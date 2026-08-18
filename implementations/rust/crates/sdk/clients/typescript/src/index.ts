// Veridag TypeScript client — typed wrapper over the Veridag REST API.
//
// Mirrors the Rust `veridag-sdk` `VeridagClient` trait so the same application
// logic reads identically across languages. All byte fields are hex strings.
//
// Design priorities: zero runtime dependencies (uses global `fetch`), strict
// types, and explicit error mapping. Safe for browser, Node 18+, and edge
// runtimes (Workers, Deno).

export type Hex = string; // 0x-prefixed or bare hex string

export interface Address extends String {}
export type Hash = Hex;
export type ObjectId = Hex;
export type BatchId = Hex;
export type Signature = Hex;

export type Operation =
  | {
      TransferValue: {
        from: { id: ObjectId; expected: number };
        to: Address;
        amount: number;
      };
    }
  | { CreateObject: Record<string, unknown> }
  | { CallWasm: Record<string, unknown> };

export interface Transaction {
  protocol_version: number;
  chain_id: number;
  sender: Address;
  nonce: number;
  expiry_epoch: number;
  operation: Operation;
  signature: Signature;
}

export interface Checkpoint {
  sequence: number;
  state_root: Hash;
  transaction_root: Hash;
  dag_commitment: Hash;
  validator_set_commitment: Hash;
  id: Hash;
  votes: number;
}

export class VeridagClientError extends Error {
  constructor(
    public kind: "rejected" | "not_found" | "transport",
    message: string,
  ) {
    super(message);
    this.name = "VeridagClientError";
  }
}

export interface VeridagClient {
  submit(tx: Transaction): Promise<string>;
  stateRoot(): Promise<Hash | null>;
  latestCheckpoint(): Promise<Checkpoint | null>;
  balanceOf(owner: Address): Promise<number>;
  getObject(id: ObjectId): Promise<string | null>;
}

/** HTTP/JSON implementation of `VeridagClient` against a Veridag node. */
export class HttpClient implements VeridagClient {
  constructor(private readonly baseUrl: string) {}

  private async getJson<T>(path: string): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      headers: { accept: "application/json" },
    });
    if (!res.ok) {
      const body = (await res.json().catch(() => ({}))) as { error?: string };
      throw new VeridagClientError(
        res.status === 404 ? "not_found" : "transport",
        body.error ?? `HTTP ${res.status}`,
      );
    }
    return (await res.json()) as T;
  }

  private async postJson<T>(path: string, body: unknown): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      throw new VeridagClientError(
        res.status === 400 ? "rejected" : "transport",
        data.error ?? `HTTP ${res.status}`,
      );
    }
    return (await res.json()) as T;
  }

  async submit(tx: Transaction): Promise<string> {
    const r = await this.postJson<{ tx_id: string }>("/v1/submit", tx);
    return r.tx_id;
  }

  async stateRoot(): Promise<Hash | null> {
    const r = await this.getJson<{ root: Hash | null }>("/v1/state-root");
    return r.root;
  }

  async latestCheckpoint(): Promise<Checkpoint | null> {
    return this.getJson<Checkpoint | null>("/v1/checkpoint/latest");
  }

  async balanceOf(owner: Address): Promise<number> {
    const r = await this.getJson<{ balance: number }>(
      `/v1/balance/${owner}`,
    );
    return r.balance;
  }

  async getObject(id: ObjectId): Promise<string | null> {
    const r = await this.getJson<{ data: string | null }>(
      `/v1/object/${id}`,
    );
    return r.data;
  }
}
