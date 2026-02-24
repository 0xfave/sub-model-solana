"use client";

import { useState, useEffect, useCallback } from "react";
import dynamic from "next/dynamic";
import {
  Wallet,
  RefreshCw,
  X,
  Plus,
  Inbox,
  Layers,
  CreditCard,
  Search,
  Zap,
  Loader2,
} from "lucide-react";
import { useWalletState } from "@/context/WalletContext";
import { connection, programId, getProgram } from "@/lib/anchor";
import { PublicKey, Transaction, SystemProgram } from "@solana/web3.js";
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
  createInitializeAccountInstruction,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { AnchorProvider } from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";

const WalletMultiButton = dynamic(
  () =>
    import("@solana/wallet-adapter-react-ui").then(
      (mod) => mod.WalletMultiButton
    ),
  {
    ssr: false,
    loading: () => (
      <button className="flex items-center gap-2 px-4 py-2 bg-[#c5a059] hover:bg-[#d4af37] text-[#051d19] rounded text-sm font-bold transition-colors">
        <Wallet className="w-4 h-4" /> Connect Wallet
      </button>
    ),
  }
);

interface Plan {
  publicKey: PublicKey;
  planId: string;
  price: number;
  durationSeconds: number;
  trialDays: number;
  tokenMint: PublicKey;
  owner: PublicKey;
  description: string;
}

interface Subscription {
  publicKey: PublicKey;
  plan: PublicKey;
  status: number;
  currentPeriodEnd: number;
  cancelAtPeriodEnd?: boolean;
}

function EmptyState({
  message,
  icon: Icon,
}: {
  message: string;
  icon: React.ElementType;
}) {
  return (
    <div className="bg-white dark:bg-[#c5a059]/5 border border-slate-200 dark:border-[#c5a059]/20 rounded-xl p-8 flex flex-col items-center justify-center text-center min-h-[200px]">
      <Icon className="w-10 h-10 text-[#c5a059] mb-3" />
      <p className="text-slate-500 font-medium">{message}</p>
    </div>
  );
}

