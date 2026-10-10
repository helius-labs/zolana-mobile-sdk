import 'package:zolana_mobile/zolana_mobile.dart';

/// An asset the demo holds: SOL, or an SPL token by its mint.
class DemoAsset {
  const DemoAsset(this.symbol, this.decimals, {this.mint, this.tokenProgram});

  final String symbol;
  final int decimals;

  /// `null` for SOL.
  final String? mint;
  final String? tokenProgram;

  /// `1.5 SOL`, trimmed to at most four decimals.
  String format(BigInt units) =>
      '${amountString(units, maxDecimals: 4)} $symbol';

  /// Exact decimal amount, or rounded down to [maxDecimals].
  String amountString(BigInt units, {int? maxDecimals}) {
    final scale = BigInt.from(10).pow(decimals);
    var fraction = (units % scale).toString().padLeft(decimals, '0');
    fraction = fraction
        .substring(
          0,
          maxDecimals == null ? decimals : maxDecimals.clamp(0, decimals),
        )
        .replaceAll(RegExp(r'0+$'), '');
    final whole = units ~/ scale;
    return fraction.isEmpty ? '$whole' : '$whole.$fraction';
  }

  /// Base units for a decimal amount, or null if it is not one.
  BigInt? parse(String text) {
    final match = RegExp('^(\\d*)(?:\\.(\\d{0,$decimals}))?\$')
        .firstMatch(text.trim());
    if (match == null ||
        (match.group(1)!.isEmpty && (match.group(2) ?? '').isEmpty)) {
      return null;
    }
    final whole = BigInt.parse(match.group(1)!.isEmpty ? '0' : match.group(1)!);
    final fraction = BigInt.parse(
      (match.group(2) ?? '').padRight(decimals, '0'),
    );
    return whole * BigInt.from(10).pow(decimals) + fraction;
  }

  /// The wallet's config entry of a token; SOL needs none.
  MintConfig? get config => mint == null
      ? null
      : MintConfig(mint: mint!, tokenProgram: tokenProgram!);
}

const sol = DemoAsset('SOL', 9);

/// A devnet SPL Token mint the demo accounts hold, 1,000 each, registered
/// with the shielded pool. Account A is its mint authority; the native
/// devnet test (`wallet_flow`, `ZOLANA_E2E_MINT`) mints from it too.
const demoToken = DemoAsset(
  'TEST',
  6,
  mint: 'Y8FDajuDMQWu7d5ES3jbZ67NzZta266WuFgQ92N4yd6',
  tokenProgram: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
);

const demoAssets = [sol, demoToken];

/// The demo asset of [mint], `null` for SOL; `null` for a mint it does not
/// hold.
DemoAsset? demoAsset(String? mint) {
  for (final asset in demoAssets) {
    if (asset.mint == mint) return asset;
  }
  return null;
}
