# Legal Memorandum: Regulatory Classification of USDV & Protocol Gas
## Analysis Under Federal Securities and Commodities Laws

**To**: Board of Directors, VeriDAG Sovereign Settlement Labs, Inc.  
**From**: Special FinTech & Regulatory Counsel  
**Date**: September 2026  
**Subject**: Regulatory Status of Sovereign Digital Dollar (USDV) and Native Protocol Settlement Units Under the Securities Act of 1933, Securities Exchange Act of 1934, and Commodity Exchange Act

---

## 1. Executive Summary & Legal Conclusion

Based on applicable federal statutes, judicial precedent, and administrative guidance:

1. **USDV is NOT an "Investment Contract" (Security)**: Applying the four-prong test articulated in *SEC v. W.J. Howey Co.*, 328 U.S. 293 (1946), USDV lacks the essential element of a "reasonable expectation of profits derived from the entrepreneurial or managerial efforts of others." USDV is a 100% reserve-backed payment instrument pegged 1:1 to the US Dollar without interest or dividend yield passed to retail holders.
2. **USDV is NOT a "Note" Under the Reves Family Resemblance Test**: Under *Reves v. Ernst & Young*, 494 U.S. 56 (1990), USDV is issued to facilitate commercial cash management and cross-border trade settlement rather than for investment or capital appreciation.
3. **Consensus Gas / Computation Metering is a Consumptive Utility**: Gas consumption within the `veridag-wasm-runtime` is consumed strictly to meter CPU and state storage resources, bearing no characteristics of an investment contract.

---

## 2. Four-Prong Analysis Under *SEC v. W.J. Howey Co.*

Under *Howey*, an instrument is an investment contract if it represents:
1. An investment of money,
2. In a common enterprise,
3. With a reasonable expectation of profits,
4. Derived solely from the entrepreneurial or managerial efforts of others.

### Prong 1: Investment of Money
While purchasers exchange US fiat currency for USDV, the exchange is a dollar-for-dollar commercial purchase for immediate liquidity and trade clearing, analogous to purchasing traveler's checks or commercial warehouse receipts, rather than speculative capital commitment.

### Prong 2: Common Enterprise
USDV reserves are segregated in qualified bankruptcy-remote accounts held in trust for token holders. However, even if horizontal commonality were found among reserve holdings, the instrument fails Prongs 3 and 4 as a matter of law.

### Prong 3: Expectation of Profit (The Crucial Failure)
The Supreme Court held in *United Housing Corp. v. Forman*, 421 U.S. 837 (1975), that where a purchaser is motivated by a desire to use or consume the item purchased, securities laws do not apply. 
- USDV is designed strictly as a stable store of value ($1.00 USDV = $1.00 USD).
- USDV tokens pay zero interest to holders.
- Promotional collateral and documentation disclaim any possibility of price appreciation.
- The instrument is incapable of capital gains; hence, holders have no expectation of profit.

### Prong 4: Efforts of Others
The stability of USDV is maintained by passive 100% mathematical reserve matching in short-term US Treasury Bills and FDIC cash, not through the managerial discretion, lending risk, or entrepreneurial business strategy of the issuer.

---

## 3. Commodity Exchange Act (CEA) Considerations

Under Section 1a(9) of the Commodity Exchange Act (7 U.S.C. § 1a(9)), virtual currencies are broadly recognized by the CFTC as commodities (*CFTC v. McDonnell*, 287 F. Supp. 3d 213 (E.D.N.Y. 2018)).
- Spot transactions in USDV for commercial trade settlement fall outside CFTC registration requirements, provided no leveraged retail margin or derivative contracts are marketed by the platform.
- Protocol bridges (e.g., `veridag-ethereum` and `veridag-bitcoin`) act purely as deterministic cryptographic data relays and do not constitute designated contract markets (DCMs) or swap execution facilities (SEFs).

---

## 4. Operational Recommendations

To preserve this legal classification in perpetuity:
1. **Never Distribute Native Yield to Token Holders**: Any yield accrued from the Treasury reserve float must be allocated to infrastructure operating costs, consortium validator staking rewards, or protocol reserve buffers—never directly to USDV balances.
2. **Maintain Strict Parity Marketing**: All commercial documentation, web assets, and communications must present USDV exclusively as a settlement utility for USMCA trade, enterprise clearing, and agent micro-transactions.
3. **Monthly Third-Party CPA Verification**: Ensure monthly public publication of reserve attestations signed by an independent AICPA-accredited accounting firm to maintain the factual basis of the 1:1 backing.
