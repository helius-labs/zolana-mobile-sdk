import 'dart:async';
import 'dart:typed_data';

import 'http_transport.dart';
import 'rust/third_party/zolana_mobile.dart' as native;
import 'rust/third_party/zolana_mobile.dart' show WalletError;

/// Signs as one Solana account: a platform keystore, a wallet adapter or a
/// remote custodian. The Solana secret key never enters this package.
///
/// [signMessage] is asked for two things: once, the Zolana derivation message
/// that opens the wallet, with no [transaction], and then each prepared
/// transaction's message, with the [transaction] to show the user before
/// signing.
abstract interface class SolanaSigner {
  /// Base58 public key.
  String get publicKey;

  /// Ed25519 signature over [message].
  Future<Uint8List> signMessage(
    Uint8List message, {
    PreparedTransaction? transaction,
  });
}

/// Sends one HTTP request of the wallet. The wallet never opens a connection:
/// Solana RPC and indexer calls (`POST`, JSON), proving-key downloads (`GET`,
/// files of several MB) and remote proofs (`POST` to `WalletConfig.proverUrl`,
/// then `GET` of a queued proof's status) all go through its transport, the
/// package's own on `package:http` or the one the application passes to
/// [ZolanaWallet.open]. The request carries the method, the URL with its
/// `api-key`, the content type of a `POST` and `x-sync` or `x-async` on a
/// proof request, nothing else; send it as it is, following redirects (the
/// key host may redirect). A proof request's body is the transaction's
/// witness, the wallet's nullifier secret included: do not log it.
///
/// Return the server's response whatever its status, with its headers (a
/// prover inside a TEE marks its encrypted body in them), and throw only when
/// there is none; the wallet then fails with the exception's message, never
/// its stack trace. Throw a [TransportResponseLost] when the status arrived
/// and the body could not be read: the wallet then does not send the request
/// again, so a prover does not prove the spend twice. Stop reading and throw
/// when a body exceeds [native.TransportRequest.maxResponseBytes]; the wallet
/// refuses a longer body either way. Bound each request by
/// [native.TransportRequest.timeoutMs] when it is set (600 seconds for a
/// proof request), and by a timeout of your own otherwise: the wallet, and
/// [ZolanaWallet.close], wait for each answer, so the transport must not call
/// the wallet either.
typedef ZolanaTransport = Future<native.TransportResponse> Function(
  native.TransportRequest request,
);

/// A [ZolanaTransport] as the native wallet calls it: a failure is a message,
/// never a throw.
typedef NativeTransport = Future<native.TransportOutcome> Function(
  native.TransportRequest request,
);

/// A failure the wallet reports. [error] says what happened, with its data:
/// the amounts of a short balance, the account a conflict is about. It never
/// contains key material.
class ZolanaWalletException implements Exception {
  const ZolanaWalletException(this.error);

  final WalletError error;

  /// [error] as text, for logs.
  String get message => '$error';

  @override
  String toString() => 'Zolana wallet: $error';
}

/// The native calls that open a [ZolanaWallet]; replaced in tests.
abstract interface class WalletBackend {
  Future<Uint8List> derivationMessage(String solanaPubkey);
  Future<native.MobileWallet> open(
    native.WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
    NativeTransport transport,
  );
  Future<native.MobileWallet> openWithKeys(
    native.WalletConfig config,
    String solanaPubkey,
    native.WalletKeys keys,
    NativeTransport transport,
  );
}

/// A transaction the wallet built and, where needed, proved, awaiting
/// signatures. Each of [signers] signs [message] with Ed25519, in order.
/// [kind], [amount], [mint], [recipient] and [feePayer] are what to show the
/// user before signing.
class PreparedTransaction {
  PreparedTransaction._(
    this._pending,
    this.kind,
    this.amount,
    this.mint,
    this.recipient,
    this.feePayer,
    this.message,
    this.signers,
    this.lastValidBlockHeight,
  );

  static Future<PreparedTransaction> _read(
    native.PendingTransaction pending,
  ) async => PreparedTransaction._(
    pending,
    await pending.kind(),
    await pending.amount(),
    await pending.mint(),
    await pending.recipient(),
    await pending.feePayer(),
    await pending.messageBytes(),
    await pending.signers(),
    await pending.lastValidBlockHeight(),
  );

