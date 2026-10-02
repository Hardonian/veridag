import { encodeSignedTransaction } from "./codec.ts";
import { bytesToHex, hash } from "./crypto.ts";
import type { SignedTransaction } from "./types.ts";

export interface NodeHealth {
  status: string;
  version: string;
  protocolVersion: number;
  chainId: number;
  validatorId: string;
  /** @deprecated Development servers before v1 exposed their seed. */
  validatorSeed?: number;
  committeeN: number;
  committeeQuorum: number;
  maxRound: number;
  highestWave: number;
  stateRoot: string;
  checkpointsCount: number;
}

export interface StateRootResponse {
  stateRoot: string;
  highestWave: number;
}

export interface AccountBalanceResponse {
  address: string;
  objectId: string;
  balance: number;
  exists: boolean;
}

export interface LatestCheckpointResponse {
  id: string;
  sequence: number;
  epoch: number;
  stateRoot: string;
  transactionRoot: string;
  dagCommitment: string;
  validatorSetCommitment: string;
  votesCount: number;
}

export interface TxSubmitResponse {
  status: string;
  txId: string;
  sender: string;
}

interface HealthWire {
  status: string;
  version: string;
  protocol_version: number;
  chain_id: number;
  validator_id?: string;
  validator_seed?: number;
  committee_n: number;
  committee_quorum: number;
  max_round: number;
  highest_wave: number;
  state_root: string;
  checkpoints_count: number;
}

interface StateRootWire {
  state_root: string;
  highest_wave: number;
}

interface AccountWire {
  address: string;
  object_id: string;
  balance: number;
  exists: boolean;
}

interface CheckpointWire {
  id: string;
  sequence: number;
  epoch: number;
  state_root: string;
  transaction_root: string;
  dag_commitment: string;
  validator_set_commitment: string;
  votes_count: number;
}

interface SubmitWire {
  status?: string;
  tx_id?: string;
  sender?: string;
  error?: string;
}

/**
 * High-performance HTTP client for interacting with Veridag validator RPC nodes.
 */
export class VeridagClient {
  readonly rpcUrl: string;
  readonly token?: string;
  readonly timeoutMs: number;

  constructor(
    rpcUrl = "http://127.0.0.1:8080",
    options: { token?: string; timeoutMs?: number } = {},
  ) {
    this.rpcUrl = rpcUrl.replace(/\/$/, "");
    this.token = options.token;
    this.timeoutMs = options.timeoutMs ?? 10_000;
  }

  private headers(extra: Record<string, string> = {}): Record<string, string> {
    return {
      ...extra,
      ...(this.token ? { Authorization: `Bearer ${this.token}` } : {}),
    };
  }

  private async request(path: string, init: RequestInit = {}): Promise<Response> {
    return fetch(`${this.rpcUrl}${path}`, {
      ...init,
      headers: this.headers(init.headers as Record<string, string> | undefined),
      signal: init.signal ?? AbortSignal.timeout(this.timeoutMs),
    });
  }

  /**
   * Query the node health and consensus status.
   */
  async health(): Promise<NodeHealth> {
    const res = await this.request("/v1/health");
    if (!res.ok) {
      throw new Error(`Health check failed with status ${res.status}`);
    }
    const data = (await res.json()) as HealthWire;
    return {
      status: data.status,
      version: data.version,
      protocolVersion: data.protocol_version,
      chainId: data.chain_id,
      validatorId: data.validator_id ?? "",
      validatorSeed: data.validator_seed,
      committeeN: data.committee_n,
      committeeQuorum: data.committee_quorum,
      maxRound: data.max_round,
      highestWave: data.highest_wave,
      stateRoot: data.state_root,
      checkpointsCount: data.checkpoints_count,
    };
  }

  /**
   * Query the latest committed state root.
   */
  async getStateRoot(): Promise<StateRootResponse> {
    const res = await this.request("/v1/state/root");
    if (!res.ok) {
      throw new Error(`getStateRoot failed with status ${res.status}`);
    }
    const data = (await res.json()) as StateRootWire;
    return {
      stateRoot: data.state_root,
      highestWave: data.highest_wave,
    };
  }

  /**
   * Query an account balance and object state.
   */
  async getAccountBalance(address: Uint8Array | string): Promise<AccountBalanceResponse> {
    const hex = typeof address === "string" ? address.replace(/^0x/, "") : bytesToHex(address);
    const res = await this.request(`/v1/state/account/${hex}`);
    if (!res.ok) {
      throw new Error(`getAccountBalance failed with status ${res.status}`);
    }
    const data = (await res.json()) as AccountWire;
    return {
      address: data.address,
      objectId: data.object_id,
      balance: data.balance,
      exists: data.exists,
    };
  }

  /**
   * Query the latest finalized checkpoint with validator quorum proof.
   */
  async getLatestCheckpoint(): Promise<LatestCheckpointResponse> {
    const res = await this.request("/v1/checkpoints/latest");
    if (!res.ok) {
      throw new Error(`getLatestCheckpoint failed with status ${res.status}`);
    }
    const data = (await res.json()) as CheckpointWire;
    return {
      id: data.id,
      sequence: data.sequence,
      epoch: data.epoch,
      stateRoot: data.state_root,
      transactionRoot: data.transaction_root,
      dagCommitment: data.dag_commitment,
      validatorSetCommitment: data.validator_set_commitment,
      votesCount: data.votes_count,
    };
  }

  /**
   * Submit a signed transaction to the validator mempool.
   */
  async submitTransaction(stx: SignedTransaction, senderPublicKey?: Uint8Array): Promise<TxSubmitResponse> {
    const rawTxBytes = encodeSignedTransaction(stx);
    const body: Record<string, string> = {
      raw_tx_hex: bytesToHex(rawTxBytes),
    };
    if (senderPublicKey) {
      body.public_key = bytesToHex(senderPublicKey);
    }

    const res = await this.request("/v1/tx/submit", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Idempotency-Key": bytesToHex(hash("VERIDAG_TX_V1", rawTxBytes)),
      },
      body: JSON.stringify(body),
    });

    const data = (await res.json()) as SubmitWire;
    if (!res.ok) {
      throw new Error(`submitTransaction failed: ${data.error || res.statusText}`);
    }

    return {
      status: data.status,
      txId: data.tx_id,
      sender: data.sender,
    } as TxSubmitResponse;
  }
}
