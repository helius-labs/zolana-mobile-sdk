import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

import 'action_sheet.dart';
import 'demo_keys.dart';
import 'demo_signer.dart';
import 'format.dart';
import 'main.dart' show DemoScreen;
import 'test_cluster_rpc.dart';

const _apiKey = String.fromEnvironment('ZOLANA_API_KEY');

/// SOL kept public when shielding the maximum, for this account's fees.
final _feeReserve = BigInt.from(10000000);

/// Enough public SOL to pay for registering and a first shield.
final _setupMinimum = BigInt.from(5000000);

/// Zolana devnet (the devnet-c stack, which runs the zolana revision this SDK
/// pins). `--dart-define=ZOLANA_API_KEY=...` uses Helius for Solana RPC
/// instead of the rate-limited public endpoint.
class Network {
  const Network(this.rpcUrl, this.indexerUrl);

  final String rpcUrl;
  final String indexerUrl;

  static const devnet = Network(
    _apiKey == ''
        ? 'https://api.devnet.solana.com'
        : 'https://devnet.helius-rpc.com/?api-key=$_apiKey',
    'https://d2xah7tnhdhcom.cloudfront.net',
  );
}

enum _Phase { opening, needsFunds, settingUp, ready, failed }

class WalletScreen extends StatefulWidget {
  const WalletScreen({super.key});

  @override
  State<WalletScreen> createState() => _WalletScreenState();
}