  final native.PendingTransaction _pending;
  final native.PendingTransactionKind kind;

  /// Base units a deposit, transfer or withdrawal moves.
  final BigInt? amount;

  /// The mint of the asset moved, or of the token account created; `null`
  /// for SOL.
  final String? mint;

  /// The account a transfer or withdrawal pays, or whose token account is
  /// created.
  final String? recipient;

  /// Base58 account that pays the network fee: the first of [signers].
  final String feePayer;

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
/// works when `WalletConfig.mints` lists its mint, with its token program, and
/// the shielded pool has registered it; otherwise calls fail with
/// [WalletError.mintNotConfigured] or [WalletError.assetNotSupported].
///
/// The wallet keeps no chain state: [balances], [privateBalance], [activity]
/// and every spend read the wallet's notes from the indexer when they run.
/// Only the notes its prepared spends reserve are kept, in memory: the next
/// spend selects other notes until a prepared spend is submitted or
/// confirmed, [release]d, or past its
/// [PreparedTransaction.lastValidBlockHeight]. A spend that would need a
/// reserved note fails with [WalletError.notesReserved].
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
    this._keys,
    this.shieldedAddress,
    this._defaultTransport,
  );

  final SolanaSigner? _signer;
  final native.MobileWallet _wallet;
  final native.ProvingKeys _keys;

  /// The package's transport, when the application passed none; closed with
  /// the wallet.
  final HttpTransport? _defaultTransport;
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
  /// Every request of the wallet goes through [transport], the application's
  /// own networking, which applies its own URL policy. Without one, the
  /// package sends with `package:http`: one connection pool per wallet,
  /// closed with it. It refuses a plaintext URL off loopback in [config]
  /// before anything else, with [WalletError.rpcUrlInsecure],
  /// [WalletError.indexerUrlInsecure], [WalletError.provingKeyUrlInsecure] or
  /// [WalletError.proverUrlInsecure], unless [allowInsecureHttp] is set: the
  /// indexer sees the wallet's view tags and a prover its witnesses, so set
  /// it only for a test cluster. The wallet opens no connection itself either
  /// way.
  static Future<ZolanaWallet> open({
    required native.WalletConfig config,
    required SolanaSigner signer,
    ZolanaTransport? transport,
    bool allowInsecureHttp = false,
    WalletBackend backend = const _NativeBackend(),
  }) => _native(() async {
    if (transport == null) _refusePlaintext(config, allowInsecureHttp);
    final message = await backend.derivationMessage(signer.publicKey);
    final signature = await signer.signMessage(message);
    return _open(
      signer.publicKey,
      signer,
      transport,
      (transport) =>
          backend.open(config, signer.publicKey, signature, transport),
    );
  });

  /// Open the wallet of [solanaPublicKey] from [keys] that [exportKeys]
  /// returned, without asking for a signature. Fails with
  /// [WalletError.invalidWalletKeys] unless each private key yields its
  /// public key.
  ///
  /// Without a [signer], [register], [deposit], [transfer] and [withdraw] fail
  /// with [WalletError.signerMissing]; the `prepare` methods, [submit] and
  /// [confirm] work. A [signer] for another account fails with
  /// [WalletError.signerMismatch].
  /// [transport] and [allowInsecureHttp] work as in [open].
  static Future<ZolanaWallet> openWithKeys({
    required native.WalletConfig config,
    required String solanaPublicKey,
    required native.WalletKeys keys,
    SolanaSigner? signer,
    ZolanaTransport? transport,
    bool allowInsecureHttp = false,
    WalletBackend backend = const _NativeBackend(),
  }) => _native(() async {
    if (signer != null && signer.publicKey != solanaPublicKey) {
      throw ZolanaWalletException(
        WalletError.signerMismatch(
          wallet: solanaPublicKey,
          signer: signer.publicKey,
        ),
      );
    }
    if (transport == null) _refusePlaintext(config, allowInsecureHttp);
    return _open(
      solanaPublicKey,
      signer,
      transport,
      (transport) =>
          backend.openWithKeys(config, solanaPublicKey, keys, transport),
    );
  });

  /// Opens the native wallet with [transport], or with the package's own.
  /// A failure after the native wallet opened releases it.
  static Future<ZolanaWallet> _open(
    String solanaPublicKey,
    SolanaSigner? signer,
    ZolanaTransport? transport,
    Future<native.MobileWallet> Function(NativeTransport transport) open,
  ) async {
    final http = transport == null ? HttpTransport() : null;
    native.MobileWallet? wallet;
    try {
      wallet = await open(_nativeTransport(transport ?? http!.send));
      return ZolanaWallet._(
        solanaPublicKey,
        signer,
        wallet,
        await wallet.provingKeys(),
        await wallet.shieldedAddress(),
        http,
      );
    } catch (_) {
      wallet?.dispose();
      http?.close();
      rethrow;
    }
  }

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
  /// held in SOL and the mints of `WalletConfig.mints`.
  Future<List<native.TokenBalance>> balances() => _serial(_wallet.balances);

  /// Spendable private balance of [mint], read from the indexer now.
  Future<BigInt> privateBalance({String? mint}) =>
      _serial(() => _wallet.privateBalance(mint: mint));

  /// Transaction history, read from the indexer now, newest first: one entry
  /// per asset each transaction moved. A spend whose outputs are all this
  /// wallet's own is listed as a withdrawal, one with another wallet's output
  /// as sent; the indexer does not say which spends were withdrawals.
  Future<List<native.ActivityEntry>> activity() => _serial(_wallet.activity);

  /// Publish [shieldedAddress] so others can send to this wallet. `null` when
  /// it is already registered. Fails with [WalletError.registrationConflict]
  /// when the registry holds other keys; the wallet never replaces them.
  Future<PreparedTransaction?> prepareRegistration() =>
      _serial(() => _prepareOptional(_wallet.prepareRegistration()));

  /// Whether this account's user record lets merges of its notes run;
  /// `false` before registration.
  Future<bool> mergingEnabled() => _serial(_wallet.mergingEnabled);

  /// Turn merges of this account's notes on or off; this account signs.
  /// `null` when the record already says so. While merging is on, a merge
  /// proved with this wallet's nullifier secret needs no signature of this
  /// account: whoever holds the secret and the notes, such as the prover at
  /// `WalletConfig.proverUrl`, can merge them. A merge cannot move funds.
  Future<PreparedTransaction?> prepareMerging(bool enabled) =>
      _serial(() => _prepareOptional(_wallet.prepareMerging(enabled: enabled)));

  /// Combine the smallest notes of [mint] (SOL when null) on one tree into
  /// one: at most [maxInputs], 24 when null, 54 at most. A spend takes at most
  /// 40 notes, so merge when it fails with [WalletError.mergeRequired] or
  /// [WalletError.tooManyInputTrees], then prepare it again. Notes that
  /// prepared spends reserve are left out, and the merge reserves its own.
  /// Fails with [WalletError.nothingToMerge] below two notes, and with
  /// [WalletError.mergingDisabled] until [prepareMerging] turned merging on.
  ///
  /// A merge needs no signature of this account: [feePayer] (this account
  /// when null) pays and signs alone. [proving] works as in
  /// [prepareTransfer]; a remote prover receives this wallet's nullifier
  /// secret and learns every merged amount. The proof expires ten minutes
  /// after this call, and [refresh] does not extend it.
  Future<PreparedTransaction> prepareMerge({
    String? mint,
    int? maxInputs,
    String? feePayer,
    native.Proving? proving,
  }) => _serial(
    () => _prepare(
      _wallet.prepareMerge(
        mint: mint,
        maxInputs: maxInputs,
        feePayer: feePayer,
        proving: proving,
      ),
    ),
  );

  /// Move public funds from this account into the private balance. The
  /// deposit, its asset, its amount and this account are public.
  Future<PreparedTransaction> prepareDeposit(BigInt amount, {String? mint}) =>
      _serial(
        () => _prepare(_wallet.prepareDeposit(mint: mint, amount: amount)),
      );

  /// Send private funds to the registered wallet of [recipient] (a Solana
  /// public key). Reads the spendable notes, takes the largest first, builds
  /// and proves on the device. Fails with [WalletError.mergeRequired] when
  /// the amount needs more notes than one transaction spends (40), and with
  /// [WalletError.tooManyInputTrees] when they are on more than two trees.
  ///
  /// [feePayer] (a Solana public key, such as the application's backend)
  /// pays the network fee instead of [solanaPublicKey]. The proof binds it,
  /// so it cannot change afterwards. [PreparedTransaction.signers] is then
  /// `[feePayer, solanaPublicKey]`.
  ///
  /// [proving] says where the spend is proved, `WalletConfig.proving` when
  /// null. `Proving.remote` asks the prover at `WalletConfig.proverUrl`
  /// through the transport; without one it fails with
  /// [WalletError.remoteProverMissing]. The wallet verifies a remote proof
  /// against the pinned verifying key before it builds the message: a
  /// response that is not a proof of the pinned key fails with
  /// [WalletError.proofMalformed], a proof that does not verify with
  /// [WalletError.proofInvalid].
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
  /// [WalletError.recipientTokenAccountMissing] before proving. Create the
  /// account with the application's Solana client first. [feePayer] and
  /// [proving] work as in [prepareTransfer].
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
  /// wait as [confirm] does. Returns the transaction signature. A send that
  /// fails without the chain's verdict, such as a lost confirmation, is
  /// checked against the signature's status until the blockhash expires. A
  /// note the chain already spent fails with [WalletError.notesAlreadySpent].
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
  /// [WalletError.signatureInvalid] unless [signature] is the fee payer's
  /// signature over [PreparedTransaction.message].
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

  /// [prepareMerging], signed and submitted. `null` when the record already
  /// says so.
  Future<String?> setMerging(bool enabled) => _serial(() async {
    final transaction = await _prepareOptional(
      _wallet.prepareMerging(enabled: enabled),
    );
    return transaction == null ? null : _signAndSubmit(transaction);
  });

  /// [prepareMerge], signed and submitted by this account.
  Future<String> merge({
    String? mint,
    int? maxInputs,
    native.Proving? proving,
  }) => _serial(
    () => _send(
      _wallet.prepareMerge(mint: mint, maxInputs: maxInputs, proving: proving),
    ),
  );

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

  /// Stop this wallet. Operations not yet started fail with
  /// [WalletError.walletClosed], and so does the running one, whether it is
  /// proving or waiting for the network: the application never receives a
  /// transaction after it locks. Nothing is signed or submitted after this
  /// call. The package's own transport aborts its requests at once; an
  /// application's transport should abort its own when the application locks.
  /// Waits for the running native step (a proof cannot be interrupted), then
  /// releases the native wallet: its keys and proving key. Lock the UI
  /// without waiting for it; wait only before opening the next account.
  ///
  /// It does not recall a transaction already submitted.
  Future<void> close() => _closing ??= () {
    // Requests in flight fail now instead of holding up the lock.
    _defaultTransport?.close();
    return _last.then((_) {
      _wallet.dispose();
      _keys.dispose();
    });
  }();

  /// The proving keys this wallet proves with on the device, smallest
  /// first, with how much of each is downloaded. Reads only file sizes.
  Future<List<native.ProvingKeyStatus>> provingKeys() {
    _ensureOpen();
    return _native(_keys.status);
  }

  /// Download ahead of time the proving keys of spends of up to [maxInputs]
  /// notes with [outputs] outputs, and of merges of up to [maxMergeInputs]
  /// notes, so the first proof of each shape does not wait for its key. A
  /// transfer or withdrawal has two outputs.
  ///
  /// The download starts when the stream is listened to and reports each
  /// part of a few MB; keys already on the device are skipped. Cancelling the
  /// subscription stops it after the part in flight, and so does [close]:
  /// what was downloaded stays, and the next download of that key resumes
  /// from it. It runs beside the wallet's other calls, and a proof that needs
  /// a key it is downloading waits for it. A failure, such as
  /// [WalletError.provingKeyDownloadFailed], ends the stream with a
  /// [ZolanaWalletException].
  Stream<native.ProvingKeyProgress> prefetchProvingKeys({
    required int maxInputs,
    List<int> outputs = const [2],
    int maxMergeInputs = 0,
  }) {
    var stopped = false;
    late final StreamController<native.ProvingKeyProgress> updates;
    updates = StreamController(
      onListen: () async {
        try {
          _ensureOpen();
          final names = await _native(
            () => _keys.needed(
              maxInputs: maxInputs,
              outputs: outputs,
              maxMergeInputs: maxMergeInputs,
            ),
          );
          final done = await _native(
            () => _keys.prefetch(
              names: names,
              progress: (update) {
                if (stopped || isClosed) return false;
                updates.add(update);
                return true;
              },
            ),
          );
          if (!done && !stopped) _ensureOpen();
        } catch (error, stack) {
          if (!stopped) updates.addError(error, stack);
        }
        await updates.close();
      },
      onCancel: () => stopped = true,
    );
    return updates.stream;
  }

  /// Remove the downloaded proving keys and partial downloads from the
  /// device. A prefetch or proof downloading a key finishes its part first;
  /// the next proof downloads its key again.
  Future<void> clearProvingKeys() {
    _ensureOpen();
    return _native(_keys.clear);
  }

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
      throw const ZolanaWalletException(WalletError.signerMissing());
    }
    final signers = transaction.signers;
    if (signers.length != 1 || signers.single != solanaPublicKey) {
      throw ZolanaWalletException(
        WalletError.unexpectedSigners(signers: signers),
      );
    }
    return signer.signMessage(transaction.message, transaction: transaction);
  }

  void _ensureOpen() {
    if (isClosed) {
      throw const ZolanaWalletException(WalletError.walletClosed());
    }
  }

  Future<T> _serial<T>(Future<T> Function() operation) {
    final result = _last.then((_) async {
      _ensureOpen();
      try {
        return await _native(operation);
      } catch (_) {
        // A step that failed because close() aborted it reports the close.
        _ensureOpen();
        rethrow;
      }
    });
    _last = result.then((_) {}, onError: (_) {});
    return result;
  }
}

