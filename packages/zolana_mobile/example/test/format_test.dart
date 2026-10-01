import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile_demo/format.dart';

void main() {
  test('parses and formats SOL amounts exactly', () {
    expect(parseSol('0.1'), BigInt.from(100000000));
    expect(parseSol('1'), BigInt.from(1000000000));
    expect(parseSol('.5'), BigInt.from(500000000));
    expect(parseSol('0.000000001'), BigInt.one);
    for (final invalid in ['', '.', 'abc', '1.0000000001', '-1', '1,5']) {
      expect(parseSol(invalid), isNull, reason: invalid);
    }
    expect(solString(BigInt.from(1234567891)), '1.234567891');
    expect(formatSol(BigInt.from(1234567891)), '1.2345 SOL');
    expect(formatSol(BigInt.zero), '0 SOL');
    expect(parseSol(solString(BigInt.from(987654321))), BigInt.from(987654321));
  });

  test('recognizes Solana addresses', () {
    expect(
      isSolanaAddress('DzWQxW7A9N4Fo4SYiZGws6RQ2mn8rD4n92LaqTfgH93N'),
      isTrue,
    );
    expect(isSolanaAddress('0OIl'), isFalse);
    expect(
      isSolanaAddress('DzWQxW7A9N4Fo4SYiZGws6RQ2mn8rD4n92LaqTfgH93N0'),
      isFalse,
    );
  });
}
