// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity 0.8.28;

import "../USDV.sol";
import "../VeridagLightClient.sol";
import "../VeridagBridge.sol";

abstract contract MinimalInvariantTargeting {
    struct FuzzSelector {
        address addr;
        bytes4[] selectors;
    }

    struct FuzzArtifactSelector {
        string artifact;
        bytes4[] selectors;
    }

    struct FuzzInterface {
        address addr;
        string[] artifacts;
    }

    address[] private targeted;

    function targetContract(address target) internal {
        targeted.push(target);
    }

    function targetContracts() public view returns (address[] memory) {
        return targeted;
    }

    function excludeContracts() public pure returns (address[] memory) {
        return new address[](0);
    }

    function targetSenders() public pure returns (address[] memory) {
        return new address[](0);
    }

    function excludeSenders() public pure returns (address[] memory) {
        return new address[](0);
    }

    function targetArtifacts() public pure returns (string[] memory) {
        return new string[](0);
    }

    function excludeArtifacts() public pure returns (string[] memory) {
        return new string[](0);
    }

    function targetSelectors() public pure returns (FuzzSelector[] memory) {
        return new FuzzSelector[](0);
    }

    function excludeSelectors() public pure returns (FuzzSelector[] memory) {
        return new FuzzSelector[](0);
    }

    function targetArtifactSelectors() public pure returns (FuzzArtifactSelector[] memory) {
        return new FuzzArtifactSelector[](0);
    }

    function targetInterfaces() public pure returns (FuzzInterface[] memory) {
        return new FuzzInterface[](0);
    }
}

contract USDVInvariantHandler {
    USDV public immutable token;
    VeridagBridge public immutable bridge;

    address public constant ALICE = address(0xA11CE);
    address public constant BOB = address(0xB0B);

    uint64 public successfulDeposits;

    constructor(USDV token_, VeridagBridge bridge_) {
        token = token_;
        bridge = bridge_;
    }

    function mint(uint8 actorIndex, uint96 amount) external {
        token.mint(_actor(actorIndex), uint256(amount));
    }

    function transferFromHandler(bool toAlice, uint96 rawAmount) external {
        uint256 balance = token.balanceOf(address(this));
        if (balance == 0) return;

        uint256 amount = uint256(rawAmount) % (balance + 1);
        token.transfer(toAlice ? ALICE : BOB, amount);
    }

    function burn(uint8 actorIndex, uint96 rawAmount) external {
        address actor = _actor(actorIndex);
        uint256 balance = token.balanceOf(actor);
        if (balance == 0) return;

        uint256 amount = uint256(rawAmount) % (balance + 1);
        token.burn(actor, amount);
    }

    function deposit(uint96 rawAmount, bytes32 recipient) external {
        uint256 balance = token.balanceOf(address(this));
        if (balance == 0) return;

        uint256 amount = (uint256(rawAmount) % balance) + 1;
        if (recipient == bytes32(0)) recipient = bytes32(uint256(1));
        bridge.depositUSDV(recipient, amount);
        successfulDeposits++;
    }

    function _actor(uint8 actorIndex) private view returns (address) {
        uint8 selected = actorIndex % 3;
        if (selected == 0) return address(this);
        if (selected == 1) return ALICE;
        return BOB;
    }
}

contract VeridagInvariantTest is MinimalInvariantTargeting {
    USDV private token;
    VeridagLightClient private lightClient;
    VeridagBridge private bridge;
    USDVInvariantHandler private handler;

    function setUp() public {
        token = new USDV(address(this));
        lightClient = new VeridagLightClient(address(this), bytes32(uint256(1)), bytes32(uint256(2)));
        bridge = new VeridagBridge(address(token), address(lightClient), address(this));
        handler = new USDVInvariantHandler(token, bridge);

        token.setMinter(address(handler), true);
        token.setBurner(address(handler), true);
        token.setBurner(address(bridge), true);

        targetContract(address(handler));
    }

    function invariant_supplyEqualsTrackedBalances() public view {
        uint256 trackedBalances =
            token.balanceOf(address(handler)) + token.balanceOf(handler.ALICE()) + token.balanceOf(handler.BOB());
        require(token.totalSupply() == trackedBalances, "USDV supply diverged from balances");
    }

    function invariant_depositSequenceIsExact() public view {
        require(bridge.depositSequence() == handler.successfulDeposits(), "bridge deposit sequence diverged");
    }

    function invariant_bridgeBindingsDoNotChange() public view {
        require(address(bridge.usdv()) == address(token), "bridge token binding changed");
        require(address(bridge.lightClient()) == address(lightClient), "bridge light-client binding changed");
    }
}
