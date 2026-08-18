// Package veridag provides a Go client for the Veridag REST API.
//
// It mirrors the Rust `veridag-sdk` `VeridagClient` trait so the same
// application logic reads identically across languages. Only the standard
// library is used (net/http, encoding/json).
package veridag

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

// Hex is a hex-encoded string (address / hash / object id / signature).
type Hex string

// Address, Hash, ObjectID, Signature are hex strings with semantic meaning.
type (
	Address   = Hex
	Hash      = Hex
	ObjectID  = Hex
	Signature = Hex
)

// Transaction is the wire type for a signed transaction.
type Transaction struct {
	ProtocolVersion int               `json:"protocol_version"`
	ChainID         int               `json:"chain_id"`
	Sender          Address           `json:"sender"`
	Nonce           uint64            `json:"nonce"`
	ExpiryEpoch     uint64            `json:"expiry_epoch"`
	Operation       json.RawMessage   `json:"operation"`
	Signature       Signature         `json:"signature"`
}

// Checkpoint is the wire type for a finalized checkpoint.
type Checkpoint struct {
	Sequence                uint64 `json:"sequence"`
	StateRoot               Hash   `json:"state_root"`
	TransactionRoot         Hash   `json:"transaction_root"`
	DagCommitment           Hash   `json:"dag_commitment"`
	ValidatorSetCommitment  Hash   `json:"validator_set_commitment"`
	ID                      Hash   `json:"id"`
	Votes                   int    `json:"votes"`
}

// ClientError is returned on transport / validation failures.
type ClientError struct {
	Kind    string
	Message string
}

func (e *ClientError) Error() string { return e.Kind + ": " + e.Message }

// Client is the transport-agnostic surface (mirrors the SDK trait).
type Client interface {
	Submit(tx *Transaction) (string, error)
	StateRoot() (Hash, error)
	LatestCheckpoint() (*Checkpoint, error)
	BalanceOf(owner Address) (uint64, error)
	GetObject(id ObjectID) (string, error)
}

// HTTPClient implements Client against a Veridag node over HTTP/JSON.
type HTTPClient struct{ BaseURL string }

func (c *HTTPClient) do(method, path string, body interface{}, out interface{}) error {
	var reader io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return &ClientError{"transport", err.Error()}
		}
		reader = bytes.NewReader(b)
	}
	req, err := http.NewRequest(method, c.BaseURL+path, reader)
	if err != nil {
		return &ClientError{"transport", err.Error()}
	}
	if body != nil {
		req.Header.Set("content-type", "application/json")
	}
	req.Header.Set("accept", "application/json")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return &ClientError{"transport", err.Error()}
	}
	defer resp.Body.Close()
	if resp.StatusCode == 404 {
		return &ClientError{"not_found", "resource not found"}
	}
	if resp.StatusCode >= 400 {
		return &ClientError{"transport", fmt.Sprintf("HTTP %d", resp.StatusCode)}
	}
	if out != nil {
		return json.NewDecoder(resp.Body).Decode(out)
	}
	return nil
}

// Submit posts a signed transaction and returns its id.
func (c *HTTPClient) Submit(tx *Transaction) (string, error) {
	var r struct {
		TxID string `json:"tx_id"`
	}
	if err := c.do(http.MethodPost, "/v1/submit", tx, &r); err != nil {
		return "", err
	}
	return r.TxID, nil
}

// StateRoot fetches the current committed state root.
func (c *HTTPClient) StateRoot() (Hash, error) {
	var r struct {
		Root Hash `json:"root"`
	}
	if err := c.do(http.MethodGet, "/v1/state-root", nil, &r); err != nil {
		return "", err
	}
	return r.Root, nil
}

// LatestCheckpoint fetches the most recent finalized checkpoint.
func (c *HTTPClient) LatestCheckpoint() (*Checkpoint, error) {
	var r Checkpoint
	if err := c.do(http.MethodGet, "/v1/checkpoint/latest", nil, &r); err != nil {
		return nil, err
	}
	return &r, nil
}

// BalanceOf returns the balance of an owner address.
func (c *HTTPClient) BalanceOf(owner Address) (uint64, error) {
	var r struct {
		Balance uint64 `json:"balance"`
	}
	if err := c.do(http.MethodGet, "/v1/balance/"+string(owner), nil, &r); err != nil {
		return 0, err
	}
	return r.Balance, nil
}

// GetObject returns the raw serialized object bytes (hex) by id.
func (c *HTTPClient) GetObject(id ObjectID) (string, error) {
	var r struct {
		Data string `json:"data"`
	}
	if err := c.do(http.MethodGet, "/v1/object/"+string(id), nil, &r); err != nil {
		return "", err
	}
	return r.Data, nil
}
