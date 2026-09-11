/**
 * End-to-end proof that the deployed Trestly contract + trestly-sdk work together
 * on live testnet, without Freighter or stellar-cli — signing is done directly
 * with generated Keypairs via the same signTransaction(xdr) callback shape the
 * app hands the SDK.
 *
 * Runs both contract paths:
 *   A. Happy path:  create_payment -> (wait for window) -> release
 *   B. Dispute path: create_payment -> raise_dispute -> resolve_dispute (refund)
 *
 * Usage: node scripts/e2e-testnet.cjs
 * Requires scripts/testnet-deployer.json (run deploy-testnet.cjs first).
 */
const fs = require("fs");
const path = require("path");
const https = require("https");

const stellarSdkPath = path.join(__dirname, "..", "..", "trestly-sdk", "node_modules", "@stellar", "stellar-sdk");
const { Keypair, TransactionBuilder, Networks, Asset } = require(stellarSdkPath);
const sdk = require(path.join(__dirname, "..", "..", "trestly-sdk", "dist", "cjs", "index.js"));

const DEPLOY_INFO_PATH = path.join(__dirname, "testnet-deployer.json");
const NETWORK_PASSPHRASE = Networks.TESTNET;

function friendbot(publicKey) {
  return new Promise((resolve, reject) => {
    https
      .get(`https://friendbot.stellar.org?addr=${encodeURIComponent(publicKey)}`, (res) => {
        let body = "";
        res.on("data", (d) => (body += d));
        res.on("end", () => {
          if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) resolve(body);
          else reject(new Error(`Friendbot funding failed (${res.statusCode}): ${body}`));
        });
      })
      .on("error", reject);
  });
}

function signerFor(keypair) {
  return async (xdr) => {
    const tx = TransactionBuilder.fromXDR(xdr, NETWORK_PASSPHRASE);
    tx.sign(keypair);
    return tx.toXDR();
  };
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function main() {
  if (!fs.existsSync(DEPLOY_INFO_PATH)) {
    throw new Error(`Missing ${DEPLOY_INFO_PATH}. Run scripts/deploy-testnet.cjs first.`);
  }
  const deployInfo = JSON.parse(fs.readFileSync(DEPLOY_INFO_PATH, "utf8"));
  const config = {
    contractId: deployInfo.contractId,
    rpcUrl: deployInfo.rpcUrl,
    networkPassphrase: deployInfo.networkPassphrase,
  };
  console.log(`Contract: ${config.contractId}\n`);

  const arbiter = Keypair.fromSecret(deployInfo.deployerSecret);
  const payer = Keypair.random();
  const payee = Keypair.random();

  console.log(`Payer:   ${payer.publicKey()}`);
  console.log(`Payee:   ${payee.publicKey()}`);
  console.log(`Arbiter: ${arbiter.publicKey()} (reused deployer, already funded)\n`);

  console.log("Funding payer + payee via Friendbot...");
  await friendbot(payer.publicKey());
  await friendbot(payee.publicKey());
  console.log("Funded.\n");

  const nativeToken = Asset.native().contractId(NETWORK_PASSPHRASE);
  const amount = 10_000_000n; // 1 XLM (7 decimals)

  // ---------- Path A: happy path (auto release after window) ----------
  console.log("=== Path A: create -> wait -> release ===");
  const windowSecsA = 8;
  const createA = await sdk.createPayment(config, {
    payer: payer.publicKey(),
    payee: payee.publicKey(),
    token: nativeToken,
    amount,
    disputeWindowSecs: windowSecsA,
    arbiter: arbiter.publicKey(),
    signTransaction: signerFor(payer),
  });
  console.log(`Created payment #${createA.paymentId}  tx: ${createA.txHash}`);

  let paymentA = await sdk.getPayment(config, createA.paymentId);
  console.log(`  disputed=${paymentA.disputed} resolved=${paymentA.resolved}`);

  console.log(`Waiting ${windowSecsA + 5}s for dispute window to close...`);
  await sleep((windowSecsA + 5) * 1000);

  const releaseResult = await sdk.release(
    config,
    createA.paymentId,
    payee.publicKey(),
    signerFor(payee)
  );
  console.log(`Released  tx: ${releaseResult.txHash}`);

  paymentA = await sdk.getPayment(config, createA.paymentId);
  console.log(`  disputed=${paymentA.disputed} resolved=${paymentA.resolved} (expect resolved=true)\n`);
  if (!paymentA.resolved) throw new Error("Path A FAILED: payment not resolved after release");

  // ---------- Path B: dispute path ----------
  console.log("=== Path B: create -> raise_dispute -> resolve_dispute (refund to payer) ===");
  const windowSecsB = 300;
  const createB = await sdk.createPayment(config, {
    payer: payer.publicKey(),
    payee: payee.publicKey(),
    token: nativeToken,
    amount,
    disputeWindowSecs: windowSecsB,
    arbiter: arbiter.publicKey(),
    signTransaction: signerFor(payer),
  });
  console.log(`Created payment #${createB.paymentId}  tx: ${createB.txHash}`);

  const disputeResult = await sdk.raiseDispute(config, {
    paymentId: createB.paymentId,
    payer: payer.publicKey(),
    signTransaction: signerFor(payer),
  });
  console.log(`Raised dispute  tx: ${disputeResult.txHash}`);

  let paymentB = await sdk.getPayment(config, createB.paymentId);
  console.log(`  disputed=${paymentB.disputed} resolved=${paymentB.resolved} (expect disputed=true)`);
  if (!paymentB.disputed) throw new Error("Path B FAILED: dispute not recorded");

  const resolveResult = await sdk.resolveDispute(config, {
    paymentId: createB.paymentId,
    arbiter: arbiter.publicKey(),
    refundToPayer: true,
    signTransaction: signerFor(arbiter),
  });
  console.log(`Resolved dispute (refund to payer)  tx: ${resolveResult.txHash}`);

  paymentB = await sdk.getPayment(config, createB.paymentId);
  console.log(`  disputed=${paymentB.disputed} resolved=${paymentB.resolved} (expect resolved=true)\n`);
  if (!paymentB.resolved) throw new Error("Path B FAILED: dispute not resolved");

  console.log("ALL PATHS PASSED.");
  console.log(`\nView on Stellar Expert:`);
  console.log(`  https://stellar.expert/explorer/testnet/contract/${config.contractId}`);
  console.log(`  https://stellar.expert/explorer/testnet/tx/${createA.txHash}`);
  console.log(`  https://stellar.expert/explorer/testnet/tx/${releaseResult.txHash}`);
  console.log(`  https://stellar.expert/explorer/testnet/tx/${disputeResult.txHash}`);
  console.log(`  https://stellar.expert/explorer/testnet/tx/${resolveResult.txHash}`);
}

main().catch((err) => {
  console.error("\nE2E FAILED:", err);
  process.exit(1);
});
