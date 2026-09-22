import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

/// An in-memory Ed25519 key standing in for the wallet's real signer.
///
/// A production app signs through a platform keystore, a Solana wallet
/// adapter or a custodian instead, and shows [purpose] before signing. This
/// key is generated per launch and lost on exit: use it only on test clusters.
class DemoSigner implements SolanaSigner {
  DemoSigner._(this._keyPair, this.publicKey);

  final SimpleKeyPair _keyPair;

  @override
  final String publicKey;

  static Future<DemoSigner> generate() async {
    final keyPair = await Ed25519().newKeyPair();
    final publicKey = await keyPair.extractPublicKey();
    return DemoSigner._(keyPair, base58Encode(publicKey.bytes));
  }

  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    required String purpose,
  }) async {
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
