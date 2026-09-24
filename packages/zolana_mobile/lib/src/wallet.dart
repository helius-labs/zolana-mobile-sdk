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

/// A failure reported by the native wallet, such as
/// `recipient_not_registered` or `signature_invalid`, or a client error
/// describing what failed. It never contains key material.
class ZolanaWalletException implements Exception {
  const ZolanaWalletException(this.message);

  final String message;

  @override
  String toString() => 'Zolana wallet: $message';
}

/// The native operations [ZolanaWallet] drives; replaced in tests.
abstract interface class WalletBackend {
  Future<Uint8List> derivationMessage(String solanaPubkey);
  Future<NativeWallet> open(
    native.WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
  );
}

/// One opened native wallet.
abstract interface class NativeWallet {
  Future<String> shieldedAddress();
  Future<native.RegistrationStatus> registrationStatus();
  Future<native.SyncSummary> sync();
  Future<List<native.TokenBalance>> balances();
  Future<BigInt> privateBalance(String? mint);
  Future<BigInt> publicBalance(String? mint);
  Future<List<native.ActivityEntry>> activity();
  Future<NativePending?> prepareRegistration();
  Future<NativePending> prepareDeposit(String? mint, BigInt amount);
  Future<NativePending> prepareTransfer(
    String recipient,
    String? mint,
    BigInt amount,
  );
  Future<NativePending> prepareWithdrawal(
    String recipient,
    String? mint,
    BigInt amount,
  );
  Future<NativePending?> prepareTokenAccount(String owner, String mint);
  Future<String> submit(NativePending pending, List<Uint8List> signatures);
  Future<void> confirm(NativePending pending, String signature);

  /// Release the native wallet. Called once, after its last operation.
  void dispose();
}

/// A native transaction awaiting signatures.
abstract interface class NativePending {
  Future<native.PendingTransactionKind> kind();
  Future<String> summary();
  Future<Uint8List> messageBytes();
  Future<List<String>> signers();
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
  );

  static Future<PreparedTransaction> _read(NativePending pending) async =>
      PreparedTransaction._(
        pending,
        await pending.kind(),
        await pending.summary(),
        await pending.messageBytes(),
        await pending.signers(),
      );

  final NativePending _pending;
  final native.PendingTransactionKind kind;

  /// Description to show the user before signing.
  final String summary;

  /// The serialized Solana message every signer signs.
  final Uint8List message;

  /// Base58 public keys that must sign, in signature order. The first is the
  /// fee payer, whose signature is the transaction signature.
  final List<String> signers;
}

/// A private wallet for SOL and SPL tokens that proves on the device.
///
/// Amounts are in base units: lamports for SOL, the mint's smallest unit for
/// a token. `mint` is the base58 mint address, or `null` for SOL. A token
/// works once the shielded pool has registered its mint; otherwise calls fail
/// with `asset_not_supported`.
///
/// Every operation runs after the previous one finishes: the native wallet
/// holds one proving key at a time and its state is updated by [sync].
/// [close] it when the application locks or switches accounts.
///
/// [register], [deposit], [transfer] and [withdraw] prepare, sign with the
/// wallet's [SolanaSigner] and submit. An application that signs and sends
/// transactions itself uses the `prepare` methods, then [submit] or, after
/// sending, [confirm].
class ZolanaWallet {
  ZolanaWallet._(this._signer, this._wallet, this.shieldedAddress);

  final SolanaSigner _signer;
  final NativeWallet _wallet;
  Future<void> _last = Future.value();
  Future<void>? _closing;

  /// The address other Zolana wallets pay, once [register] has published it.
  final String shieldedAddress;

  String get solanaPublicKey => _signer.publicKey;

  bool get isClosed => _closing != null;

  /// Open the wallet [signer] controls. Asks the signer to sign the Zolana
  /// derivation message; the resulting keys stay in memory for this session.
  static Future<ZolanaWallet> open({
    required native.WalletConfig config,
    required SolanaSigner signer,
    WalletBackend backend = const _NativeBackend(),
  }) => _native(() async {
    final message = await backend.derivationMessage(signer.publicKey);
    final signature = await signer.signMessage(
      message,
      purpose: 'Open your private Zolana wallet',
    );
    final wallet = await backend.open(config, signer.publicKey, signature);
    return ZolanaWallet._(signer, wallet, await wallet.shieldedAddress());
  });

  /// Whether the user registry publishes [shieldedAddress]. A `conflict`
  /// means it holds other keys: payments to this account go to them, not to
  /// this wallet.
  Future<native.RegistrationStatus> registrationStatus() =>
      _serial(_wallet.registrationStatus);

