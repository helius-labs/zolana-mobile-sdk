import 'package:zolana_mobile/zolana_mobile.dart';

import 'demo_assets.dart';

final lamportsPerSol = BigInt.from(1000000000);

/// `1.5 SOL`, trimmed to at most four decimals.
String formatSol(BigInt lamports) => sol.format(lamports);

final _base58 = RegExp(r'^[1-9A-HJ-NP-Za-km-z]{32,44}$');

bool isSolanaAddress(String value) => _base58.hasMatch(value);

String shortAddress(String value) => value.length <= 12
    ? value
    : '${value.substring(0, 4)}…${value.substring(value.length - 4)}';

String explorerUrl(String signature) =>
    'https://explorer.solana.com/tx/$signature?cluster=devnet';

/// What a prepared transaction does, to show before signing. A mint the demo
/// does not hold shows in base units.
String approvalText(PreparedTransaction tx) {
  final amount = switch ((tx.amount, demoAsset(tx.mint))) {
    (null, _) => '',
    (final units?, final asset?) => asset.format(units),
    (final units?, null) => '$units units of ${shortAddress(tx.mint!)}',
  };
  final to = tx.recipient == null ? '' : shortAddress(tx.recipient!);
  return switch (tx.kind) {
    PendingTransactionKind.registration => 'Register for private payments',
    PendingTransactionKind.deposit => 'Shield $amount',
    PendingTransactionKind.transfer => 'Send $amount privately to $to',
    PendingTransactionKind.withdrawal => 'Unshield $amount to $to',
    PendingTransactionKind.merging => 'Change who can merge your notes',
    PendingTransactionKind.merge => 'Combine notes holding $amount',
  };
}

/// A sentence for the wallet's failures, amounts in [asset]; other errors
/// pass through.
String friendlyError(Object error, {DemoAsset asset = sol}) {
  if (error is! ZolanaWalletException) return '$error';
  return switch (error.error) {
    WalletError_RecipientNotRegistered() =>
      "That account hasn't set up a private wallet yet.",
    WalletError_RegistrationConflict() =>
      'This account is registered with keys from another app.',
    WalletError_InsufficientPrivateBalance(:final available) =>
      'Insufficient private balance: ${asset.format(available)} available.',
    WalletError_MergeRequired(:final maxInputs) =>
      'This amount needs more than $maxInputs notes. Send a smaller amount.',
    WalletError_TooManyInputTrees() =>
      'Your balance is split across trees. Send a smaller amount.',
    WalletError_AmountZero() => 'Enter an amount above zero.',
    WalletError_NothingToMerge() => 'There is nothing to merge yet.',
    WalletError_NotesAlreadySpent() =>
      'Your balance is still updating. Try again in a moment.',
    WalletError_MergingDisabled() =>
      'Merging is off for this account. Turn it on first.',
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
