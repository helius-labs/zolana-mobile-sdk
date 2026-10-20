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

/// What a prepared transaction's [operation] does, to show before signing. A
/// mint the demo does not hold shows in base units.
String approvalText(Operation operation) {
  String format(BigInt units, String? mint) => switch (demoAsset(mint)) {
    final asset? => asset.format(units),
    null => '$units units of ${shortAddress(mint!)}',
  };
  // Each output says whether its recipient receives it privately, as a note,
  // or publicly, to its Solana account.
  String pay(Output output, String? mint) => switch (output) {
    Output_Private(:final recipient, :final amount) =>
      '${format(amount, mint)} privately to ${shortAddress(recipient)}',
    Output_Public(:final recipient, :final amount) =>
      '${format(amount, mint)} publicly to ${shortAddress(recipient)}',
  };
  return switch (operation) {
    Operation_Registration() => 'Register for private payments',
    Operation_Deposit(:final mint, :final amount) =>
      'Shield ${format(amount, mint)}',
    Operation_Spend(:final mint, :final outputs) =>
      'Send ${outputs.map((output) => pay(output, mint)).join(', ')}',
    Operation_Merging(:final enabled) =>
      enabled ? 'Allow merges of your notes' : 'Stop merges of your notes',
    Operation_Merge(:final mint, :final amount) =>
      'Combine notes holding ${format(amount, mint)}',
    Operation_Custom(:final programs) =>
      'Call ${programs.map(shortAddress).join(', ')}',
  };
}

/// A sentence for the wallet's failures, amounts in [asset]; other errors
/// pass through.
String friendlyError(Object error, {DemoAsset asset = sol}) {
  if (error is! WalletError) return '$error';
  return switch (error) {
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
    WalletError_TransactionExpired() =>
      "This payment didn't go through in time. Nothing was sent; try again.",
    WalletError_TransactionOutcomeUnknown() => "Couldn't tell whether this payment went through. Check activity before trying again.",
    WalletError_Client(:final message)
        when message.contains('already used or queued') =>
      'Your balance just changed. Try again.',
    WalletError_Client(:final message) when message.contains('insufficient') =>
      'Not enough balance for this and its fees.',
    WalletError_Client(:final message) => message,
    final other => '$other',
  };
}
