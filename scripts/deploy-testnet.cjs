/**
 * Deploys the Trestly contract to Stellar testnet without needing stellar-cli.
 *
 * Uses @stellar/stellar-sdk directly (already a dependency of trestly-sdk) to:
 *   1. Generate + fund a throwaway deployer keypair via Friendbot.
 *   2. Upload the compiled contract wasm.
 *   3. Create a contract instance from that wasm.
 *   4. Save the result to scripts/testnet-deployer.json (gitignored).
 *
 * Usage: node scripts/deploy-testnet.cjs
 */
const fs = require("fs");
const path = require("path");
const https = require("https");
const {
  Keypair,
  TransactionBuilder,
  BASE_FEE,
  Networks,
  Operation,
  rpc,
  scValToNative,
  Address,
} = require(path.join(__dirname, "..", "..", "trestly-sdk", "node_modules", "@stellar", "stellar-sdk"));

const RPC_URL = "https://soroban-testnet.stellar.org";
const NETWORK_PASSPHRASE = Networks.TESTNET;
const WASM_PATH = path.join(__dirname, "..", "target", "wasm32v1-none", "release", "trestly.wasm");
const OUTPUT_PATH = path.join(__dirname, "testnet-deployer.json");

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

async function submit(server, keypair, operation) {
  const account = await server.getAccount(keypair.publicKey());
  const built = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(operation)
    .setTimeout(180)
    .build();

  const simulated = await server.simulateTransaction(built);
  if (rpc.Api.isSimulationError(simulated)) {
    throw new Error(`Simulation failed: ${simulated.error}`);
  }

  const prepared = rpc.assembleTransaction(built, simulated).build();
  prepared.sign(keypair);

  const sendResult = await server.sendTransaction(prepared);
  if (sendResult.status === "ERROR") {
    throw new Error(`Send failed: ${JSON.stringify(sendResult.errorResult)}`);
  }

  let status;
  for (let i = 0; i < 30; i++) {
    status = await server.getTransaction(sendResult.hash);
    if (status.status === "SUCCESS") return { hash: sendResult.hash, result: status };
    if (status.status === "FAILED") {
      throw new Error(`Transaction failed: ${JSON.stringify(status)}`);
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error("Transaction confirmation timeout");
}

async function main() {
  if (!fs.existsSync(WASM_PATH)) {
    throw new Error(`Wasm not found at ${WASM_PATH}. Run: cargo build --target wasm32v1-none --release`);
  }
  const wasm = fs.readFileSync(WASM_PATH);
  console.log(`Loaded wasm: ${wasm.length} bytes`);

  const server = new rpc.Server(RPC_URL);

  const deployer = Keypair.random();
  console.log(`Deployer: ${deployer.publicKey()}`);
  console.log("Funding via Friendbot...");
  await friendbot(deployer.publicKey());
  console.log("Funded.");

  console.log("Uploading contract wasm...");
  const uploadOp = Operation.uploadContractWasm({ wasm });
  const uploadResult = await submit(server, deployer, uploadOp);
  const wasmHash = scValToNative(uploadResult.result.returnValue);
  console.log(`Uploaded. Wasm hash: ${Buffer.from(wasmHash).toString("hex")}`);
  console.log(`  tx: ${uploadResult.hash}`);

  console.log("Creating contract instance...");
  const createOp = Operation.createCustomContract({
    address: new Address(deployer.publicKey()),
    wasmHash: Buffer.from(wasmHash),
  });
  const createResult = await submit(server, deployer, createOp);
  const contractAddress = scValToNative(createResult.result.returnValue);
  console.log(`Contract deployed: ${contractAddress}`);
  console.log(`  tx: ${createResult.hash}`);

  const output = {
    network: "testnet",
    rpcUrl: RPC_URL,
    networkPassphrase: NETWORK_PASSPHRASE,
    contractId: contractAddress,
    deployerPublicKey: deployer.publicKey(),
    deployerSecret: deployer.secret(),
    uploadTxHash: uploadResult.hash,
    createTxHash: createResult.hash,
    deployedAt: new Date().toISOString(),
  };
  fs.writeFileSync(OUTPUT_PATH, JSON.stringify(output, null, 2));
  console.log(`\nSaved deployment info to ${OUTPUT_PATH}`);
  console.log(`\nContract ID: ${contractAddress}`);
}

main().catch((err) => {
  console.error("Deployment failed:", err);
  process.exit(1);
});
