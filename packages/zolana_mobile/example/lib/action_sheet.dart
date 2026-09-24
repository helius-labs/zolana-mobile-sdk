import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'demo_keys.dart';
import 'format.dart';

enum WalletAction { send, shield, unshield }

/// One action from amount to result: the amount (with Max), the route the
/// funds take and what stays private, then progress and the signature.
class ActionSheet extends StatefulWidget {
  const ActionSheet({
    super.key,
    required this.action,
    required this.available,
    required this.run,
    this.suggestedRecipient,
  });

  final WalletAction action;

  /// Lamports the action can spend.
  final BigInt available;

  /// Performs the action and returns its transaction signature.
  final Future<String> Function(BigInt lamports, String? recipient) run;

  final DemoAccount? suggestedRecipient;

  @override
  State<ActionSheet> createState() => _ActionSheetState();
}

enum _Stage { input, working, done, failed }

class _ActionSheetState extends State<ActionSheet> {
  final _amount = TextEditingController();
  final _recipient = TextEditingController();
  _Stage _stage = _Stage.input;
  String? _signature;
  String? _error;

  @override
  void initState() {
    super.initState();
    _recipient.text = widget.suggestedRecipient?.publicKey ?? '';
    _amount.addListener(() => setState(() {}));
    _recipient.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _amount.dispose();
    _recipient.dispose();
    super.dispose();
  }

  bool get _needsRecipient => widget.action == WalletAction.send;

  String get _title => switch (widget.action) {
    WalletAction.send => 'Send privately',
    WalletAction.shield => 'Shield',
    WalletAction.unshield => 'Unshield',
  };

  String get _route {
    final recipient = _recipientLabel;
    return switch (widget.action) {
      WalletAction.send => 'Private balance → $recipient',
      WalletAction.shield => 'Public wallet → Private balance',
      WalletAction.unshield => 'Private balance → Public wallet',
    };
  }

  String get _recipientLabel {
    final suggested = widget.suggestedRecipient;
    final entered = _recipient.text.trim();
    if (suggested != null && entered == suggested.publicKey) {
      return 'Account ${suggested.name}';
    }
    return entered.isEmpty ? 'recipient' : shortAddress(entered);
  }

  String get _privacy => switch (widget.action) {
    WalletAction.send =>
      'Amount and recipient stay private. Proved on this device.',
    WalletAction.shield => 'Shielding is public: this account and the amount.',
    WalletAction.unshield => 'Unshielding is public: this account and the amount. Proved on this device.',
  };

  String get _working => switch (widget.action) {
    WalletAction.send => 'Proving on device and sending…',
    WalletAction.shield => 'Shielding…',
    WalletAction.unshield => 'Proving on device and unshielding…',
  };

  BigInt? get _lamports => parseSol(_amount.text);

  String? get _amountProblem {
    final lamports = _lamports;
    if (_amount.text.trim().isEmpty) return null;
    if (lamports == null || lamports <= BigInt.zero) return 'Enter an amount';
    if (lamports > widget.available) return 'More than available';
    return null;
  }

  String? get _recipientProblem {
    if (!_needsRecipient) return null;
    final value = _recipient.text.trim();
    if (value.isEmpty) return null;
    return isSolanaAddress(value) ? null : 'Not a Solana address';
  }

  bool get _ready =>
      _lamports != null &&
      _amountProblem == null &&
      _recipientProblem == null &&
      (!_needsRecipient || _recipient.text.trim().isNotEmpty);

