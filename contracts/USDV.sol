// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity 0.8.28;

/**
 * @title USDV (Veridag Dollar)
 * @notice Canonical institutional US Sovereign Stablecoin contract on Ethereum L1.
 * @dev Fully compliant ERC-20, ERC-2612 (Permit), EIP-3009 (Transfer with Authorization)
 *      with institutional capability-based governance (Mint, Burn, Freeze, Pause).
 *      Fixed 6-decimal precision matching US Dollar micro-cents and native Veridag state.
 */
contract USDV {
    // --- ERC-20 Metadata ---
    string public constant name = "Veridag Dollar";
    string public constant symbol = "USDV";
    uint8 public constant decimals = 6;

    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    // --- Institutional Capability Roles ---
    address public admin;
    mapping(address => bool) public isMinter;
    mapping(address => bool) public isBurner;
    mapping(address => bool) public isCompliance;
    mapping(address => bool) public isPauser;

    // --- Compliance & Sanctions State ---
    mapping(address => bool) public isFrozen;
    bool public paused;

    // --- EIP-712 & ERC-2612 Permit ---
    bytes32 public immutable DOMAIN_SEPARATOR;
    // keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)")
    bytes32 public constant PERMIT_TYPEHASH = 0x6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c9;
    // keccak256("TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)")
    bytes32 public constant TRANSFER_WITH_AUTHORIZATION_TYPEHASH = 0x7c7db6fe8d1eadd1d53615cca6107ba0ea0bc1b4869eaa93c0e0e85e347f8ae7;

    mapping(address => uint256) public nonces;
    mapping(address => mapping(bytes32 => bool)) public authorizationState;

    // --- Events ---
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event AccountFrozen(address indexed target);
    event AccountUnfrozen(address indexed target);
    event FundsSeized(address indexed target, address indexed escrow, uint256 amount);
    event Paused(address indexed caller);
    event Unpaused(address indexed caller);
    event RoleGranted(string role, address indexed account);
    event RoleRevoked(string role, address indexed account);

    modifier onlyAdmin() {
        require(msg.sender == admin, "USDV: caller not admin");
        _;
    }

    modifier onlyMinter() {
        require(isMinter[msg.sender] || msg.sender == admin, "USDV: caller not minter");
        _;
    }

    modifier onlyBurner() {
        require(isBurner[msg.sender] || msg.sender == admin, "USDV: caller not burner");
        _;
    }

    modifier onlyCompliance() {
        require(isCompliance[msg.sender] || msg.sender == admin, "USDV: caller not compliance");
        _;
    }

    modifier onlyPauser() {
        require(isPauser[msg.sender] || msg.sender == admin, "USDV: caller not pauser");
        _;
    }

    modifier notPaused() {
        require(!paused, "USDV: token transfers paused");
        _;
    }

    modifier notFrozen(address account) {
        require(!isFrozen[account], "USDV: account frozen under compliance order");
        _;
    }

    constructor(address initialAdmin) {
        require(initialAdmin != address(0), "USDV: zero admin address");
        admin = initialAdmin;
        isMinter[initialAdmin] = true;
        isBurner[initialAdmin] = true;
        isCompliance[initialAdmin] = true;
        isPauser[initialAdmin] = true;

        DOMAIN_SEPARATOR = keccak256(
            abi.encode(
                keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                keccak256(bytes(name)),
                keccak256(bytes("1")),
                block.chainid,
                address(this)
            )
        );
    }

    // --- ERC-20 Implementation ---

    function transfer(address to, uint256 value) external notPaused notFrozen(msg.sender) notFrozen(to) returns (bool) {
        _transfer(msg.sender, to, value);
        return true;
    }

    function approve(address spender, uint256 value) external notPaused notFrozen(msg.sender) notFrozen(spender) returns (bool) {
        allowance[msg.sender][spender] = value;
        emit Approval(msg.sender, spender, value);
        return true;
    }

    function transferFrom(address from, address to, uint256 value) external notPaused notFrozen(from) notFrozen(to) notFrozen(msg.sender) returns (bool) {
        uint256 allowed = allowance[from][msg.sender];
        if (allowed != type(uint256).max) {
            require(allowed >= value, "USDV: allowance exceeded");
            allowance[from][msg.sender] = allowed - value;
        }
        _transfer(from, to, value);
        return true;
    }

    function _transfer(address from, address to, uint256 value) internal {
        require(to != address(0), "USDV: transfer to zero address");
        require(balanceOf[from] >= value, "USDV: insufficient balance");

        unchecked {
            balanceOf[from] -= value;
            balanceOf[to] += value;
        }

        emit Transfer(from, to, value);
    }

    // --- Institutional Mint & Burn ---

    function mint(address to, uint256 value) external onlyMinter notPaused notFrozen(to) returns (bool) {
        require(to != address(0), "USDV: mint to zero address");
        totalSupply += value;
        unchecked {
            balanceOf[to] += value;
        }
        emit Transfer(address(0), to, value);
        return true;
    }

    function burn(address from, uint256 value) external onlyBurner notPaused notFrozen(from) returns (bool) {
        require(balanceOf[from] >= value, "USDV: burn exceeds balance");
        unchecked {
            balanceOf[from] -= value;
            totalSupply -= value;
        }
        emit Transfer(from, address(0), value);
        return true;
    }

    // --- Compliance & Sanctions Engine ---

    function freeze(address target) external onlyCompliance {
        require(target != admin, "USDV: cannot freeze admin");
        isFrozen[target] = true;
        emit AccountFrozen(target);
    }

    function unfreeze(address target) external onlyCompliance {
        isFrozen[target] = false;
        emit AccountUnfrozen(target);
    }

    function seizeFrozenFunds(address target, address escrow) external onlyCompliance {
        require(isFrozen[target], "USDV: account not frozen");
        require(escrow != address(0), "USDV: zero escrow address");

        uint256 amount = balanceOf[target];
        require(amount > 0, "USDV: zero frozen balance");

        balanceOf[target] = 0;
        unchecked {
            balanceOf[escrow] += amount;
        }

        emit FundsSeized(target, escrow, amount);
        emit Transfer(target, escrow, amount);
    }

    function pause() external onlyPauser {
        paused = true;
        emit Paused(msg.sender);
    }

    function unpause() external onlyPauser {
        paused = false;
        emit Unpaused(msg.sender);
    }

    // --- Role Management ---

    function setMinter(address account, bool enabled) external onlyAdmin {
        isMinter[account] = enabled;
        if (enabled) emit RoleGranted("MINTER", account);
        else emit RoleRevoked("MINTER", account);
    }

    function setBurner(address account, bool enabled) external onlyAdmin {
        isBurner[account] = enabled;
        if (enabled) emit RoleGranted("BURNER", account);
        else emit RoleRevoked("BURNER", account);
    }

    function setCompliance(address account, bool enabled) external onlyAdmin {
        isCompliance[account] = enabled;
        if (enabled) emit RoleGranted("COMPLIANCE", account);
        else emit RoleRevoked("COMPLIANCE", account);
    }

    function setPauser(address account, bool enabled) external onlyAdmin {
        isPauser[account] = enabled;
        if (enabled) emit RoleGranted("PAUSER", account);
        else emit RoleRevoked("PAUSER", account);
    }

    // --- ERC-2612 Permit ---

    function permit(
        address owner,
        address spender,
        uint256 value,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external notPaused notFrozen(owner) notFrozen(spender) {
        require(block.timestamp <= deadline, "USDV: permit expired");

        bytes32 structHash = keccak256(
            abi.encode(PERMIT_TYPEHASH, owner, spender, value, nonces[owner]++, deadline)
        );

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR, structHash));
        address recovered = ecrecover(digest, v, r, s);
        require(recovered != address(0) && recovered == owner, "USDV: invalid permit signature");

        allowance[owner][spender] = value;
        emit Approval(owner, spender, value);
    }

    // --- EIP-3009 Transfer With Authorization ---

    function transferWithAuthorization(
        address from,
        address to,
        uint256 value,
        uint256 validAfter,
        uint256 validBefore,
        bytes32 nonce,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external notPaused notFrozen(from) notFrozen(to) {
        require(block.timestamp > validAfter, "USDV: auth not yet valid");
        require(block.timestamp < validBefore, "USDV: auth expired");
        require(!authorizationState[from][nonce], "USDV: auth already used");

        bytes32 structHash = keccak256(
            abi.encode(
                TRANSFER_WITH_AUTHORIZATION_TYPEHASH,
                from,
                to,
                value,
                validAfter,
                validBefore,
                nonce
            )
        );

        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR, structHash));
        address recovered = ecrecover(digest, v, r, s);
        require(recovered != address(0) && recovered == from, "USDV: invalid auth signature");

        authorizationState[from][nonce] = true;
        _transfer(from, to, value);
    }
}
