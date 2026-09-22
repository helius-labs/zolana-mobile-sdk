import 'package:zolana_mobile/zolana_mobile.dart';

final lamportsPerSol = BigInt.from(1000000000);

/// `1.5 SOL`, trimmed to at most four decimals.
String formatSol(BigInt lamports) =>
    '${solString(lamports, maxDecimals: 4)} SOL';

/// Exact decimal SOL, or rounded down to [maxDecimals].
String solString(BigInt lamports, {int maxDecimals = 9}) {
  final whole = lamports ~/ lamportsPerSol;
  var fraction = (lamports % lamportsPerSol).toString().padLeft(9, '0');
  fraction = fraction.substring(0, maxDecimals).replaceAll(RegExp(r'0+$'), '');
  return fraction.isEmpty ? '$whole' : '$whole.$fraction';
}

/// Lamports for a decimal SOL string, or null if it is not one.
BigInt? parseSol(String text) {
  final match = RegExp(r'^(\d*)(?:\.(\d{0,9}))?$').firstMatch(text.trim());
  if (match == null ||
      (match.group(1)!.isEmpty && (match.group(2) ?? '').isEmpty)) {
    return null;
  }
  final whole = BigInt.parse(match.group(1)!.isEmpty ? '0' : match.group(1)!);
  final fraction = BigInt.parse((match.group(2) ?? '').padRight(9, '0'));
  return whole * lamportsPerSol + fraction;
}

final _base58 = RegExp(r'^[1-9A-HJ-NP-Za-km-z]{32,44}$');

bool isSolanaAddress(String value) => _base58.hasMatch(value);

String shortAddress(String value) => value.length <= 12
    ? value
    : '${value.substring(0, 4)}…${value.substring(value.length - 4)}';

String explorerUrl(String signature) =>
    'https://explorer.solana.com/tx/$signature?cluster=devnet';

/// A sentence for the wallet's failure codes; client errors pass through.
String friendlyError(Object error) {
  final message = error is ZolanaWalletException ? error.message : '$error';
  return switch (message) {
    'recipient_not_registered' =>
      "That account hasn't set up a private wallet yet.",
    'prover_busy' => 'Another proof is running. Try again in a moment.',
    'proving_key_download_failed' =>
      "Couldn't download the proving key. Check your connection.",
    'proof_failed' => "Couldn't prove this transaction.",
    _ when message.contains('already used or queued') => 'Your balance changed since the last sync. Pull to refresh and try again.',
    _ when message.contains('insufficient') =>
      'Not enough balance for this and its fees.',
    _ => message,
  };
}