  Future<void> _submit() async {
    setState(() => _stage = _Stage.working);
    try {
      final signature = await widget.run(
        _lamports!,
        _needsRecipient ? _recipient.text.trim() : null,
      );
      if (mounted) {
        setState(() {
          _signature = signature;
          _stage = _Stage.done;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = friendlyError(error);
          _stage = _Stage.failed;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        8,
        24,
        24 + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: AnimatedSize(
        duration: const Duration(milliseconds: 200),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: switch (_stage) {
            _Stage.input => _input(theme),
            _Stage.working => [
              const SizedBox(height: 32),
              const Center(child: CircularProgressIndicator()),
              const SizedBox(height: 24),
              Text(_working, textAlign: TextAlign.center),
              const SizedBox(height: 32),
            ],
            _Stage.done => [
              const SizedBox(height: 16),
              const Icon(Icons.check_circle_outline, size: 56),
              const SizedBox(height: 16),
              Text(
                '${_done()} ${formatSol(_lamports!)}',
                textAlign: TextAlign.center,
                style: theme.textTheme.titleLarge,
              ),
              const SizedBox(height: 8),
              Text(_route, textAlign: TextAlign.center),
              const SizedBox(height: 16),
              OutlinedButton.icon(
                onPressed: () => Clipboard.setData(
                  ClipboardData(text: explorerUrl(_signature!)),
                ),
                icon: const Icon(Icons.copy, size: 18),
                label: Text(
                  'Copy explorer link · ${shortAddress(_signature!)}',
                ),
              ),
              const SizedBox(height: 8),
              FilledButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: const Text('Done'),
              ),
            ],
            _Stage.failed => [
              const SizedBox(height: 16),
              const Icon(Icons.error_outline, size: 56),
              const SizedBox(height: 16),
              Text(_error!, textAlign: TextAlign.center),
              const SizedBox(height: 24),
              FilledButton(
                onPressed: () => setState(() => _stage = _Stage.input),
                child: const Text('Try again'),
              ),
            ],
          },
        ),
      ),
    );
  }

  String _done() => switch (widget.action) {
    WalletAction.send => 'Sent',
    WalletAction.shield => 'Shielded',
    WalletAction.unshield => 'Unshielded',
  };

  List<Widget> _input(ThemeData theme) => [
    Text(_title, style: theme.textTheme.titleLarge),
    const SizedBox(height: 16),
    if (_needsRecipient) ...[
      TextField(
        controller: _recipient,
        decoration: InputDecoration(
          labelText: 'To',
          helperText: 'A Solana account with a private wallet',
          errorText: _recipientProblem,
          border: const OutlineInputBorder(),
        ),
      ),
      const SizedBox(height: 16),
    ],
    TextField(
      controller: _amount,
      autofocus: !_needsRecipient,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      decoration: InputDecoration(
        labelText: 'Amount',
        suffixText: 'SOL',
        errorText: _amountProblem,
        helperText: 'Available ${formatSol(widget.available)}',
        border: const OutlineInputBorder(),
        suffixIcon: TextButton(
          onPressed: widget.available > BigInt.zero
              ? () => _amount.text = solString(widget.available)
              : null,
          child: const Text('Max'),
        ),
      ),
    ),
    const SizedBox(height: 16),
    Text(_route, style: theme.textTheme.bodyMedium),
    const SizedBox(height: 4),
    Text(
      _privacy,
      style: theme.textTheme.bodySmall?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
      ),
    ),
    const SizedBox(height: 24),
    FilledButton(onPressed: _ready ? _submit : null, child: Text(_title)),
  ];
}

/// Share this account: other Zolana wallets pay it by its Solana address.
class ReceiveSheet extends StatelessWidget {
  const ReceiveSheet({super.key, required this.address});

  final String address;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(24, 8, 24, 32),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Receive privately', style: theme.textTheme.titleLarge),
          const SizedBox(height: 16),
          SelectableText(
            address,
            style: theme.textTheme.bodyLarge?.copyWith(fontFamily: 'monospace'),
          ),
          const SizedBox(height: 8),
          Text(
            'Other Zolana wallets send to this address. What they send '
            'arrives in your private balance.',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 24),
          FilledButton.icon(
            onPressed: () {
              Clipboard.setData(ClipboardData(text: address));
              Navigator.of(context).pop();
            },
            icon: const Icon(Icons.copy, size: 18),
            label: const Text('Copy address'),
          ),
        ],
      ),
    );
  }
}
