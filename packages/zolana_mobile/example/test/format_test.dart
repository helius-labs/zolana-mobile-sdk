import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile_demo/demo_assets.dart';
import 'package:zolana_mobile_demo/format.dart';

void main() {
  test('parses and formats SOL amounts exactly', () {
    expect(sol.parse('0.1'), BigInt.from(100000000));
    expect(sol.parse('1'), BigInt.from(1000000000));
    expect(sol.parse('.5'), BigInt.from(500000000));
    expect(sol.parse('0.000000001'), BigInt.one);
    for (final invalid in ['', '.', 'abc', '1.0000000001', '-1', '1,5']) {
      expect(sol.parse(invalid), isNull, reason: invalid);
    }
    expect(sol.amountString(BigInt.from(1234567891)), '1.234567891');
    expect(formatSol(BigInt.from(1234567891)), '1.2345 SOL');
    expect(formatSol(BigInt.zero), '0 SOL');
    expect(
      sol.parse(sol.amountString(BigInt.from(987654321))),
      BigInt.from(987654321),
    );
  });

  test('parses and formats token amounts in their decimals', () {
    expect(demoToken.parse('2.5'), BigInt.from(2500000));
    expect(demoToken.parse('0.000001'), BigInt.one);
    expect(demoToken.parse('0.0000001'), isNull);
    expect(demoToken.format(BigInt.from(1234567)), '1.2345 TEST');
    expect(demoToken.amountString(BigInt.from(1000000)), '1');
    expect(demoAsset(demoToken.mint), demoToken);
    expect(demoAsset(null), sol);
    expect(demoAsset('another mint'), isNull);
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