class _WalletScreenState extends State<WalletScreen> {
  Network _network = Network.devnet;
  DemoAccount _account = demoAccounts.first;
  _Phase _phase = _Phase.opening;
  String _status = 'Opening wallet…';
  String? _error;
  ZolanaWallet? _wallet;
  BigInt? _public;
  BigInt? _private;
  List<ActivityEntry> _activity = const [];
  bool _refreshing = false;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    _open();
  }

  @override
  void dispose() {
    unawaited(_wallet?.close());
    super.dispose();
  }

  bool _current(int generation) => mounted && generation == _generation;

  Future<void> _open() async {
    final generation = ++_generation;
    final previous = _wallet;
    setState(() {
      _phase = _Phase.opening;
      _status = 'Opening wallet…';
      _wallet = null;
      _public = null;
      _private = null;
      _activity = const [];
    });
    try {
      // Release the previous account before this one proves.
      await previous?.close();
      final signer = await DemoSigner.fromSeedHex(_account.seedHex);
      final keys =
          '${(await getApplicationSupportDirectory()).path}/proving-keys';
      final wallet = await ZolanaWallet.open(
        signer: signer,
        config: WalletConfig(
          rpcUrl: _network.rpcUrl,
          indexerUrl: _network.indexerUrl,
          provingKeyDir: keys,
          allowInsecureHttp: Uri.parse(_network.indexerUrl).scheme == 'http',
        ),
      );
      if (!_current(generation)) {
        unawaited(wallet.close());
        return;
      }
      _wallet = wallet;
      await _finishSetup(generation);
    } catch (error) {
      _fail(generation, error);
    }
  }

  /// Registration is the one step a new wallet needs before others can pay
  /// it. Do it without asking, once the account can pay for it.
  Future<void> _finishSetup(int generation) async {
    final wallet = _wallet!;
    try {
      final public = await wallet.publicBalance();
      if (!_current(generation)) return;
      setState(() => _public = public);
      final registration = await wallet.registrationStatus();
      if (registration == RegistrationStatus.conflict) {
        throw const ZolanaWalletException('registration_conflict');
      }
      if (registration == RegistrationStatus.notRegistered) {
        if (public < _setupMinimum) {
          setState(() => _phase = _Phase.needsFunds);
          return;
        }
        setState(() {
          _phase = _Phase.settingUp;
          _status = 'Setting up your private balance…';
        });
        await wallet.register();
      }
      if (!_current(generation)) return;
      setState(() => _phase = _Phase.ready);
      await _refresh();
    } catch (error) {
      _fail(generation, error);
    }
  }

  void _fail(int generation, Object error) {
    if (!_current(generation)) return;
    setState(() {
      _phase = _Phase.failed;
      _error = friendlyError(error);
    });
  }

  Future<void> _refresh() async {
    final wallet = _wallet;
    if (wallet == null || _refreshing) return;
    final generation = _generation;
    setState(() => _refreshing = true);
    try {
      await wallet.sync();
      final private = await wallet.privateBalance();
      final public = await wallet.publicBalance();
      final activity = await wallet.activity();
      if (!_current(generation)) return;
      setState(() {
        _private = private;
        _public = public;
        // The demo shows SOL only.
        _activity = activity
            .where(
              (entry) =>
                  entry.mint == null && entry.kind != ActivityKind.internal,
            )
            .toList();
      });
    } catch (error) {
      if (_current(generation)) _snack(friendlyError(error));
    } finally {
      if (_current(generation)) setState(() => _refreshing = false);
    }
  }

  Future<void> _airdrop() async {
    final wallet = _wallet;
    if (wallet == null) return;
    final generation = _generation;
    _snack('Requesting 1 devnet SOL…');
    try {
      await TestClusterRpc(_network.rpcUrl)
          .airdrop(wallet.solanaPublicKey, lamportsPerSol);
      if (!_current(generation)) return;
      if (_phase == _Phase.needsFunds) {
        await _finishSetup(generation);
      } else {
        await _refresh();
      }
    } catch (_) {
      if (_current(generation)) {
        _snack('Devnet airdrop is rate limited. Use faucet.solana.com.');
      }
    }
  }

  Future<void> _act(WalletAction action) async {
    final wallet = _wallet!;
    final available = switch (action) {
      WalletAction.shield => _maxShield(),
      _ => _private ?? BigInt.zero,
    };
    final other = demoAccounts.firstWhere((account) => account != _account);
    await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (_) => ActionSheet(
        action: action,
        available: available,
        suggestedRecipient: action == WalletAction.send ? other : null,
        run: (lamports, recipient) => switch (action) {
          WalletAction.send => wallet.transfer(
            recipient: recipient!,
            amount: lamports,
          ),
          WalletAction.shield => wallet.deposit(lamports),
          WalletAction.unshield => wallet.withdraw(
            recipient: wallet.solanaPublicKey,
            amount: lamports,
          ),
        },
      ),
    );
    // However the sheet closed (Done, swipe, back) the balance may have
    // changed, so the screen never shows notes the action already spent.
    await _refresh();
  }

  BigInt _maxShield() {
    final public = _public ?? BigInt.zero;
    return public > _feeReserve ? public - _feeReserve : BigInt.zero;
  }

  void _receive() => showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    builder: (_) => ReceiveSheet(address: _wallet!.solanaPublicKey),
  );

  void _snack(String message) =>
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(message)));

  Future<void> _editNetwork() async {
    final rpc = TextEditingController(text: _network.rpcUrl);
    final indexer = TextEditingController(text: _network.indexerUrl);
    final saved = await showDialog<Network>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Network'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: rpc,
              decoration: const InputDecoration(labelText: 'Solana RPC'),
            ),
            TextField(
              controller: indexer,
              decoration: const InputDecoration(labelText: 'Zolana indexer'),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, Network.devnet),
            child: const Text('Reset'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(
              context,
              Network(rpc.text.trim(), indexer.text.trim()),
            ),
            child: const Text('Save'),
          ),
        ],
      ),
    );
    rpc.dispose();
    indexer.dispose();
    if (saved != null) {
      _network = saved;
      await _open();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: PopupMenuButton<DemoAccount>(
          initialValue: _account,
          onSelected: (account) {
            if (account == _account) return;
            _account = account;
            _open();
          },
          itemBuilder: (_) => [
            for (final account in demoAccounts)
              PopupMenuItem(
                value: account,
                child: Text(
                  'Account ${account.name} · ${shortAddress(account.publicKey)}',
                ),
              ),
          ],
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text('Account ${_account.name}'),
              const Icon(Icons.arrow_drop_down),
            ],
          ),
        ),
        actions: [
          PopupMenuButton<String>(
            onSelected: (choice) => switch (choice) {
              'airdrop' => _airdrop(),
              'network' => _editNetwork(),
              _ => Navigator.of(context).push(
                MaterialPageRoute<void>(builder: (_) => const DemoScreen()),
              ),
            },
            itemBuilder: (_) => const [
              PopupMenuItem(
                value: 'airdrop',
                child: Text('Airdrop 1 devnet SOL'),
              ),
              PopupMenuItem(value: 'network', child: Text('Network')),
              PopupMenuItem(value: 'benchmark', child: Text('Proof benchmark')),
            ],
          ),
        ],
      ),
      body: switch (_phase) {
        _Phase.opening || _Phase.settingUp => _Centered(
          children: [
            const CircularProgressIndicator(),
            const SizedBox(height: 24),
            Text(_status),
          ],
        ),
        _Phase.needsFunds => _Centered(
          children: [
            Text(
              'Add devnet SOL to finish setup',
              style: Theme.of(context).textTheme.titleLarge,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 12),
            const Text(
              'Registering your private wallet costs a small fee.',
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 24),
            SelectableText(_account.publicKey, textAlign: TextAlign.center),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: _airdrop,
              child: const Text('Airdrop 1 devnet SOL'),
            ),
            TextButton(
              onPressed: () => _finishSetup(_generation),
              child: const Text('I funded it, check again'),
            ),
          ],
        ),
        _Phase.failed => _Centered(
          children: [
            const Icon(Icons.error_outline, size: 48),
            const SizedBox(height: 16),
            Text(_error ?? '', textAlign: TextAlign.center),
            const SizedBox(height: 24),
            FilledButton(onPressed: _open, child: const Text('Try again')),
          ],
        ),
        _Phase.ready => RefreshIndicator(
          onRefresh: _refresh,
          child: ListView(
            padding: const EdgeInsets.fromLTRB(24, 16, 24, 32),
            children: [
              _Balances(
                private: _private,
                public: _public,
                syncing: _refreshing,
              ),
              const SizedBox(height: 32),
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  _ActionButton(
                    icon: Icons.south,
                    label: 'Receive',
                    onPressed: _receive,
                  ),
                  _ActionButton(
                    icon: Icons.north_east,
                    label: 'Send',
                    onPressed: (_private ?? BigInt.zero) > BigInt.zero
                        ? () => _act(WalletAction.send)
                        : null,
                  ),
                  _ActionButton(
                    icon: Icons.shield_outlined,
                    label: 'Shield',
                    onPressed: _maxShield() > BigInt.zero
                        ? () => _act(WalletAction.shield)
                        : null,
                  ),
                  _ActionButton(
                    icon: Icons.lock_open,
                    label: 'Unshield',
                    onPressed: (_private ?? BigInt.zero) > BigInt.zero
                        ? () => _act(WalletAction.unshield)
                        : null,
                  ),
                ],
              ),
              const SizedBox(height: 32),
              Text('Activity', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              if (_activity.isEmpty)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 24),
                  child: Text(
                    _refreshing
                        ? 'Syncing…'
                        : 'Nothing yet. Shield some SOL to start.',
                    textAlign: TextAlign.center,
                  ),
                )
              else
                for (final entry in _activity)
                  _ActivityRow(
                    entry: entry,
                    onTap: () {
                      Clipboard.setData(
                        ClipboardData(text: explorerUrl(entry.signature)),
                      );
                      _snack('Explorer link copied');
                    },
                  ),
            ],
          ),
        ),
      },
    );
  }
}