/// [send] as the native wallet calls it. A failure becomes its message, and
/// the status of a lost response: thrown across the bridge, it would carry
/// the Dart stack trace with it, and the wallet's callback cannot fail.
NativeTransport _nativeTransport(ZolanaTransport send) => (request) async {
  try {
    return native.TransportOutcome(response: await send(request));
  } on TransportResponseLost catch (lost) {
    return native.TransportOutcome(
      failure: native.TransportFailure(
        message: _message(lost.cause),
        status: lost.status,
      ),
    );
  } catch (error) {
    return native.TransportOutcome(
      failure: native.TransportFailure(message: _message(error)),
    );
  }
};

String _message(Object error) {
  try {
    return '$error';
  } catch (_) {
    return 'transport error';
  }
}

/// The default transport's policy: `https`, or `http` to this device.
void _refusePlaintext(native.WalletConfig config, bool allowInsecureHttp) {
  if (allowInsecureHttp) return;
  for (final (url, insecure) in [
    (config.rpcUrl, (String url) => WalletError.rpcUrlInsecure(url: url)),
    (
      config.indexerUrl,
      (String url) => WalletError.indexerUrlInsecure(url: url),
    ),
    (
      config.provingKeyUrl,
      (String url) => WalletError.provingKeyUrlInsecure(url: url),
    ),
    (config.proverUrl, (String url) => WalletError.proverUrlInsecure(url: url)),
  ]) {
    if (url != null && !isSecureUrl(url)) {
      throw ZolanaWalletException(insecure(_redactApiKeys(url)));
    }
  }
}

/// [text] with the value of every `api-key` parameter replaced by
/// `redacted`, as the native wallet masks them in its errors.
String _redactApiKeys(String text) => text.replaceAllMapped(
  RegExp(r'(api-key=)[^&#)"\s]*'),
  (match) => '${match[1]}redacted',
);

Future<T> _native<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on WalletError catch (error) {
    throw ZolanaWalletException(error);
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
    NativeTransport transport,
  ) async => native.MobileWallet.open(
    config: config,
    solanaPubkey: solanaPubkey,
    derivationSignature: derivationSignature,
    transport: await native.Transport.newInstance(send: transport),
  );

  @override
  Future<native.MobileWallet> openWithKeys(
    native.WalletConfig config,
    String solanaPubkey,
    native.WalletKeys keys,
    NativeTransport transport,
  ) async => native.MobileWallet.openWithKeys(
    config: config,
    solanaPubkey: solanaPubkey,
    keys: keys,
    // A native transport per wallet: opening a wallet consumes it.
    transport: await native.Transport.newInstance(send: transport),
  );
}
