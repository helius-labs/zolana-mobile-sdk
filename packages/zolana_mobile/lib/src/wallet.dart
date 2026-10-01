import 'dart:async';
import 'dart:typed_data';

import 'rust/third_party/zolana_mobile.dart' as native;

/// Signs as one Solana account: a platform keystore, a wallet adapter or a
/// remote custodian. The Solana secret key never enters this package.
///
/// [signMessage] is asked for two things: once, the Zolana derivation message
/// that opens the wallet, and then each transaction message. [purpose] is a
/// description to show the user before signing.
abstract interface class SolanaSigner {
  /// Base58 public key.
  String get publicKey;

  /// Ed25519 signature over [message].
  Future<Uint8List> signMessage(Uint8List message, {required String purpose});
}

/// The application's backend prover, for spends proved with
/// `Proving.remote`. It receives [request], the `/prove` request body the
/// Zolana SDK's prover client sends, unchanged, and returns its prover's
/// proof: the gnark proof JSON, alone or as the `proof` of the prover's
/// response. When it throws, the spend fails with `remote_prover_failed`.
///
/// The wallet verifies the proof on the device against the pinned verifying
/// key, before it builds the message. The request carries the transaction's
/// witness, the wallet's nullifier secret included. The wallet, and
/// [ZolanaWallet.close], wait for it: give it a timeout, and do not call the
/// wallet from it.
typedef RemoteProver = Future<Uint8List> Function(Uint8List request);

/// A failure reported by the native wallet, such as
/// `recipient_not_registered` or `signature_invalid`, or a client error
/// describing what failed. It never contains key material.
class ZolanaWalletException implements Exception {
  const ZolanaWalletException(this.message);

  final String message;

  @override
  String toString() => 'Zolana wallet: $message';
}

/// The native calls that open a [ZolanaWallet]; replaced in tests.
abstract interface class WalletBackend {
  Future<Uint8List> derivationMessage(String solanaPubkey);
  Future<native.MobileWallet> open(
    native.WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
  );
  Future<native.MobileWallet> openWithKeys(
    native.WalletConfig config,
    String solanaPubkey,
    native.WalletKeys keys,
  );
}

/// A transaction the wallet built and, where needed, proved, awaiting
/// signatures. Each of [signers] signs [message] with Ed25519, in order.
class PreparedTransaction {
  PreparedTransaction._(
    this._pending,
    this.kind,
    this.summary,
    this.message,
    this.signers,
    this.lastValidBlockHeight,
  );

  static Future<PreparedTransaction> _read(
    native.PendingTransaction pending,
  ) async => PreparedTransaction._(
    pending,
    await pending.kind(),
    await pending.summary(),
    await pending.messageBytes(),
    await pending.signers(),
    await pending.lastValidBlockHeight(),
  );

  final native.PendingTransaction _pending;
  final native.PendingTransactionKind kind;

  /// Description to show the user before signing.
  final String summary;

  /// The serialized Solana message every signer signs.
  final Uint8List message;

  /// Base58 public keys that must sign, in signature order. The first is the
  /// fee payer, whose signature is the transaction signature.
  final List<String> signers;

  /// The last block height at which [message] can still land. Past it,
  /// [ZolanaWallet.refresh] gives it a new blockhash without a new proof.
  final BigInt lastValidBlockHeight;
}

/// A private wallet for SOL and SPL tokens that proves on the device.
///
/// Amounts are in base units: lamports for SOL, the mint's smallest unit for
/// a token. `mint` is the base58 mint address, or `null` for SOL. A token
/// works once the shielded pool has registered its mint; otherwise calls fail
/// with `asset_not_supported`.
///
/// The wallet keeps no chain state: [balances], [privateBalance], [activity]
/// and every spend read the wallet's notes from the indexer when they run.
/// Only the notes its prepared spends reserve are kept, in memory: the next
/// spend selects other notes until a prepared spend is submitted or
/// confirmed, [release]d, or past its
/// [PreparedTransaction.lastValidBlockHeight]. A spend that would need a
/// reserved note fails with `notes_reserved`.
///
/// Every operation runs after the previous one finishes: the native wallet
/// holds one proving key at a time. [close] it when the application locks or
/// switches accounts.
///
/// [register], [deposit], [transfer] and [withdraw] prepare, sign with the
/// wallet's [SolanaSigner] and submit. An application that signs and sends
/// transactions itself uses the `prepare` methods, then [submit] or, after
/// sending, [confirm].
class ZolanaWallet {
  ZolanaWallet._(
    this.solanaPublicKey,
    this._signer,
    this._wallet,
    this.shieldedAddress,
  );

