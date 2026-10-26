import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

import 'encoding.dart';
import 'format.dart';
import 'spend_checks.dart';

export 'encoding.dart' show base58Encode;

/// An in-memory Ed25519 key standing in for the wallet's real signer.
///
/// A production app signs through a platform keystore, a Solana wallet
/// adapter or a custodian instead, and asks the user before signing. This one
/// tells [onSign] what it signs, from the transaction's facts. Use it only on
/// test clusters.
class DemoSigner implements SolanaSigner {
  DemoSigner._(this._keyPair, this.publicKey, this.onSign, this.spendChecks);

  final SimpleKeyPair _keyPair;
  final SpendChecks? spendChecks;
  SpendRequest? _spendRequest;
  final void Function(String approval)? onSign;

  @override
  final String publicKey;

  static Future<DemoSigner> generate() async =>
      _from(await Ed25519().newKeyPair(), null, null);

  /// The signer for a 32-byte Ed25519 seed given as hex.
  static Future<DemoSigner> fromSeedHex(
    String seedHex, {
    void Function(String approval)? onSign,
    SpendChecks? spendChecks,
  }) async {
    final seed = [
      for (var i = 0; i < seedHex.length; i += 2)
        int.parse(seedHex.substring(i, i + 2), radix: 16),
    ];
    return _from(await Ed25519().newKeyPairFromSeed(seed), onSign, spendChecks);
  }

  static Future<DemoSigner> _from(
    SimpleKeyPair keyPair,
    void Function(String approval)? onSign,
    SpendChecks? spendChecks,
  ) async {
    final publicKey = await keyPair.extractPublicKey();
    return DemoSigner._(
      keyPair,
      base58Encode(publicKey.bytes),
      onSign,
      spendChecks,
    );
  }

  /// Capture the UI's request independently of PreparedTransaction.operation.
  Future<String> spend(
    ZolanaWallet wallet, {
    required List<Output> outputs,
    String? mint,
    Proving? proving,
  }) async {
    if (_spendRequest != null) throw StateError('A payment is already pending');
    _spendRequest = SpendRequest(outputs: outputs, mint: mint);
    try {
      return await wallet.spend(outputs: outputs, mint: mint, proving: proving);
    } finally {
      _spendRequest = null;
    }
  }

  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    PreparedTransaction? transaction,
  }) async {
    // Sign exactly the byte snapshot we checked, even across asynchronous RPC.
    final bytes = Uint8List.fromList(message);
    final checks = spendChecks;
    var operation = transaction?.operation;
    if (checks != null &&
        (_spendRequest != null ||
            operation is Operation_Spend ||
            (transaction?.outputs.isNotEmpty ?? false))) {
      final request = _spendRequest;
      if (request == null || transaction == null) {
        throw StateError('No prepared payment was requested');
      }
      final created = await checks.verify(
        message: bytes,
        openings: transaction.outputs,
        sender: publicKey,
        request: request,
      );
      // Show the facts checked against the UI request, not the SDK summary.
      operation = Operation.spend(
        mint: request.mint,
        outputs: request.outputs,
        createdTokenAccounts: created,
      );
    }
    onSign?.call(
      operation == null
          ? 'Open your private Zolana wallet'
          : approvalText(operation),
    );
    final signature = await Ed25519().sign(bytes, keyPair: _keyPair);
    return Uint8List.fromList(signature.bytes);
  }
}
