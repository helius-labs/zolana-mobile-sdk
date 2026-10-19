import 'package:zolana_mobile/zolana_mobile.dart';

/// What one spend with [privateOutputs] private outputs can take now without
/// a merge, its fee included. The private outputs and the change take the
/// proof's output slots and public outputs take none, so this is the limit
/// of the narrowest proof width that holds `privateOutputs + 1`. Zero without
/// notes.
BigInt spendLimit(TokenBalance? balance, int privateOutputs) {
  final width = BigInt.from(privateOutputs + 1);
  // The limits are ascending by width, so the first that holds it is the
  // narrowest.
  for (final limit in balance?.limits ?? const <SpendLimit>[]) {
    if (limit.outputs >= width) return limit.amount;
  }
  return BigInt.zero;
}

/// The largest amount whose [fee] on top still fits in [limit]. [fee] must
/// not shrink as the amount grows.
BigInt largestAmount(BigInt limit, BigInt Function(BigInt amount) fee) {
  var low = BigInt.zero;
  var high = limit;
  while (low < high) {
    final mid = (low + high + BigInt.one) >> 1;
    if (mid + fee(mid) <= limit) {
      low = mid;
    } else {
      high = mid - BigInt.one;
    }
  }
  return low;
}
