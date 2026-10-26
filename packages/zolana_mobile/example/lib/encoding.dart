import 'dart:typed_data';

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
  return '1' * bytes.takeWhile((byte) => byte == 0).length +
      digits.toString().split('').reversed.join();
}

Uint8List base58Decode(String text) {
  var value = BigInt.zero;
  for (final rune in text.runes) {
    final digit = _alphabet.indexOf(String.fromCharCode(rune));
    if (digit < 0) throw const FormatException('Invalid base58');
    value = value * BigInt.from(58) + BigInt.from(digit);
  }
  final bytes = <int>[];
  while (value > BigInt.zero) {
    bytes.add((value & BigInt.from(255)).toInt());
    value >>= 8;
  }
  return Uint8List.fromList([
    ...List<int>.filled(text.split('').takeWhile((c) => c == '1').length, 0),
    ...bytes.reversed,
  ]);
}