  final SolanaSigner? _signer;
  final native.MobileWallet _wallet;
  Future<void> _last = Future.value();
  Future<void>? _closing;

  /// The address other Zolana wallets pay, once [register] has published it.
  final String shieldedAddress;

  /// Base58 public key of the Solana account this wallet belongs to.
  final String solanaPublicKey;

  bool get isClosed => _closing != null;

  /// Open the wallet [signer] controls. Asks the signer to sign the Zolana
  /// derivation message; the resulting keys stay in memory for this session.
  ///
  /// [remoteProver] proves the spends that ask for `Proving.remote`.
  static Future<ZolanaWallet> open({
    required native.WalletConfig config,
    required SolanaSigner signer,
    RemoteProver? remoteProver,
    WalletBackend backend = const _NativeBackend(),
  }) => _native(() async {
    final message = await backend.derivationMessage(signer.publicKey);
    final signature = await signer.signMessage(
      message,
      purpose: 'Open your private Zolana wallet',
    );
    final wallet = await backend.open(config, signer.publicKey, signature);
    await _setRemoteProver(wallet, remoteProver);
    return ZolanaWallet._(
      signer.publicKey,
      signer,
      wallet,
      await wallet.shieldedAddress(),
    );
  });

  /// Open the wallet of [solanaPublicKey] from [keys] that [exportKeys]
  /// returned, without asking for a signature. Fails with
  /// `wallet_keys_invalid` unless each private key yields its public key.
  ///
  /// Without a [signer], [register], [deposit], [transfer] and [withdraw] fail
  /// with `signer_missing`; the `prepare` methods, [submit] and [confirm]
  /// work. A [signer] for another account fails with `signer_mismatch`.
  /// [remoteProver] works as in [open].
  static Future<ZolanaWallet> openWithKeys({
    required native.WalletConfig config,
    required String solanaPublicKey,
    required native.WalletKeys keys,
    SolanaSigner? signer,
    RemoteProver? remoteProver,
    WalletBackend backend = const _NativeBackend(),
  }) => _native(() async {
    if (signer != null && signer.publicKey != solanaPublicKey) {
      throw const ZolanaWalletException('signer_mismatch');
    }
    final wallet = await backend.openWithKeys(config, solanaPublicKey, keys);
    await _setRemoteProver(wallet, remoteProver);
    return ZolanaWallet._(
      solanaPublicKey,
      signer,
      wallet,
      await wallet.shieldedAddress(),
    );
  });

  /// The keys [openWithKeys] opens this wallet from. They cannot move funds,
  /// but they show its balances and history and link its spends: keep them
  /// in the device's secure storage only, per account and network.
  Future<native.WalletKeys> exportKeys() => _serial(_wallet.exportKeys);

  /// Whether the user registry publishes [shieldedAddress]. A `conflict`
  /// means it holds other keys: payments to this account go to them, not to
  /// this wallet.
  Future<native.RegistrationStatus> registrationStatus() =>
      _serial(_wallet.registrationStatus);

  /// Spendable private balances, read from the indexer now: one per asset
  /// held in SOL, the configured `WalletConfig.mints` and the mints named so
  /// far.
  Future<List<native.TokenBalance>> balances() => _serial(_wallet.balances);

  /// Spendable private balance of [mint], read from the indexer now.
  Future<BigInt> privateBalance({String? mint}) =>
      _serial(() => _wallet.privateBalance(mint: mint));

  /// Transaction history, read from the indexer now, newest first: one entry
  /// per asset each transaction moved. A spend whose outputs are all this
  /// wallet's own is listed as unshielded, one with another wallet's output
  /// as sent; the indexer does not say which spends were withdrawals.
  Future<List<native.ActivityEntry>> activity() => _serial(_wallet.activity);

  /// Publish [shieldedAddress] so others can send to this wallet. `null` when
  /// it is already registered. Fails with `registration_conflict` when the
  /// registry holds other keys; the wallet never replaces them.
  Future<PreparedTransaction?> prepareRegistration() =>
      _serial(() => _prepareOptional(_wallet.prepareRegistration()));

  /// Move public funds from this account into the private balance. The
  /// deposit, its asset, its amount and this account are public.
  Future<PreparedTransaction> prepareDeposit(BigInt amount, {String? mint}) =>
      _serial(
        () => _prepare(_wallet.prepareDeposit(mint: mint, amount: amount)),
      );

