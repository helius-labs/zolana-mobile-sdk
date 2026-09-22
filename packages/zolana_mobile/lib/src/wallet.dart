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
  Future<bool> isRegistered();
  Future<native.SyncSummary> sync();
  Future<BigInt> privateLamports();
  Future<BigInt> publicLamports();
  Future<List<native.ActivityEntry>> activity();
  Future<NativePending?> prepareRegistration();
  Future<NativePending> prepareDeposit(BigInt lamports);
  Future<NativePending> prepareTransfer(String recipient, BigInt lamports);
  Future<NativePending> prepareWithdrawal(String recipient, BigInt lamports);
  Future<String> submit(NativePending pending, List<Uint8List> signatures);
}

/// A native transaction awaiting signatures.
abstract interface class NativePending {
  Future<String> summary();
  Future<Uint8List> messageBytes();
  Future<List<String>> signers();
}

/// A private SOL wallet that proves on the device.
///
/// Every operation runs after the previous one finishes: the native wallet
/// holds one proving key at a time and its state is updated by [sync].
class ZolanaWallet {
  ZolanaWallet._(this._signer, this._wallet, this.shieldedAddress);

  final SolanaSigner _signer;
  final NativeWallet _wallet;
  Future<void> _last = Future.value();

  /// The address other Zolana wallets pay, once [register] has published it.
  final String shieldedAddress;

  String get solanaPublicKey => _signer.publicKey;

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

  Future<bool> isRegistered() => _serial(_wallet.isRegistered);

  /// Publish [shieldedAddress] so others can send to this wallet. Returns the
  /// transaction signature, or `null` when it is already registered.
  Future<String?> register() => _serial(() async {
    final pending = await _wallet.prepareRegistration();
    return pending == null ? null : _signAndSubmit(pending);
  });

  /// Move public SOL from this account into the private balance. The
  /// deposit, its amount and this account are public.
  Future<String> deposit(BigInt lamports) => _serial(
    () async => _signAndSubmit(await _wallet.prepareDeposit(lamports)),
  );

  /// Fetch and decrypt this wallet's notes.
  Future<native.SyncSummary> sync() => _serial(_wallet.sync);

  /// Spendable private SOL as of the last [sync].
  Future<BigInt> privateLamports() => _serial(_wallet.privateLamports);

  /// Public SOL of [solanaPublicKey], read from the RPC now.
  Future<BigInt> publicLamports() => _serial(_wallet.publicLamports);

  /// SOL history found by the last [sync], newest first.
  Future<List<native.ActivityEntry>> activity() => _serial(_wallet.activity);

  /// Send private SOL to the registered wallet of [recipient] (a Solana
  /// public key). Builds and proves on the device, then asks the signer.
  Future<String> transfer({
    required String recipient,
    required BigInt lamports,
  }) => _serial(
    () async =>
        _signAndSubmit(await _wallet.prepareTransfer(recipient, lamports)),
  );

  /// Move private SOL to the public account [recipient]. The recipient and
  /// amount are public.
  Future<String> withdraw({
    required String recipient,
    required BigInt lamports,
  }) => _serial(
    () async =>
        _signAndSubmit(await _wallet.prepareWithdrawal(recipient, lamports)),
  );

  Future<String> _signAndSubmit(NativePending pending) async {
    final signers = await pending.signers();
    if (signers.length != 1 || signers.single != _signer.publicKey) {
      throw const ZolanaWalletException('unexpected_signers');
    }
    final signature = await _signer.signMessage(
      await pending.messageBytes(),
      purpose: await pending.summary(),
    );
    return _wallet.submit(pending, [signature]);
  }

  Future<T> _serial<T>(Future<T> Function() operation) {
    final result = _last.then((_) => _native(operation));
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
  Future<bool> isRegistered() => _wallet.isRegistered();

  @override
  Future<native.SyncSummary> sync() => _wallet.sync_();

  @override
  Future<BigInt> privateLamports() => _wallet.privateLamports();

  @override
  Future<BigInt> publicLamports() => _wallet.publicLamports();

  @override
  Future<List<native.ActivityEntry>> activity() => _wallet.activity();

  @override
  Future<NativePending?> prepareRegistration() async {
    final pending = await _wallet.prepareRegistration();
    return pending == null ? null : _NativePending(pending);
  }

  @override
  Future<NativePending> prepareDeposit(BigInt lamports) async =>
      _NativePending(await _wallet.prepareDeposit(lamports: lamports));

  @override
  Future<NativePending> prepareTransfer(
    String recipient,
    BigInt lamports,
  ) async => _NativePending(
    await _wallet.prepareTransfer(recipient: recipient, lamports: lamports),
  );

  @override
  Future<NativePending> prepareWithdrawal(
    String recipient,
    BigInt lamports,
  ) async => _NativePending(
    await _wallet.prepareWithdrawal(recipient: recipient, lamports: lamports),
  );

  @override
  Future<String> submit(NativePending pending, List<Uint8List> signatures) =>
      _wallet.submit(
        pending: (pending as _NativePending)._pending,
        signatures: signatures,
      );
}

class _NativePending implements NativePending {
  _NativePending(this._pending);

  final native.PendingTransaction _pending;

  @override
  Future<String> summary() => _pending.summary();

  @override
  Future<Uint8List> messageBytes() => _pending.messageBytes();

  @override
  Future<List<String>> signers() => _pending.signers();
}
