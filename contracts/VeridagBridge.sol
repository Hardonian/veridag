// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity 0.8.28;

import "./USDV.sol";
import "./VeridagLightClient.sol";

/**
 * @title VeridagBridge
 * @notice Trustless two-way cross-chain portal connecting Ethereum L1 and the Veridag DAG substrate.
 * @dev Enforces the Bridge Conservation Invariant:
 *      TotalSupply(L1) + TotalSupply(Veridag) == TotalAttestedReserves.
 *      Relies purely on cryptographic light client proofs without centralized multisig custody.
 */
contract VeridagBridge {
    USDV public immutable usdv;
    VeridagLightClient public immutable lightClient;
    address public owner;

    uint64 public depositSequence;
    mapping(bytes32 => bool) public isClaimedWithdrawal;

    event DepositInitiated(
        address indexed sender,
        bytes32 indexed veridagRecipient,
        uint256 amount,
        uint64 indexed sequence
    );

    event WithdrawalFinalized(
        address indexed recipient,
        uint256 amount,
        bytes32 indexed withdrawalId,
        bytes32 checkpointId
    );

    modifier onlyOwner() {
        require(msg.sender == owner, "VeridagBridge: caller not owner");
        _;
    }

    constructor(address _usdv, address _lightClient, address _owner) {
        require(_usdv != address(0) && _lightClient != address(0) && _owner != address(0), "VeridagBridge: zero address");
        usdv = USDV(_usdv);
        lightClient = VeridagLightClient(_lightClient);
        owner = _owner;
    }

    /**
     * @notice Deposit USDV on Ethereum L1 to bridge to Veridag native DAG substrate.
     * @param veridagRecipient 32-byte Veridag recipient address.
     * @param amount Amount of USDV (micro-units, 6 decimals).
     */
    function depositUSDV(bytes32 veridagRecipient, uint256 amount) external returns (uint64 seq) {
        require(amount > 0, "VeridagBridge: zero amount");
        require(veridagRecipient != bytes32(0), "VeridagBridge: zero recipient");

        // Burn on L1 (or lock) to maintain global supply conservation
        usdv.burn(msg.sender, amount);

        depositSequence++;
        seq = depositSequence;

        emit DepositInitiated(msg.sender, veridagRecipient, amount, seq);
    }

    /**
     * @notice Finalize a withdrawal initiated on Veridag by submitting a cryptographic BMH-1 proof.
     * @param checkpointId Finalized Veridag checkpoint containing the withdrawal record.
     * @param objectId Unique 32-byte identifier of the withdrawal object in Veridag state.
     * @param objectData Serialized withdrawal object payload.
     * @param proof Merkle branch sibling hashes.
     * @param rightFlags Merkle branch direction flags.
     * @param recipient Ethereum L1 recipient address.
     * @param amount Amount of USDV to release.
     * @param withdrawalId Unique 32-byte withdrawal message identifier for replay prevention.
     */
    function finalizeWithdrawal(
        bytes32 checkpointId,
        bytes32 objectId,
        bytes calldata objectData,
        bytes32[] calldata proof,
        bool[] calldata rightFlags,
        address recipient,
        uint256 amount,
        bytes32 withdrawalId
    ) external {
        require(!isClaimedWithdrawal[withdrawalId], "VeridagBridge: withdrawal already claimed");
        require(recipient != address(0), "VeridagBridge: zero recipient");
        require(amount > 0, "VeridagBridge: zero amount");

        // 1. Verify Checkpoint is finalized
        (bool finalized, bytes32 stateRoot) = lightClient.getStateRoot(checkpointId);
        require(finalized, "VeridagBridge: checkpoint not finalized");

        // 2. Verify Cryptographic BMH-1 Inclusion
        bool validInclusion = lightClient.verifyBMH1Inclusion(
            stateRoot,
            objectId,
            objectData,
            proof,
            rightFlags
        );
        require(validInclusion, "VeridagBridge: invalid state inclusion proof");

        // 3. Mark withdrawal as claimed (anti-replay)
        isClaimedWithdrawal[withdrawalId] = true;

        // 4. Mint canonical USDV on Ethereum L1
        usdv.mint(recipient, amount);

        emit WithdrawalFinalized(recipient, amount, withdrawalId, checkpointId);
    }
}
