import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'demo_assets.dart';
import 'format.dart';

class MergeSheet extends StatefulWidget {
  const MergeSheet({
    super.key,
    required this.asset,
    required this.notes,
    required this.readEnabled,
    required this.setEnabled,
    required this.merge,
  });

  final DemoAsset asset;
  final BigInt notes;
  final Future<bool> Function() readEnabled;
  final Future<String?> Function(bool enabled) setEnabled;
  final Future<String> Function() merge;

  @override
  State<MergeSheet> createState() => _MergeSheetState();
}

class _MergeSheetState extends State<MergeSheet> {
  bool? _enabled;
  bool _busy = true;
  bool _merged = false;
  String _progress = 'Checking merge setting…';
  String? _error;
  String? _signature;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final enabled = await widget.readEnabled();
      if (mounted) setState(() => _enabled = enabled);
    } catch (error) {
      if (mounted) setState(() => _error = friendlyError(error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _run({bool? enabled}) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _error = null;
      _signature = null;
      _progress = enabled == null
          ? 'Proving in the TEE and merging…'
          : 'Updating merge setting…';
    });
    try {
      final signature = enabled == null
          ? await widget.merge()
          : await widget.setEnabled(enabled);
      if (!mounted) return;
      setState(() {
        _signature = signature;
        if (enabled == null) {
          _merged = true;
        } else {
          _enabled = enabled;
        }
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = friendlyError(error, asset: widget.asset));
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !_busy,
    child: Padding(
      padding: const EdgeInsets.fromLTRB(24, 8, 24, 32),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Merge private notes',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 16),
          Text(
            'Combine up to 24 ${widget.asset.symbol} notes on one tree. '
            'Your private balance stays the same; you pay network fees from public SOL. '
            'The proof is generated in an attested TEE over an encrypted connection.',
          ),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Allow merges'),
            subtitle: const Text(
              'Allows a prover to combine your notes while keeping the funds in your wallet.',
            ),
            value: _enabled ?? false,
            onChanged: _busy || _enabled == null
                ? null
                : (enabled) => _run(enabled: enabled),
          ),
          if (_busy) ...[
            const Center(child: CircularProgressIndicator()),
            const SizedBox(height: 12),
            Text(_progress, textAlign: TextAlign.center),
          ] else if (_merged)
            const Text('Notes merged. Your private balance is unchanged.')
          else
            Text(
              widget.notes < BigInt.two
                  ? 'At least two notes are needed to merge.'
                  : '${widget.notes} private notes available.',
            ),
          if (_error != null) ...[
            const SizedBox(height: 12),
            Text(
              _error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ],
          if (_signature != null) ...[
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: () => Clipboard.setData(
                ClipboardData(text: explorerUrl(_signature!)),
              ),
              icon: const Icon(Icons.copy, size: 18),
              label: const Text('Copy explorer link'),
            ),
          ],
          const SizedBox(height: 16),
          FilledButton(
            onPressed:
                !_busy &&
                    !_merged &&
                    _enabled == true &&
                    widget.notes >= BigInt.two
                ? () => _run()
                : null,
            child: const Text('Merge with TEE'),
          ),
          TextButton(
            onPressed: _busy ? null : () => Navigator.of(context).pop(),
            child: const Text('Done'),
          ),
        ],
      ),
    ),
  );
}
