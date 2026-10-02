import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

import 'format.dart';

/// An in-memory Ed25519 key standing in for the wallet's real signer.
///
/// A production app signs through a platform keystore, a Solana wallet
/// adapter or a custodian instead, and asks the user before signing. This one
/// tells [onSign] what it signs, from the transaction's facts. Use it only on
/// test clusters.
class DemoSigner implements SolanaSigner {
  DemoSigner._(this._keyPair, this.publicKey, this.onSign);

  final SimpleKeyPair _keyPair;
  final void Function(String approval)? onSign;

  @override
  final String publicKey;

  static Future<DemoSigner> generate() async =>
      _from(await Ed25519().newKeyPair(), null);

  /// The signer for a 32-byte Ed25519 seed given as hex.
  static Future<DemoSigner> fromSeedHex(
    String seedHex, {
    void Function(String approval)? onSign,
  }) async {
    final seed = [
      for (var i = 0; i < seedHex.length; i += 2)
        int.parse(seedHex.substring(i, i + 2), radix: 16),
    ];
    return _from(await Ed25519().newKeyPairFromSeed(seed), onSign);
  }

  static Future<DemoSigner> _from(
    SimpleKeyPair keyPair,
    void Function(String approval)? onSign,
  ) async {
    final publicKey = await keyPair.extractPublicKey();
    return DemoSigner._(keyPair, base58Encode(publicKey.bytes), onSign);
  }

  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    PreparedTransaction? transaction,
  }) async {
    onSign?.call(
      transaction == null
          ? 'Open your private Zolana wallet'
          : approvalText(transaction),
    );
    final signature = await Ed25519().sign(message, keyPair: _keyPair);
    return Uint8List.fromList(signature.bytes);
  }
}

const _alphabet = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

/// Bitcoin-alphabet base58, the encoding of Solana public keys.
String base58Encode(List<int> bytes) {
  var value = BigInt.zero;
  for (final byte in bytes) {
    value = (value << 8) | BigInt.from(byte);
  }
  final digits = StringBuffer();
  final base = BigInt.from(58);
  while (value > BigInt.zero) {
    digits.write(_alphabet[(value % base).toInt()]);
    value = value ~/ base;
  }
  final leadingZeros = bytes.takeWhile((byte) => byte == 0).length;
  return '1' * leadingZeros + digits.toString().split('').reversed.join();
}