  /// Fetch and decrypt this wallet's notes.
  Future<native.SyncSummary> sync() => _serial(_wallet.sync);

  /// Spendable private balances as of the last [sync], one per asset held.
  Future<List<native.TokenBalance>> balances() => _serial(_wallet.balances);

  /// Spendable private balance of [mint] as of the last [sync].
  Future<BigInt> privateBalance({String? mint}) =>
      _serial(() => _wallet.privateBalance(mint));

  /// Public balance of [solanaPublicKey], read from the RPC now: lamports, or
  /// the amount in its associated token account for [mint] (0 without one).
  Future<BigInt> publicBalance({String? mint}) =>
      _serial(() => _wallet.publicBalance(mint));

  /// History found by the last [sync], newest first.
  Future<List<native.ActivityEntry>> activity() => _serial(_wallet.activity);

  /// Publish [shieldedAddress] so others can send to this wallet. `null` when
  /// it is already registered. Fails with `registration_conflict` when the
  /// registry holds other keys; the wallet never replaces them.
  Future<PreparedTransaction?> prepareRegistration() => _serial(() async {
    final pending = await _wallet.prepareRegistration();
    return pending == null ? null : PreparedTransaction._read(pending);
  });

  /// Move public funds from this account into the private balance. The
  /// deposit, its asset, its amount and this account are public.
  Future<PreparedTransaction> prepareDeposit(BigInt amount, {String? mint}) =>
      _serial(() => _prepareDeposit(mint, amount));

  /// Send private funds to the registered wallet of [recipient] (a Solana
  /// public key). Syncs, selects notes, builds and proves on the device.
  Future<PreparedTransaction> prepareTransfer({
    required String recipient,
    required BigInt amount,
    String? mint,
  }) => _serial(() => _prepareTransfer(recipient, mint, amount));

  /// Move private funds to the public account [recipient]. The recipient,
  /// asset and amount are public. Tokens go to the recipient's associated
  /// token account; without one this fails with
  /// `recipient_token_account_missing` (see [prepareTokenAccount]).
  Future<PreparedTransaction> prepareWithdrawal({
    required String recipient,
    required BigInt amount,
    String? mint,
  }) => _serial(() => _prepareWithdrawal(recipient, mint, amount));

  /// Create [owner]'s associated token account for [mint], paid by this
  /// account, so a withdrawal can reach it. `null` when it already exists.
  Future<PreparedTransaction?> prepareTokenAccount({
    required String owner,
    required String mint,
  }) => _serial(() async {
    final pending = await _wallet.prepareTokenAccount(owner, mint);
    return pending == null ? null : PreparedTransaction._read(pending);
  });

  /// Attach [signatures] (in [PreparedTransaction.signers] order), send, and
  /// wait as [confirm] does. Returns the transaction signature.
  Future<String> submit(
    PreparedTransaction transaction,
    List<Uint8List> signatures,
  ) => _serial(() => _wallet.submit(transaction._pending, signatures));

  /// Wait for a transaction the application sent itself, by its base58
  /// [signature]: until Solana confirms it and, for shielded-pool
  /// transactions, the indexer has it. Then syncs, so the notes it spent are
  /// no longer offered. Fails with `signature_invalid` unless [signature] is
  /// the fee payer's signature over [PreparedTransaction.message].
  Future<void> confirm(PreparedTransaction transaction, String signature) =>
      _serial(() => _wallet.confirm(transaction._pending, signature));

  /// [prepareRegistration], signed and submitted. `null` when already
  /// registered.
  Future<String?> register() => _serial(() async {
    final pending = await _wallet.prepareRegistration();
    return pending == null
        ? null
        : _signAndSubmit(await PreparedTransaction._read(pending));
  });

  /// [prepareDeposit], signed and submitted.
  Future<String> deposit(BigInt amount, {String? mint}) =>
      _serial(() async => _signAndSubmit(await _prepareDeposit(mint, amount)));

  /// [prepareTransfer], signed and submitted.
  Future<String> transfer({
    required String recipient,
    required BigInt amount,
    String? mint,
  }) => _serial(
    () async =>
        _signAndSubmit(await _prepareTransfer(recipient, mint, amount)),
  );

  /// [prepareWithdrawal], signed and submitted.
  Future<String> withdraw({
    required String recipient,
    required BigInt amount,
    String? mint,
  }) => _serial(
    () async =>
        _signAndSubmit(await _prepareWithdrawal(recipient, mint, amount)),
  );