  /// Send private funds to the registered wallet of [recipient] (a Solana
  /// public key). Reads the spendable notes, takes the largest on one tree,
  /// builds and proves on the device. Fails with `merge_required` when the
  /// amount needs more notes than one transaction spends, or the balance is
  /// spread over trees.
  ///
  /// [feePayer] (a Solana public key, such as the application's backend)
  /// pays the network fee instead of [solanaPublicKey]. The proof binds it,
  /// so it cannot change afterwards. [PreparedTransaction.signers] is then
  /// `[feePayer, solanaPublicKey]`.
  ///
  /// [proving] says where the spend is proved, `WalletConfig.proving` when
  /// null. `Proving.remote` asks the [RemoteProver] given at open; without
  /// one it fails with `remote_prover_missing`.
  Future<PreparedTransaction> prepareTransfer({
    required String recipient,
    required BigInt amount,
    String? mint,
    String? feePayer,
    native.Proving? proving,
  }) => _serial(
    () => _prepare(
      _wallet.prepareTransfer(
        recipient: recipient,
        mint: mint,
        amount: amount,
        feePayer: feePayer,
        proving: proving,
      ),
    ),
  );

  /// Move private funds to the public account [recipient]. The recipient,
  /// asset and amount are public. Tokens go to the recipient's associated
  /// token account; without one this fails with
  /// `recipient_token_account_missing` before proving. Create the account
  /// with the application's Solana client first. [feePayer] and [proving]
  /// work as in [prepareTransfer].
  Future<PreparedTransaction> prepareWithdrawal({
    required String recipient,
    required BigInt amount,
    String? mint,
    String? feePayer,
    native.Proving? proving,
  }) => _serial(
    () => _prepare(
      _wallet.prepareWithdrawal(
        recipient: recipient,
        mint: mint,
        amount: amount,
        feePayer: feePayer,
        proving: proving,
      ),
    ),
  );

  /// [transaction] with a new blockhash and the same proof, for an approval
  /// that outlived [PreparedTransaction.lastValidBlockHeight]. Sign the new
  /// [PreparedTransaction.message]: signatures over the old one do not apply.
  ///
  /// The proof stays valid while the trees still hold the roots it was built
  /// on: a state tree keeps its last 500 roots, about its last 500
  /// transactions. After that the program rejects the proof as stale; prepare
  /// the spend again.
  Future<PreparedTransaction> refresh(PreparedTransaction transaction) =>
      _serial(() => _prepare(_wallet.refresh(pending: transaction._pending)));

  /// Attach [signatures] (in [PreparedTransaction.signers] order), send, and
  /// wait as [confirm] does. Returns the transaction signature.
  Future<String> submit(
    PreparedTransaction transaction,
    List<Uint8List> signatures,
  ) => _serial(
    () => _wallet.submit(pending: transaction._pending, signatures: signatures),
  );

  /// Wait for a transaction the application sent itself, by its base58
  /// [signature]: until Solana confirms it and, for shielded-pool
  /// transactions, the indexer has it, so the next balance or spend reads the
  /// notes it created and not the ones it spent. Fails with
  /// `signature_invalid` unless [signature] is the fee payer's signature over
  /// [PreparedTransaction.message].
  Future<void> confirm(PreparedTransaction transaction, String signature) =>
      _serial(
        () => _wallet.confirm(
          pending: transaction._pending,
          signature: signature,
        ),
      );

  /// Release the notes [transaction] reserves, for a prepared spend that will
  /// not be sent, such as one the user declined, so the next spend can select
  /// them. [submit] and [confirm] release them, and [register], [deposit],
  /// [transfer] and [withdraw] release them when the signer fails.
  Future<void> release(PreparedTransaction transaction) =>
      _serial(() => _wallet.release(pending: transaction._pending));

  /// Wait for a shielded-pool transaction by its [signature] alone, such as
  /// one sent before the application restarted: until Solana confirms it and
  /// the indexer has it, so the next balance reads its notes. A transaction
  /// that failed on chain fails here with the chain's error.
  Future<void> waitForTransaction(String signature) =>
      _serial(() => _wallet.waitForTransaction(signature: signature));

  /// [prepareRegistration], signed and submitted. `null` when already
  /// registered.
  Future<String?> register() => _serial(() async {
    final transaction = await _prepareOptional(_wallet.prepareRegistration());
    return transaction == null ? null : _signAndSubmit(transaction);
  });

  /// [prepareDeposit], signed and submitted.
  Future<String> deposit(BigInt amount, {String? mint}) =>
      _serial(() => _send(_wallet.prepareDeposit(mint: mint, amount: amount)));

