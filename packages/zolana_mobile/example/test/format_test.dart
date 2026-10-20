import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
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

  test('describes each operation for approval', () {
    const b = '8JEEsykAwsPBvtcXa5byTZbggSpC63EiXRW44khYsS8E';
    const collector = 'CWhA2o6Sh95Qjy3iYX1rQdMjEcZ2QX1qz3wWMcUdgCdt';
    expect(
      approvalText(
        Operation.spend(
          outputs: [
            Output.private(recipient: b, amount: BigInt.from(1000000000)),
            Output.private(recipient: collector, amount: BigInt.from(5000000)),
          ],
        ),
      ),
      'Send 1 SOL privately to 8JEE…sS8E, 0.005 SOL privately to CWhA…gCdt',
    );
    expect(
      approvalText(
        Operation.spend(
          mint: demoToken.mint,
          outputs: [
            Output.public(recipient: b, amount: BigInt.from(2500000)),
            Output.private(recipient: collector, amount: BigInt.from(12500)),
          ],
        ),
      ),
      'Send 2.5 TEST publicly to 8JEE…sS8E, 0.0125 TEST privately to CWhA…gCdt',
    );
    expect(
      approvalText(Operation.deposit(amount: BigInt.from(1000000000))),
      'Shield 1 SOL',
    );
    expect(
      approvalText(const Operation.merging(enabled: false)),
      'Stop merges of your notes',
    );
    expect(
      approvalText(const Operation.custom(programs: [collector])),
      'Call CWhA…gCdt',
    );
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