  /// Stop this wallet. Operations not yet started fail with `wallet_closed`,
  /// and nothing is signed or submitted after this call. Waits for the
  /// running native step (a proof cannot be interrupted), then releases the
  /// native wallet: its keys, notes and proving key.
  ///
  /// It does not recall a transaction already submitted.
  Future<void> close() =>
      _closing ??= _last.then((_) => _wallet.dispose());

  Future<PreparedTransaction> _prepareDeposit(
    String? mint,
    BigInt amount,
  ) async => PreparedTransaction._read(
    await _wallet.prepareDeposit(mint, amount),
  );

  Future<PreparedTransaction> _prepareTransfer(
    String recipient,
    String? mint,
    BigInt amount,
  ) async => PreparedTransaction._read(
    await _wallet.prepareTransfer(recipient, mint, amount),
  );

  Future<PreparedTransaction> _prepareWithdrawal(
    String recipient,
    String? mint,
    BigInt amount,
  ) async => PreparedTransaction._read(
    await _wallet.prepareWithdrawal(recipient, mint, amount),
  );

  Future<String> _signAndSubmit(PreparedTransaction transaction) async {
    final signers = transaction.signers;
    if (signers.length != 1 || signers.single != _signer.publicKey) {
      throw const ZolanaWalletException('unexpected_signers');
    }
    _ensureOpen();
    final signature = await _signer.signMessage(
      transaction.message,
      purpose: transaction.summary,
    );
    _ensureOpen();
    return _wallet.submit(transaction._pending, [signature]);
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
  Future<NativeWallet> open(
    native.WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
  ) async => _NativeWallet(
    await native.MobileWallet.open(
      config: config,
      solanaPubkey: solanaPubkey,
      derivationSignature: derivationSignature,
    ),
  );
}

class _NativeWallet implements NativeWallet {
  _NativeWallet(this._wallet);

  final native.MobileWallet _wallet;

  @override
  Future<String> shieldedAddress() => _wallet.shieldedAddress();

  @override
  Future<native.RegistrationStatus> registrationStatus() =>
      _wallet.registrationStatus();

  @override
  Future<native.SyncSummary> sync() => _wallet.sync_();

  @override
  Future<List<native.TokenBalance>> balances() => _wallet.balances();

  @override
  Future<BigInt> privateBalance(String? mint) =>
      _wallet.privateBalance(mint: mint);

  @override
  Future<BigInt> publicBalance(String? mint) =>
      _wallet.publicBalance(mint: mint);

  @override
  Future<List<native.ActivityEntry>> activity() => _wallet.activity();

  @override
  Future<NativePending?> prepareRegistration() async {
    final pending = await _wallet.prepareRegistration();
    return pending == null ? null : _NativePending(pending);
  }

  @override
  Future<NativePending> prepareDeposit(String? mint, BigInt amount) async =>
      _NativePending(await _wallet.prepareDeposit(mint: mint, amount: amount));

  @override
  Future<NativePending> prepareTransfer(
    String recipient,
    String? mint,
    BigInt amount,
  ) async => _NativePending(
    await _wallet.prepareTransfer(
      recipient: recipient,
      mint: mint,
      amount: amount,
    ),
  );

  @override
  Future<NativePending> prepareWithdrawal(
    String recipient,
    String? mint,
    BigInt amount,
  ) async => _NativePending(
    await _wallet.prepareWithdrawal(
      recipient: recipient,
      mint: mint,
      amount: amount,
    ),
  );

  @override
  Future<NativePending?> prepareTokenAccount(String owner, String mint) async {
    final pending = await _wallet.prepareTokenAccount(owner: owner, mint: mint);
    return pending == null ? null : _NativePending(pending);
  }

  @override
  Future<String> submit(NativePending pending, List<Uint8List> signatures) =>
      _wallet.submit(
        pending: (pending as _NativePending)._pending,
        signatures: signatures,
      );

  @override
  Future<void> confirm(NativePending pending, String signature) =>
      _wallet.confirm(
        pending: (pending as _NativePending)._pending,
        signature: signature,
      );

  @override
  void dispose() => _wallet.dispose();
}

class _NativePending implements NativePending {
  _NativePending(this._pending);

  final native.PendingTransaction _pending;

  @override
  Future<native.PendingTransactionKind> kind() => _pending.kind();

  @override
  Future<String> summary() => _pending.summary();

  @override
  Future<Uint8List> messageBytes() => _pending.messageBytes();

  @override
  Future<List<String>> signers() => _pending.signers();
}
