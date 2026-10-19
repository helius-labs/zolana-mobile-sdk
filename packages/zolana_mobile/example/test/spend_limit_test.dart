import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/spend_limit.dart';

TokenBalance _balance(List<(int, int)> limits) => TokenBalance(
  mint: null,
  amount: BigInt.from(1000),
  notes: BigInt.from(30),
  reserved: BigInt.zero,
  limits: [
    for (final (outputs, amount) in limits)
      SpendLimit(outputs: BigInt.from(outputs), amount: BigInt.from(amount)),
  ],
);

void main() {
  test('a spend reads the narrowest width that holds its outputs', () {
    final balance = _balance([(2, 100), (4, 80), (8, 50), (16, 20)]);
    final expected = {0: 100, 1: 100, 2: 80, 3: 80, 4: 50, 7: 50, 15: 20};
    for (final MapEntry(key: private, value: limit) in expected.entries) {
      expect(
        spendLimit(balance, private),
        BigInt.from(limit),
        reason: '$private private outputs',
      );
    }
    expect(spendLimit(balance, 16), BigInt.zero);
    expect(spendLimit(null, 1), BigInt.zero);
  });

  test('the largest amount leaves room for its fee and no more', () {
    for (final limit in [0, 1, 199, 200, 201, 1000, 40120000, 1 << 40]) {
      final cap = BigInt.from(limit);
      final amount = largestAmount(cap, demoFee);
      expect(amount + demoFee(amount) <= cap, isTrue, reason: '$limit');
      final next = amount + BigInt.one;
      expect(next + demoFee(next) > cap, isTrue, reason: '$limit');
    }
    expect(
      largestAmount(BigInt.from(500), (_) => BigInt.zero),
      BigInt.from(500),
    );
  });
}
