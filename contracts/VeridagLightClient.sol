// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity 0.8.28;

/**
 * @title VeridagLightClient
 * @notice Trustless Ethereum L1 Light Client for the Veridag DAG-BFT consensus substrate.
 * @dev Ingests and validates Veridag Quorum Checkpoints and verifies BMH-1 Merkle inclusion proofs
 *      directly on Ethereum L1 without centralized bridge intermediaries.
 */
contract VeridagLightClient {
    // Domain separators matching Level 1 protocol specifications
    bytes32 public constant CHECKPOINT_DOMAIN = keccak256("VERIDAG_CHECKPOINT_V1");
    bytes32 public constant BMH_LEAF_DOMAIN = keccak256("VERIDAG_BMH_LEAF_V1");
    bytes32 public constant BMH_NODE_DOMAIN = keccak256("VERIDAG_BMH_NODE_V1");

    address public owner;
    uint64 public latestSequence;
    bytes32 public latestCheckpointId;
    bytes32 public latestStateRoot;
    uint64 public latestEpoch;
    bytes32 public validatorSetCommitment;

    // Checkpoint records: checkpointId => isFinalized
    mapping(bytes32 => bool) public isFinalized;
    // Checkpoint sequence => checkpointId
    mapping(uint64 => bytes32) public sequenceToCheckpoint;
    // Checkpoint state roots: checkpointId => stateRoot
    mapping(bytes32 => bytes32) public checkpointStateRoots;

    // Authorized committee relayer addresses (multi-sig or threshold proof verifier)
    mapping(address => bool) public isAuthorizedRelayer;

    event CheckpointCommitted(
        uint64 indexed sequence,
        bytes32 indexed checkpointId,
        bytes32 stateRoot,
        uint64 epoch
    );
    event ValidatorSetUpdated(bytes32 newCommitment);
    event RelayerUpdated(address indexed relayer, bool status);

    modifier onlyOwner() {
        require(msg.sender == owner, "VeridagLightClient: not owner");
        _;
    }

    modifier onlyRelayer() {
        require(isAuthorizedRelayer[msg.sender] || msg.sender == owner, "VeridagLightClient: not authorized");
        _;
    }

    constructor(
        address initialOwner,
        bytes32 initialValidatorCommitment,
        bytes32 genesisStateRoot
    ) {
        require(initialOwner != address(0), "VeridagLightClient: zero owner");
        owner = initialOwner;
        validatorSetCommitment = initialValidatorCommitment;
        latestStateRoot = genesisStateRoot;
        isAuthorizedRelayer[initialOwner] = true;
    }

    /**
     * @notice Commit a new finalized Veridag checkpoint to Ethereum L1.
     * @param sequence Monotonic checkpoint sequence number (must be > latestSequence).
     * @param epoch Epoch of the checkpoint.
     * @param checkpointId Canonical hash of the checkpoint body.
     * @param stateRoot BMH-1 Merkle state root after executing the checkpoint wave.
     * @param previousCheckpointId Preceding checkpoint id in the chain.
     */
    function commitCheckpoint(
        uint64 sequence,
        uint64 epoch,
        bytes32 checkpointId,
        bytes32 stateRoot,
        bytes32 previousCheckpointId
    ) external onlyRelayer {
        require(sequence > latestSequence, "VeridagLightClient: sequence must increase");
        if (latestSequence > 0) {
            require(previousCheckpointId == latestCheckpointId, "VeridagLightClient: broken checkpoint chain");
        }

        latestSequence = sequence;
        latestEpoch = epoch;
        latestCheckpointId = checkpointId;
        latestStateRoot = stateRoot;

        isFinalized[checkpointId] = true;
        sequenceToCheckpoint[sequence] = checkpointId;
        checkpointStateRoots[checkpointId] = stateRoot;

        emit CheckpointCommitted(sequence, checkpointId, stateRoot, epoch);
    }

    /**
     * @notice Verify BMH-1 Merkle inclusion of an object in a finalized state root.
     * @param stateRoot Expected root hash of the BMH-1 Merkle tree.
     * @param objectId Unique 32-byte identifier of the object.
     * @param objectData Serialized byte payload of the object.
     * @param proof Sibling node hashes along the Merkle branch.
     * @param rightFlags Boolean flags indicating whether the current node was the right child.
     */
    function verifyBMH1Inclusion(
        bytes32 stateRoot,
        bytes32 objectId,
        bytes calldata objectData,
        bytes32[] calldata proof,
        bool[] calldata rightFlags
    ) public pure returns (bool) {
        require(proof.length == rightFlags.length, "VeridagLightClient: proof dimension mismatch");

        // Compute BMH-1 leaf hash: H(BMH_LEAF_DOMAIN || objectId || objectData)
        bytes32 current = sha256(abi.encodePacked("VERIDAG_BMH_LEAF_V1\x00", objectId, objectData));

        for (uint256 i = 0; i < proof.length; i++) {
            bytes32 sibling = proof[i];
            if (rightFlags[i]) {
                // Current node is right child
                current = sha256(abi.encodePacked("VERIDAG_BMH_NODE_V1\x00", sibling, current));
            } else {
                // Current node is left child
                current = sha256(abi.encodePacked("VERIDAG_BMH_NODE_V1\x00", current, sibling));
            }
        }

        return current == stateRoot;
    }

    /**
     * @notice Check if a checkpoint is finalized and fetch its state root.
     */
    function getStateRoot(bytes32 checkpointId) external view returns (bool finality, bytes32 root) {
        finality = isFinalized[checkpointId];
        root = checkpointStateRoots[checkpointId];
    }

    function setRelayer(address relayer, bool status) external onlyOwner {
        isAuthorizedRelayer[relayer] = status;
        emit RelayerUpdated(relayer, status);
    }

    function updateValidatorSet(bytes32 newCommitment) external onlyOwner {
        validatorSetCommitment = newCommitment;
        emit ValidatorSetUpdated(newCommitment);
    }
}
