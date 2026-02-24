import { Program, AnchorProvider } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import idlJson from "./idl.json";

export const connection = new Connection(
  process.env.NEXT_PUBLIC_RPC_URL || "http://localhost:8899",
  "confirmed"
);

export const programId = new PublicKey(
  "6PyMsXWBKo77maWZir1kpE8i71Kuwprgm5hR9e5Ng2r3"
);

export function getProgram(provider: AnchorProvider): Program {
  return new Program(idlJson as any, provider as any);
}

export { idlJson as idl };