  /// [prepareTransfer], signed and submitted.
  Future<String> transfer({
    required String recipient,
    required BigInt amount,
    String? mint,
    native.Proving? proving,
  }) => _serial(
    () => _send(
      _wallet.prepareTransfer(
        recipient: recipient,
        mint: mint,
        amount: amount,
        proving: proving,
      ),
    ),
  );

  /// [prepareWithdrawal], signed and submitted.
  Future<String> withdraw({
    required String recipient,
    required BigInt amount,
    String? mint,
    native.Proving? proving,
  }) => _serial(
    () => _send(
      _wallet.prepareWithdrawal(
        recipient: recipient,
        mint: mint,
        amount: amount,
        proving: proving,
      ),
    ),
  );

  /// Stop this wallet. Operations not yet started fail with `wallet_closed`,
  /// and so does a `prepare` call that is proving when [close] is called: the
  /// application never receives a transaction after it locks. Nothing is
  /// signed or submitted after this call. Waits for the running native step
  /// (a proof cannot be interrupted), then releases the native wallet: its
  /// keys and proving key. Lock the UI without waiting for it; wait only
  /// before opening the next account.
  ///
  /// It does not recall a transaction already submitted.
  Future<void> close() => _closing ??= _last.then((_) => _wallet.dispose());

  /// The transaction [prepared] built, unless the wallet was closed while it
  /// was being built.
  Future<PreparedTransaction> _prepare(
    Future<native.PendingTransaction> prepared,
  ) async => _opened(await PreparedTransaction._read(await prepared));

  Future<PreparedTransaction?> _prepareOptional(
    Future<native.PendingTransaction?> prepared,
  ) async {
    final pending = await prepared;
    return _opened(
      pending == null ? null : await PreparedTransaction._read(pending),
    );
  }

  T _opened<T>(T value) {
    _ensureOpen();
    return value;
  }

  Future<String> _send(Future<native.PendingTransaction> prepared) async =>
      _signAndSubmit(await _prepare(prepared));

  Future<String> _signAndSubmit(PreparedTransaction transaction) async {
    final Uint8List signature;
    try {
      signature = await _sign(transaction);
    } catch (_) {
      // Never sent: its notes are free for the next spend.
      if (!isClosed) await _wallet.release(pending: transaction._pending);
      rethrow;
    }
    _ensureOpen();
    return _wallet.submit(
      pending: transaction._pending,
      signatures: [signature],
    );
  }

  Future<Uint8List> _sign(PreparedTransaction transaction) async {
    final signer = _signer;
    if (signer == null) {
      throw const ZolanaWalletException('signer_missing');
    }
    final signers = transaction.signers;
    if (signers.length != 1 || signers.single != solanaPublicKey) {
      throw const ZolanaWalletException('unexpected_signers');
    }
    return signer.signMessage(
      transaction.message,
      purpose: transaction.summary,
    );
  }

  void _ensureOpen() {
    if (isClosed) throw const ZolanaWalletException('wallet_closed');
  }

  Future<T> _serial<T>(Future<T> Function() operation) {
    final result = _last.then((_) {
      _ensureOpen();
      return _native(operation);
    });
    _last = result.then((_) {}, onError: (_) {});
    return result;
  }
}

/// The native wallet takes a failed proof as `null`.
Future<void> _setRemoteProver(
  native.MobileWallet wallet,
  RemoteProver? prove,
) async {
  if (prove == null) return;
  await wallet.setRemoteProver(
    prove: (request) async {
      try {
        return await prove(request);
      } catch (_) {
        return null;
      }
    },
  );
}

Future<T> _native<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on String catch (message) {
    throw ZolanaWalletException(message);
  }
}

class _NativeBackend implements WalletBackend {
  const _NativeBackend();

  @override
  Future<Uint8List> derivationMessage(String solanaPubkey) =>
      native.derivationMessage(solanaPubkey: solanaPubkey);

  @override
  Future<native.MobileWallet> open(
    native.WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
  ) => native.MobileWallet.open(
    config: config,
    solanaPubkey: solanaPubkey,
    derivationSignature: derivationSignature,
  );

  @override
  Future<native.MobileWallet> openWithKeys(
    native.WalletConfig config,
    String solanaPubkey,
    native.WalletKeys keys,
  ) => native.MobileWallet.openWithKeys(
    config: config,
    solanaPubkey: solanaPubkey,
    keys: keys,
  );
}