export default function Dashboard() {
  const wallet = useWalletState();
  const [plans, setPlans] = useState<Plan[]>([]);
  const [userSubscriptions, setUserSubscriptions] = useState<Subscription[]>(
    []
  );
  const [loading, setLoading] = useState(true);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showCancelModal, setShowCancelModal] = useState(false);
  const [cancelSubscription, setCancelSubscription] =
    useState<Subscription | null>(null);
  const [processing, setProcessing] = useState<string | null>(null);
  const [toast, setToast] = useState<{
    message: string;
    type: "success" | "error";
  } | null>(null);

  const showToast = (message: string, type: "success" | "error") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 4000);
  };

  const SOL_MINT = new PublicKey("So11111111111111111111111111111111111111112");

  const fetchData = useCallback(async () => {
    try {
      let provider = getProvider();
      if (!provider) {
        provider = new AnchorProvider(
          connection,
          {} as any,
          AnchorProvider.defaultOptions()
        );
      }

      const program = getProgram(provider);

      const programAny = program as any;

      const fetchedPlans: any = await programAny.account.plan.all();
      const fetchedSubs: any = await programAny.account.subscription.all();

      const plans: Plan[] = fetchedPlans.map((p: any) => ({
        publicKey: p.publicKey,
        planId: p.account.planId || "unknown",
        price: p.account.price?.toNumber?.() || p.account.price || 0,
        durationSeconds:
          p.account.durationSeconds?.toNumber?.() ||
          p.account.durationSeconds ||
          0,
        trialDays:
          p.account.trialDays?.toNumber?.() || p.account.trialDays || 0,
        tokenMint: p.account.tokenMint
          ? new PublicKey(p.account.tokenMint)
          : SOL_MINT,
        owner: new PublicKey(p.account.owner),
        description: "Subscription Plan",
      }));

      const subs: Subscription[] = fetchedSubs
        .filter(
          (s: any) =>
            wallet.publicKey && s.account.user.equals(wallet.publicKey)
        )
        .map((s: any) => {
          let statusNum = 0;
          const statusVal = s.account.status;
          if (typeof statusVal === "number") {
            statusNum = statusVal;
          } else if (typeof statusVal === "object") {
            const statusKeys = Object.keys(statusVal);
            if (statusKeys.length > 0) {
              const statusMap: Record<string, number> = {
                Trialing: 0,
                Active: 1,
                PastDue: 3,
                Unpaid: 4,
                Canceled: 5,
                Paused: 6,
              };
              statusNum = statusMap[statusKeys[0]] ?? 0;
            }
          }
          return {
            publicKey: s.publicKey,
            plan: new PublicKey(s.account.plan),
            status: statusNum,
            currentPeriodEnd:
              s.account.currentPeriodEnd?.toNumber?.() ||
              s.account.currentPeriodEnd ||
              0,
            cancelAtPeriodEnd: s.account.cancelAtPeriodEnd,
          };
        });

      setPlans(plans);
      setUserSubscriptions(subs);
    } catch (error) {
      console.error("Error fetching data:", error);
      setPlans([]);
      setUserSubscriptions([]);
    }
  }, [wallet.publicKey]);

  useEffect(() => {
    setLoading(true);
    fetchData().finally(() => setLoading(false));
  }, [fetchData]);

  const getProvider = useCallback(() => {
    if (!wallet.publicKey) return null;
    return new AnchorProvider(
      connection,
      {
        publicKey: wallet.publicKey,
        signTransaction: wallet.signTransaction as
          | ((tx: Transaction) => Promise<Transaction>)
          | undefined,
        signAllTransactions: wallet.signAllTransactions as
          | ((txs: Transaction[]) => Promise<Transaction[]>)
          | undefined,
      } as any,
      AnchorProvider.defaultOptions()
    );
  }, [wallet.publicKey, wallet.signTransaction, wallet.signAllTransactions]);

  const getOrCreateWsolAccount = async (
    provider: AnchorProvider,
    userPubkey: PublicKey,
    amount: number
  ): Promise<PublicKey> => {
    const wSOLMint = new PublicKey(
      "So11111111111111111111111111111111111111112"
    );

    const wSOLAta = await getAssociatedTokenAddress(
      wSOLMint,
      userPubkey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const existingAta = await connection.getParsedTokenAccountsByOwner(
      userPubkey,
      { mint: wSOLMint }
    );

    if (existingAta.value.length > 0) {
      return existingAta.value[0].pubkey;
    }

    const rent = await connection.getMinimumBalanceForRentExemption(165);
    const lamports = amount + rent;

    const transaction = new Transaction();

    transaction.add(
      createAssociatedTokenAccountInstruction(
        userPubkey,
        wSOLAta,
        userPubkey,
        wSOLMint,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );

    transaction.add(
      SystemProgram.transfer({
        fromPubkey: userPubkey,
        toPubkey: wSOLAta,
        lamports,
      })
    );

    transaction.feePayer = userPubkey;
    transaction.recentBlockhash = (
      await connection.getLatestBlockhash()
    ).blockhash;

    const signedTx = await provider.wallet.signTransaction(transaction);
    if (!signedTx) throw new Error("Failed to sign transaction");

    const sig = await connection.sendRawTransaction(signedTx.serialize());
    await connection.confirmTransaction(sig);

    return wSOLAta;
  };

  const handleSubscribe = async (plan: Plan) => {
    if (!wallet.connected || !wallet.publicKey) {
      showToast("Please connect your wallet first", "error");
      return;
    }

    try {
      setProcessing(plan.publicKey.toBase58());
      const provider = getProvider();
      if (!provider) throw new Error("Provider not available");

      const program = getProgram(provider);

      const SOL_MINT = new PublicKey(
        "So11111111111111111111111111111111111111112"
      );
      const isWsol = plan.tokenMint.equals(SOL_MINT);

      let userAta: PublicKey;

      const userTokenAccount = await connection.getParsedTokenAccountsByOwner(
        wallet.publicKey,
        { mint: plan.tokenMint }
      );

      if (userTokenAccount.value.length > 0) {
        userAta = userTokenAccount.value[0].pubkey;
      } else if (isWsol) {
        userAta = await getOrCreateWsolAccount(
          provider,
          wallet.publicKey,
          plan.price + 1000000
        );
      } else {
        showToast(
          "You need a token account for this token. Please add the token to your wallet first.",
          "error"
        );
        return;
      }

      const merchantTokenAccount =
        await connection.getParsedTokenAccountsByOwner(plan.owner, {
          mint: plan.tokenMint,
        });
      const merchantAta = merchantTokenAccount.value[0]?.pubkey;

      if (!merchantAta) {
        showToast(
          "The plan owner hasn't set up their token account yet.",
          "error"
        );
        return;
      }

      const [subscriptionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("subscription"),
          wallet.publicKey.toBuffer(),
          plan.publicKey.toBuffer(),
        ],
        programId
      );

      const tx = await program.methods
        .subscribe()
        .accounts({
          user: wallet.publicKey,
          plan: plan.publicKey,
          subscription: subscriptionPda,
          userTokenAccount: userAta,
          merchantTokenAccount: merchantAta,
          tokenProgram: new PublicKey(
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
          ),
          systemProgram: SystemProgram.programId,
        } as any)
        .transaction();

      const txSig = await provider.sendAndConfirm!(tx);
      console.log("Subscribe transaction:", txSig);

      await fetchData();
      showToast("Successfully subscribed!", "success");
    } catch (error: any) {
      console.error("Subscribe error:", error);
      showToast(error.message || "Failed to subscribe", "error");
    } finally {
      setProcessing(null);
    }
  };

  const handleRenew = async (sub: Subscription) => {
    if (!wallet.connected || !wallet.publicKey) {
      showToast("Please connect your wallet first", "error");
      return;
    }

    try {
      setProcessing(`renew-${sub.publicKey.toBase58()}`);
      const provider = getProvider();
      if (!provider) throw new Error("Provider not available");

      const program = getProgram(provider);

      const plan = plans.find((p) => p.publicKey.equals(sub.plan));
      if (!plan) {
        showToast("Plan not found", "error");
        return;
      }

      const userTokenAccount = await connection.getParsedTokenAccountsByOwner(
        wallet.publicKey,
        { mint: plan.tokenMint }
      );
      const userAta = userTokenAccount.value[0]?.pubkey;

      if (!userAta) {
        showToast("You need a token account for this token", "error");
        return;
      }

      const merchantTokenAccount =
        await connection.getParsedTokenAccountsByOwner(plan.owner, {
          mint: plan.tokenMint,
        });
      const merchantAta = merchantTokenAccount.value[0]?.pubkey;

      if (!merchantAta) {
        showToast(
          "The plan owner hasn't set up their token account yet",
          "error"
        );
        return;
      }

      const tx = await program.methods
        .renew()
        .accounts({
          user: wallet.publicKey,
          plan: sub.plan,
          subscription: sub.publicKey,
          userTokenAccount: userAta,
          merchantTokenAccount: merchantAta,
          tokenProgram: new PublicKey(
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
          ),
          systemProgram: SystemProgram.programId,
        } as any)
        .transaction();

      const txSig = await provider.sendAndConfirm!(tx);
      console.log("Renew transaction:", txSig);

      await fetchData();
      showToast("Successfully renewed!", "success");
    } catch (error: any) {
      console.error("Renew error:", error);
      showToast(error.message || "Failed to renew", "error");
    } finally {
      setProcessing(null);
    }
  };

  const handleCancel = async (
    sub: Subscription,
    immediate: boolean = false
  ) => {
    if (!wallet.connected || !wallet.publicKey) {
      showToast("Please connect your wallet first", "error");
      return;
    }

    try {
      setProcessing(`cancel-${sub.publicKey.toBase58()}`);
      const provider = getProvider();
      if (!provider) throw new Error("Provider not available");

      const program = getProgram(provider);

      const tx = await program.methods
        .cancel(immediate)
        .accounts({
          user: wallet.publicKey,
          plan: sub.plan,
          subscription: sub.publicKey,
        } as any)
        .transaction();

      const txSig = await provider.sendAndConfirm!(tx);
      console.log("Cancel transaction:", txSig);

      await fetchData();
      showToast("Successfully cancelled!", "success");
    } catch (error: any) {
      console.error("Cancel error:", error);
      showToast(error.message || "Failed to cancel", "error");
    } finally {
      setProcessing(null);
    }
  };

  const handleCreatePlan = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!wallet.connected || !wallet.publicKey) {
      showToast("Please connect your wallet first", "error");
      return;
    }

    const formData = new FormData(e.currentTarget);
    const planId = formData.get("planId") as string;
    const priceLamports = Math.round(
      parseFloat(formData.get("price") as string) * 1e9
    );
    const duration = parseInt(formData.get("duration") as string);
    const trialDays = parseInt(formData.get("trialDays") as string) || 0;

    if (!planId || !priceLamports || !duration) {
      showToast("Please fill in all fields", "error");
      return;
    }

    try {
      setProcessing("create-plan");
      const provider = getProvider();
      if (!provider) throw new Error("Provider not available");

      const program = getProgram(provider);

      const [planPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("plan"), wallet.publicKey.toBuffer(), Buffer.from(planId)],
        programId
      );

      const tx = await program.methods
        .createPlan(
          planId,
          1,
          new BN(priceLamports),
          new BN(duration),
          new BN(trialDays),
          SOL_MINT
        )
        .accounts({
          owner: wallet.publicKey,
          plan: planPda,
          tokenMintAccount: SOL_MINT,
          systemProgram: SystemProgram.programId,
        } as any)
        .transaction();

      const txSig = await provider.sendAndConfirm!(tx);
      console.log("Create plan transaction:", txSig);

      await fetchData();
      setShowCreateModal(false);
      showToast("Plan created successfully!", "success");
    } catch (error: any) {
      console.error("Create plan error:", error);
      showToast(error.message || "Failed to create plan", "error");
    } finally {
      setProcessing(null);
    }
  };

  const formatDuration = (seconds: number) => {
    if (seconds >= 31536000) return "Yearly";
    if (seconds >= 2592000) return "Monthly";
    if (seconds >= 604800) return "Weekly";
    return `${seconds / 86400} days`;
  };

  const formatSol = (lamports: number) => {
    return (lamports / 1e9).toFixed(1);
  };

  const getStatusLabel = (status: number) => {
    const labels = [
      "Trialing",
      "Active",
      "",
      "PastDue",
      "Unpaid",
      "Canceled",
      "Paused",
    ];
    return labels[status] || "Unknown";
  };

  return (
    <div className="layout-container flex h-full grow flex-col">
      <header className="flex items-center justify-between border-b border-primary/20 px-6 py-4 bg-[#f7f5f8] dark:bg-[#051d19] sticky top-0 z-50">
        <div className="flex items-center gap-4">
          <div className="size-8 bg-[#c5a059] rounded flex items-center justify-center text-[#051d19]">
            <CreditCard className="w-5 h-5" />
          </div>
          <h2 className="text-xl font-bold leading-tight tracking-tight hidden sm:block text-[#c5a059]">
            Solana Subscriptions
          </h2>
        </div>
        <div className="flex items-center gap-4">
          <WalletMultiButton />
        </div>
      </header>

      <main className="p-6 lg:px-10 max-w-[1600px] mx-auto w-full">
        <div className="flex flex-col gap-2 mb-8">
          <h1 className="text-4xl font-black tracking-tight text-[#c5a059] uppercase">
            Protocol Dashboard
          </h1>
          <p className="text-slate-500 dark:text-slate-400">
            Manage, explore, and create decentralized recurring payments on
            Solana.
          </p>
        </div>

        {loading ? (
          <div className="flex items-center justify-center py-20">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[#c5a059]"></div>
          </div>
        ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
          {/* Column 1: Your Subscriptions */}
          <section className="flex flex-col gap-4">
            <div className="flex items-center gap-2 mb-2">
              <RefreshCw className="w-5 h-5 text-[#c5a059]" />
              <h3 className="text-lg font-bold">Your Subscriptions</h3>
            </div>
        
            {userSubscriptions.length === 0 ? (
              <EmptyState
                message={
                  wallet.connected
                    ? "No active subscriptions"
                    : "Connect wallet to view subscriptions"
                }
                icon={wallet.connected ? Inbox : Wallet}
              />
            ) : (
              <>
                {userSubscriptions
                  .filter((sub) => sub.status !== 5)
                  .map((sub) => (
                    <div
                      key={sub.publicKey.toBase58()}
                      className="bg-white dark:bg-[#c5a059]/5 border border-slate-200 dark:border-[#c5a059]/20 rounded-xl p-5 flex flex-col gap-4 shadow-sm"
                    >
                      <div className="flex justify-between items-start">
                        <div>
                          <p className="text-[10px] font-bold uppercase tracking-[0.2em] text-slate-500 mb-1">
                            {getStatusLabel(sub.status)}
                          </p>
                          <p className="text-lg font-bold">
                            Plan: {sub.plan.toBase58().slice(0, 8)}...
                          </p>
                        </div>
                        <div className="size-12 rounded border border-slate-700 bg-slate-800/50 flex items-center justify-center">
                          <RefreshCw className="w-5 h-5 text-slate-500" />
                        </div>
                      </div>
                      <div className="flex justify-between text-sm">
                        <span className="text-slate-500">
                          {sub.cancelAtPeriodEnd ? "Ends" : "Next Payment"}
                        </span>
                        <span className="font-medium">
                          {new Date(sub.currentPeriodEnd * 1000).toLocaleDateString()}
                        </span>
                      </div>
                      {!sub.cancelAtPeriodEnd && (
                        <div className="grid grid-cols-2 gap-3 mt-2">
                          <button
                            onClick={() => {
                              setCancelSubscription(sub);
                              setShowCancelModal(true);
                            }}
                            disabled={!!processing}
                            className="flex items-center justify-center gap-2 px-3 py-2 border border-red-500/50 text-red-500 hover:bg-red-500/10 rounded text-xs font-bold transition-colors disabled:opacity-50"
                          >
                            {processing === `cancel-${sub.publicKey.toBase58()}` ? (
                              <Loader2 className="w-3 h-3 animate-spin" />
                            ) : (
                              <X className="w-3 h-3" />
                            )}
                            Cancel
                          </button>
                          <button
                            onClick={() => handleRenew(sub)}
                            disabled={!!processing}
                            className="flex items-center justify-center gap-2 px-3 py-2 bg-[#c5a059] text-[#051d19] hover:bg-[#d4af37] rounded text-xs font-bold transition-colors disabled:opacity-50"
                          >
                            {processing === `renew-${sub.publicKey.toBase58()}` ? (
                              <Loader2 className="w-3 h-3 animate-spin" />
                            ) : (
                              <RefreshCw className="w-3 h-3" />
                            )}
                            Renew
                          </button>
                        </div>
                      )}
                    </div>
                  ))}
              </>
            )}
          </section>
        
          {/* Column 2: Available Subscriptions */}
          <section className="flex flex-col gap-4">
            <div className="flex items-center gap-2 mb-2">
              <Search className="w-5 h-5 text-[#c5a059]" />
              <h3 className="text-lg font-bold uppercase tracking-tight">
                Available Subscriptions
              </h3>
            </div>
        
            {plans.length === 0 ? (
              <EmptyState message="No plans available" icon={Inbox} />
            ) : (
              <>
                {plans.map((plan) => (
                  <div
                    key={plan.publicKey.toBase58()}
                    className="bg-white dark:bg-[#c5a059]/5 border border-slate-200 dark:border-[#c5a059]/10 rounded p-4 flex flex-col gap-3 shadow-sm hover:border-[#c5a059]/40 transition-all cursor-pointer group"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className="size-10 rounded bg-[#c5a059]/10 flex items-center justify-center">
                          <Zap className="w-5 h-5 text-[#c5a059] group-hover:scale-110 transition-transform" />
                        </div>
                        <div>
                          <p className="font-bold text-sm uppercase tracking-wide">
                            Plan #{plan.planId}
                          </p>
                          <p className="text-xs text-slate-500">{plan.description}</p>
                        </div>
                      </div>
                      <button
                        onClick={() => handleSubscribe(plan)}
                        disabled={!!processing}
                        className="px-4 py-1.5 bg-[#c5a059] text-[#051d19] hover:bg-[#d4af37] rounded text-xs font-black transition-all disabled:opacity-50"
                      >
                        {processing === plan.publicKey.toBase58() ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          "JOIN"
                        )}
                      </button>
                    </div>
                    <div className="flex gap-6 pt-2 border-t border-white/5">
                      <div className="flex flex-col">
                        <span className="text-[9px] uppercase font-bold text-slate-500">
                          Price
                        </span>
                        <span className="text-sm font-bold">
                          {formatSol(plan.price)} SOL
                        </span>
                      </div>
                      <div className="flex flex-col">
                        <span className="text-[9px] uppercase font-bold text-slate-500">
                          Duration
                        </span>
                        <span className="text-sm font-bold text-[#c5a059]">
                          {formatDuration(plan.durationSeconds)}
                        </span>
                      </div>
                    </div>
                  </div>
                ))}
              </>
            )}
        
            <button className="w-full py-4 text-[10px] font-black uppercase tracking-[0.2em] text-slate-500 hover:text-[#c5a059] transition-colors flex items-center justify-center gap-2 mt-2">
              View All Infrastructure Plans
              <Search className="w-4 h-4" />
            </button>
          </section>
        
          {/* Column 3: Created Subscriptions */}
          <section className="flex flex-col gap-4 relative">
            <div className="flex items-center gap-2 mb-2">
              <Plus className="w-5 h-5 text-[#c5a059]" />
              <h3 className="text-lg font-bold uppercase tracking-tight">
                Created Subscriptions
              </h3>
            </div>
        
            <div className="flex-1 flex flex-col gap-4 min-h-[400px]">
              {plans.filter(
                (p) => wallet.publicKey && p.owner.equals(wallet.publicKey)
              ).length === 0 ? (
                <EmptyState
                  message={
                    wallet.connected
                      ? "No active offerings"
                      : "Connect wallet to create plans"
                  }
                  icon={Layers}
                />
              ) : (
                plans
                  .filter(
                    (p) => wallet.publicKey && p.owner.equals(wallet.publicKey)
                  )
                  .map((plan) => (
                    <div
                      key={plan.publicKey.toBase58()}
                      className="bg-white dark:bg-[#c5a059]/5 border border-slate-200 dark:border-[#c5a059]/10 rounded p-4 flex flex-col gap-3 shadow-sm"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <div className="size-10 rounded bg-[#c5a059]/10 flex items-center justify-center">
                            <Zap className="w-5 h-5 text-[#c5a059]" />
                          </div>
                          <div>
                            <p className="font-bold text-sm uppercase tracking-wide">
                              Plan #{plan.planId}
                            </p>
                            <p className="text-xs text-slate-500">{plan.description}</p>
                          </div>
                        </div>
                      </div>
                      <div className="flex gap-6 pt-2 border-t border-white/5">
                        <div className="flex flex-col">
                          <span className="text-[9px] uppercase font-bold text-slate-500">
                            Price
                          </span>
                          <span className="text-sm font-bold">
                            {formatSol(plan.price)} SOL
                          </span>
                        </div>
                        <div className="flex flex-col">
                          <span className="text-[9px] uppercase font-bold text-slate-500">
                            Duration
                          </span>
                          <span className="text-sm font-bold text-[#c5a059]">
                            {formatDuration(plan.durationSeconds)}
                          </span>
                        </div>
                      </div>
                    </div>
                  ))
              )}
            </div>
        
            <div className="sticky bottom-0 pt-4 bg-[#051d19]/80 backdrop-blur-md">
              <button
                onClick={() => setShowCreateModal(true)}
                disabled={!!processing}
                className="w-full flex items-center justify-center gap-2 py-4 bg-[#c5a059] text-[#051d19] hover:bg-[#d4af37] rounded font-black uppercase tracking-widest shadow-xl shadow-[#c5a059]/10 transition-transform active:scale-95 disabled:opacity-50"
              >
                {processing === "create-plan" ? (
                  <Loader2 className="w-5 h-5 animate-spin" />
                ) : (
                  <Plus className="w-5 h-5" />
                )}
                Create New Plan
              </button>
            </div>
          </section>
        </div>
        )}
      </main>

      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-[#051d19] border border-[#c5a059]/20 rounded-xl p-6 w-full max-w-md">
            <h3 className="text-xl font-bold text-[#c5a059] mb-4">
              Create Subscription Plan
            </h3>
            <form className="flex flex-col gap-4" onSubmit={handleCreatePlan}>
              <div>
                <label className="block text-sm font-bold text-slate-400 mb-1">
                  Plan ID
                </label>
                <input
                  name="planId"
                  type="text"
                  className="w-full px-4 py-2 bg-slate-800 border border-slate-700 rounded text-white"
                  placeholder="e.g., premium-001"
                  required
                />
              </div>
              <div>
                <label className="block text-sm font-bold text-slate-400 mb-1">
                  Price (SOL)
                </label>
                <input
                  name="price"
                  type="number"
                  step="0.1"
                  min="0.000000001"
                  className="w-full px-4 py-2 bg-slate-800 border border-slate-700 rounded text-white"
                  placeholder="e.g., 1.5"
                  required
                />
              </div>
              <div>
                <label className="block text-sm font-bold text-slate-400 mb-1">
                  Duration
                </label>
                <select
                  name="duration"
                  className="w-full px-4 py-2 bg-slate-800 border border-slate-700 rounded text-white"
                >
                  <option value="604800">Weekly</option>
                  <option value="2592000">Monthly</option>
                  <option value="31536000">Yearly</option>
                </select>
              </div>
              <div>
                <label className="block text-sm font-bold text-slate-400 mb-1">
                  Trial Days (optional)
                </label>
                <input
                  name="trialDays"
                  type="number"
                  min="0"
                  max="14"
                  defaultValue="0"
                  className="w-full px-4 py-2 bg-slate-800 border border-slate-700 rounded text-white"
                  placeholder="0"
                />
              </div>
              <div className="flex gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowCreateModal(false)}
                  className="flex-1 px-4 py-2 border border-slate-700 text-slate-400 rounded hover:border-white"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!!processing}
                  className="flex-1 px-4 py-2 bg-[#c5a059] text-[#051d19] rounded font-bold disabled:opacity-50"
                >
                  {processing === "create-plan" ? (
                    <Loader2 className="w-4 h-4 animate-spin mx-auto" />
                  ) : (
                    "Create"
                  )}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {showCancelModal && cancelSubscription && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-[#051d19] border border-[#c5a059]/20 rounded-xl p-6 w-full max-w-md">
            <h3 className="text-xl font-bold text-[#c5a059] mb-4">
              Cancel Subscription
            </h3>
            <p className="text-slate-400 mb-6">
              Choose how you want to cancel your subscription:
            </p>
            <div className="flex flex-col gap-3">
              <button
                onClick={() => {
                  handleCancel(cancelSubscription, true);
                  setShowCancelModal(false);
                  setCancelSubscription(null);
                }}
                disabled={!!processing}
                className="w-full px-4 py-3 border border-red-500/50 text-red-500 hover:bg-red-500/10 rounded font-bold disabled:opacity-50"
              >
                Cancel Now
                <span className="block text-xs font-normal text-slate-400 mt-1">
                  Immediately revokes access
                </span>
              </button>
              <button
                onClick={() => {
                  handleCancel(cancelSubscription, false);
                  setShowCancelModal(false);
                  setCancelSubscription(null);
                }}
                disabled={!!processing}
                className="w-full px-4 py-3 border border-orange-500/50 text-orange-500 hover:bg-orange-500/10 rounded font-bold disabled:opacity-50"
              >
                Cancel at Period End
                <span className="block text-xs font-normal text-slate-400 mt-1">
                  Access remains until current period ends
                </span>
              </button>
              <button
                onClick={() => {
                  setShowCancelModal(false);
                  setCancelSubscription(null);
                }}
                className="w-full px-4 py-3 border border-slate-700 text-slate-400 hover:border-white rounded"
              >
                Keep Subscription
              </button>
            </div>
          </div>
        </div>
      )}

      {toast && (
        <div
          className={`fixed bottom-6 right-6 px-6 py-3 rounded-xl font-medium shadow-lg z-50 transition-all duration-300 ${
            toast.type === "success"
              ? "bg-green-600 text-white border border-green-500"
              : "bg-red-600 text-white border border-red-500"
          }`}
        >
          {toast.message}
        </div>
      )}
    </div>
  );
}
