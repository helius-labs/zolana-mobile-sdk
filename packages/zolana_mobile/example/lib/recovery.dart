import 'dart:typed_data';

import 'package:zolana_mobile/zolana_mobile.dart';

import 'demo_signer.dart' show base58Encode;

/// Where the application keeps each account's wallet keys between launches,
/// so that the wallet opens without asking for a signature.
///
/// The keys cannot move funds, but they show the wallet's balances and
/// history and link its spends. A real application keeps them in the
/// platform keystore (the iOS Keychain, the Android Keystore), on this device
/// only, without cloud sync or backups, apart per account and network.
abstract interface class KeyVault {
  Future<WalletKeys?> read(String account);
  Future<void> write(String account, WalletKeys keys);
  Future<void> delete(String account);
}

/// A [KeyVault] in memory, empty at each launch: the demo's stand-in for the
/// platform keystore.
class MemoryKeyVault implements KeyVault {
  final _keys = <String, WalletKeys>{};

  @override
  Future<WalletKeys?> read(String account) async => _keys[account];

  @override
  Future<void> write(String account, WalletKeys keys) async {
    _keys[account] = keys;
  }

  @override
  Future<void> delete(String account) async {
    _keys.remove(account);
  }
}

/// Opens [signer]'s wallet.
///
/// With no keys in [vault] for the account, as on a new phone or after a
/// reinstall, it asks [signer] for the derivation signature, exactly as on
/// first use, and saves the wallet's keys in [vault]. The same account
/// derives the same keys, and every note and the history are read from the
/// indexer: nothing from the old phone is needed.
///
/// With saved keys it opens from them and asks for nothing; [signer] still
/// signs each spend. Keys that no longer open the wallet are derived again.
Future<ZolanaWallet> openWallet({
  required WalletConfig config,
  required SolanaSigner signer,
  required KeyVault vault,
  ZolanaTransport? transport,
}) async {
  final account = signer.publicKey;
  final keys = await vault.read(account);
  if (keys != null) {
    try {
      return await ZolanaWallet.openWithKeys(
        config: config,
        solanaPublicKey: account,
        keys: keys,
        signer: signer,
        transport: transport,
      );
    } on WalletError_InvalidWalletKeys {
      await vault.delete(account);
    }
  }
  final wallet = await ZolanaWallet.open(
    config: config,
    signer: signer,
    transport: transport,
  );
  await vault.write(account, await wallet.exportKeys());
  return wallet;
}

/// Where the application keeps the signature of each account's send until
/// its outcome is known: storage that survives a restart, such as a file.
abstract interface class PendingSends {
  Future<String?> read(String account);
  Future<void> write(String account, String signature);
  Future<void> delete(String account);
}

/// [PendingSends] in memory: the demo's stand-in for the application's
/// storage.
class MemoryPendingSends implements PendingSends {
  final _signatures = <String, String>{};

  @override
  Future<String?> read(String account) async => _signatures[account];

  @override
  Future<void> write(String account, String signature) async {
    _signatures[account] = signature;
  }

  @override
  Future<void> delete(String account) async {
    _signatures.remove(account);
  }
}

/// Signs [transaction] with [signer] and submits it. Its signature, the
/// transaction signature, is saved in [pending] before it goes out, so that a
/// send the application was closed during is found again after a restart.
/// Returns the signature.
///
/// The signature stays saved when the wallet cannot tell whether the
/// transaction landed: [WalletError.transactionOutcomeUnknown], or
/// [WalletError.walletClosed] when the application locked during the send.
/// [settlePendingSend] then waits for it before anything else is sent. Any
/// other result is the outcome, and the signature is dropped. After
/// [WalletError.transactionExpired] the transaction can no longer land:
/// sending it again, through `ZolanaWallet.refresh` or prepared anew, is
/// safe.
Future<String> saveAndSend(
  ZolanaWallet wallet,
  SolanaSigner signer,
  PendingSends pending,
  PreparedTransaction transaction,
) async {
  final account = wallet.solanaPublicKey;
  final Uint8List signed;
  try {
    signed = await signer.signMessage(
      transaction.message,
      transaction: transaction,
    );
  } catch (_) {
    // Declined: it is never sent, so its notes are free for the next spend.
    if (!wallet.isClosed) await wallet.release(transaction);
    rethrow;
  }
  final signature = base58Encode(signed);
  await pending.write(account, signature);
  try {
    await wallet.submit(transaction, [signed]);
  } on WalletError catch (error) {
    if (error is! WalletError_TransactionOutcomeUnknown &&
        error is! WalletError_WalletClosed) {
      await pending.delete(account);
    }
    rethrow;
  }
  await pending.delete(account);
  return signature;
}

/// Waits for the send [pending] holds for [wallet]'s account, such as one
/// the application was closed during: until Solana confirms it and the
/// indexer has it, so that balances include it. Returns its signature, or
/// `null` when no send is pending. Call it after opening and before the next
/// spend.
///
/// Fails, the signature still saved, while the wallet cannot confirm the
/// transaction: look it up again before sending anything else.
Future<String?> settlePendingSend(
  ZolanaWallet wallet,
  PendingSends pending,
) async {
  final account = wallet.solanaPublicKey;
  final signature = await pending.read(account);
  if (signature == null) return null;
  await wallet.waitForTransaction(signature);
  await pending.delete(account);
  return signature;
}
