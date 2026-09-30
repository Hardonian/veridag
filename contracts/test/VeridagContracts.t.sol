// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity 0.8.28;

import "../USDV.sol";
import "../VeridagLightClient.sol";
import "../VeridagBridge.sol";

interface Vm {
    function prank(address sender) external;
    function expectRevert(bytes calldata reason) external;
}

contract VeridagContractsTest {
    Vm private constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    USDV private token;
    VeridagLightClient private lightClient;
    VeridagBridge private bridge;

    address private constant ALICE = address(0xA11CE);
    address private constant BOB = address(0xB0B);
    address private constant RELAYER_ONE = address(0x1001);
    address private constant RELAYER_TWO = address(0x1002);

    function setUp() public {
        token = new USDV(address(this));
        lightClient = new VeridagLightClient(address(this), bytes32(uint256(1)), bytes32(uint256(2)));
        bridge = new VeridagBridge(address(token), address(lightClient), address(this));
        token.setMinter(address(bridge), true);
        token.setBurner(address(bridge), true);
    }

    function testCheckpointRequiresRelayerThreshold() public {
        lightClient.setRelayer(RELAYER_ONE, true);
        lightClient.setRelayer(RELAYER_TWO, true);
        lightClient.setRelayerThreshold(3);

        lightClient.commitCheckpoint(1, 0, bytes32(uint256(11)), bytes32(uint256(12)), bytes32(0));
        require(lightClient.latestSequence() == 0, "checkpoint finalized with one approval");

        vm.prank(RELAYER_ONE);
        lightClient.commitCheckpoint(1, 0, bytes32(uint256(11)), bytes32(uint256(12)), bytes32(0));
        require(lightClient.latestSequence() == 0, "checkpoint finalized with two approvals");

        vm.prank(RELAYER_TWO);
        lightClient.commitCheckpoint(1, 0, bytes32(uint256(11)), bytes32(uint256(12)), bytes32(0));
        require(lightClient.latestSequence() == 1, "checkpoint did not finalize at threshold");
    }

    function testRelayerCannotApproveTwice() public {
        lightClient.setRelayer(RELAYER_ONE, true);
        lightClient.setRelayerThreshold(2);
        lightClient.commitCheckpoint(1, 0, bytes32(uint256(11)), bytes32(uint256(12)), bytes32(0));

        vm.expectRevert(bytes("VeridagLightClient: duplicate approval"));
        lightClient.commitCheckpoint(1, 0, bytes32(uint256(11)), bytes32(uint256(12)), bytes32(0));
    }

    function testWithdrawalFieldsAreBoundToProvenObject() public {
        bytes32 withdrawalId = keccak256("withdrawal-1");
        uint256 amount = 25_000_000;
        bytes memory objectData = abi.encode(withdrawalId, ALICE, amount);
        bytes32 objectId = keccak256("withdrawal-object");
        bytes32 stateRoot = sha256(abi.encodePacked("VERIDAG_BMH_LEAF_V1\x00", objectId, objectData));
        bytes32 checkpointId = keccak256("checkpoint-1");

        lightClient.commitCheckpoint(1, 0, checkpointId, stateRoot, bytes32(0));
        bridge.finalizeWithdrawal(
            checkpointId,
            objectId,
            objectData,
            new bytes32[](0),
            new bool[](0),
            ALICE,
            amount,
            withdrawalId
        );
        require(token.balanceOf(ALICE) == amount, "withdrawal was not minted");

        vm.expectRevert(bytes("VeridagBridge: withdrawal fields do not match object data"));
        bridge.finalizeWithdrawal(
            checkpointId,
            objectId,
            objectData,
            new bytes32[](0),
            new bool[](0),
            BOB,
            amount,
            keccak256("different-withdrawal")
        );
    }

    function testDepositBurnsExactlyTheRequestedAmount() public {
        token.mint(ALICE, 50_000_000);
        vm.prank(ALICE);
        bridge.depositUSDV(bytes32(uint256(123)), 12_000_000);
        require(token.balanceOf(ALICE) == 38_000_000, "unexpected post-deposit balance");
        require(token.totalSupply() == 38_000_000, "deposit did not conserve supply");
    }

    function testPausedBridgeRejectsDeposits() public {
        token.mint(ALICE, 1_000_000);
        bridge.setPaused(true);
        vm.prank(ALICE);
        vm.expectRevert(bytes("VeridagBridge: paused"));
        bridge.depositUSDV(bytes32(uint256(123)), 1);
    }

    function testFuzzTransferPreservesSupply(uint96 minted, uint96 transferred) public {
        uint256 amount = uint256(minted) + 1;
        uint256 move = uint256(transferred) % (amount + 1);
        token.mint(ALICE, amount);
        uint256 supplyBefore = token.totalSupply();
        vm.prank(ALICE);
        token.transfer(BOB, move);
        require(token.totalSupply() == supplyBefore, "transfer changed supply");
        require(token.balanceOf(ALICE) + token.balanceOf(BOB) == supplyBefore, "balances do not sum to supply");
    }

    function testTwoStepAdminTransfer() public {
        token.transferAdmin(ALICE);
        vm.prank(ALICE);
        token.acceptAdmin();
        require(token.admin() == ALICE, "admin transfer failed");
        require(token.pendingAdmin() == address(0), "pending admin not cleared");
    }
}