class _Balances extends StatelessWidget {
  const _Balances({
    required this.private,
    required this.public,
    required this.syncing,
  });

  final BigInt? private;
  final BigInt? public;
  final bool syncing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final muted = theme.textTheme.bodyMedium?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    return Column(
      children: [
        Text('Private balance', style: muted),
        const SizedBox(height: 8),
        // An unknown balance is not zero: show a dash until the sync ends.
        Text(
          private == null ? '—' : formatSol(private!),
          style: theme.textTheme.displaySmall,
        ),
        const SizedBox(height: 8),
        Text(
          syncing
              ? 'Syncing…'
              : 'Public ${public == null ? '—' : formatSol(public!)}',
          style: muted,
        ),
      ],
    );
  }
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.icon,
    required this.label,
    required this.onPressed,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        IconButton.filledTonal(
          onPressed: onPressed,
          icon: Icon(icon),
          iconSize: 28,
          padding: const EdgeInsets.all(16),
        ),
        const SizedBox(height: 8),
        Text(label, style: Theme.of(context).textTheme.labelMedium),
      ],
    );
  }
}

class _ActivityRow extends StatelessWidget {
  const _ActivityRow({required this.entry, required this.onTap});

  final ActivityEntry entry;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final (icon, title, route, sign) = switch (entry.kind) {
      ActivityKind.shielded => (
        Icons.shield_outlined,
        'Shielded',
        'Public → Private',
        '+',
      ),
      ActivityKind.unshielded => (
        Icons.lock_open,
        'Unshielded',
        'Private → Public',
        '−',
      ),
      ActivityKind.sent => (Icons.north_east, 'Sent', 'Private transfer', '−'),
      ActivityKind.received => (
        Icons.south_west,
        'Received',
        'Private transfer',
        '+',
      ),
      ActivityKind.internal => (Icons.swap_horiz, 'Internal', '', ''),
    };
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: CircleAvatar(child: Icon(icon, size: 20)),
      title: Text(title),
      subtitle: Text(route),
      trailing: Text(
        '$sign${formatSol(entry.amount)}',
        style: Theme.of(context).textTheme.bodyLarge,
      ),
      onTap: onTap,
    );
  }
}

class _Centered extends StatelessWidget {
  const _Centered({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(32),
      child: Column(mainAxisSize: MainAxisSize.min, children: children),
    ),
  );
}
