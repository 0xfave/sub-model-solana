"use client";

import React, { createContext, useContext, useState, useEffect } from "react";
import { createSolanaClient } from "gill";
import {
  ConnectionProvider,
  WalletProvider,
  useWallet as useAdapterWallet,
} from "@solana/wallet-adapter-react";
import { PhantomWalletAdapter } from "@solana/wallet-adapter-wallets";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import { Connection, Transaction, PublicKey } from "@solana/web3.js";

const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || "http://localhost:8899";

export const client = createSolanaClient({
  urlOrMoniker: RPC_URL,
});

export const connection = new Connection(RPC_URL, "confirmed");

interface WalletState {
  connected: boolean;
  publicKey: PublicKey | null;
  signTransaction: ((tx: Transaction) => Promise<Transaction>) | null;
  signAllTransactions: ((txs: Transaction[]) => Promise<Transaction[]>) | null;
}

const WalletContext = createContext<WalletState>({
  connected: false,
  publicKey: null,
  signTransaction: null,
  signAllTransactions: null,
});

export const useWalletState = () => useContext(WalletContext);

export function WalletProviderWrapper({
  children,
}: {
  children: React.ReactNode;
}) {
  const wallets = React.useMemo(() => [new PhantomWalletAdapter()], []);

  return (
    <ConnectionProvider endpoint={RPC_URL}>
      <WalletProvider wallets={wallets} autoConnect={false}>
        <WalletModalProvider>
          <WalletConnectionHandler>{children}</WalletConnectionHandler>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}

function WalletConnectionHandler({ children }: { children: React.ReactNode }) {
  const adapterWallet = useAdapterWallet();

  const [state, setState] = useState<WalletState>({
    connected: false,
    publicKey: null,
    signTransaction: null,
    signAllTransactions: null,
  });

  useEffect(() => {
    if (adapterWallet.connected && adapterWallet.publicKey) {
      setState({
        connected: true,
        publicKey: adapterWallet.publicKey,
        signTransaction: adapterWallet.signTransaction ?? null,
        signAllTransactions: adapterWallet.signAllTransactions ?? null,
      });
    } else {
      setState({
        connected: false,
        publicKey: null,
        signTransaction: null,
        signAllTransactions: null,
      });
    }
  }, [
    adapterWallet.connected,
    adapterWallet.publicKey,
    adapterWallet.signTransaction,
    adapterWallet.signAllTransactions,
  ]);

  return (
    <WalletContext.Provider value={state}>{children}</WalletContext.Provider>
  );
}
