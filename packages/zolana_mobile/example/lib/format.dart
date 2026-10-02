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

/// What a prepared transaction does, to show before signing. The demo holds
/// SOL only, so token amounts stay in base units.
String approvalText(PreparedTransaction tx) {
  final amount = switch ((tx.amount, tx.mint)) {
    (null, _) => '',
    (final lamports?, null) => formatSol(lamports),
    (final units?, final mint?) => '$units units of ${shortAddress(mint)}',
  };
  final to = tx.recipient == null ? '' : shortAddress(tx.recipient!);
  return switch (tx.kind) {
    PendingTransactionKind.registration => 'Register for private payments',
    PendingTransactionKind.deposit => 'Shield $amount',
    PendingTransactionKind.transfer => 'Send $amount privately to $to',
    PendingTransactionKind.withdrawal => 'Unshield $amount to $to',
    PendingTransactionKind.tokenAccount => 'Create a token account for $to',
  };
}

/// A sentence for the wallet's failures; other errors pass through.
String friendlyError(Object error) {
  if (error is! ZolanaWalletException) return '$error';
  return switch (error.error) {
    WalletError_RecipientNotRegistered() =>
      "That account hasn't set up a private wallet yet.",
    WalletError_RegistrationConflict() =>
      'This account is registered with keys from another app.',
    WalletError_InsufficientPrivateBalance(:final available) =>
      'Insufficient private balance: ${formatSol(available)} available.',
    WalletError_MergeRequired(:final maxInputs) =>
      'This amount needs more than $maxInputs notes. Send a smaller amount.',
    WalletError_TooManyInputTrees() =>
      'Your balance is split across trees. Send a smaller amount.',
    WalletError_AmountZero() => 'Enter an amount above zero.',
    WalletError_NotesReserved() =>
      'Another payment is waiting to be sent. Finish or cancel it first.',
    WalletError_ProverBusy() =>
      'Another proof is running. Try again in a moment.',
    WalletError_ProvingKeyDownloadFailed() =>
      "Couldn't download the proving key. Check your connection.",
    WalletError_ProofFailed() => "Couldn't prove this transaction.",
    WalletError_Client(:final message)
        when message.contains('already used or queued') =>
      'Your balance just changed. Try again.',
    WalletError_Client(:final message) when message.contains('insufficient') =>
      'Not enough balance for this and its fees.',
    WalletError_Client(:final message) => message,
    final other => '$other',
  };
}
