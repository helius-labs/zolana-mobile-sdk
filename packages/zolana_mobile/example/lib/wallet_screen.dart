import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

import 'action_sheet.dart';
import 'demo_assets.dart';
import 'demo_keys.dart';
import 'demo_signer.dart';
import 'format.dart';
import 'main.dart' show DemoScreen;
import 'merge_sheet.dart';
import 'spend_checks.dart';
import 'spend_limit.dart';
import 'test_cluster_rpc.dart';

const _apiKey = String.fromEnvironment('ZOLANA_API_KEY');

/// SOL kept public when shielding the maximum, for this account's fees.
final _feeReserve = BigInt.from(10000000);

/// Enough public SOL to pay for registering and a first shield.
final _setupMinimum = BigInt.from(5000000);

class Network {
  const Network(this.rpcUrl, this.indexerUrl, this.proverUrl);

  final String rpcUrl;
  final String indexerUrl;
  final String proverUrl;

  /// Zolana's devnet-d deployment: Solana RPC through Helius, keyed by the
  /// Helius key from `--dart-define=ZOLANA_API_KEY=...`, and the deployment's
  /// indexer, directly until the Helius gateway serves devnet-d.
  static const devnet = Network(
    'https://beta-devnet.helius-rpc.com/?api-key=$_apiKey',
    'https://d21d98xh5lgv3b.cloudfront.net',
    'https://d1unet9hn0mci0.cloudfront.net',
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
  DemoAsset _asset = sol;
  _Phase _phase = _Phase.opening;
  String? _error;
  ZolanaWallet? _wallet;
  DemoSigner? _signer;

  /// Every asset's balances and history from the last refresh: switching
  /// assets only filters them.
  List<TokenBalance>? _balances;
  Map<String?, BigInt> _publics = const {};
  List<ActivityEntry> _history = const [];

  /// How many assets the last refresh found notes in but could not count:
  /// the balances and the history leave them out.
  int _unresolved = 0;
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
      _wallet = null;
      _signer = null;
      _balances = null;
      _publics = const {};
      _history = const [];
      _unresolved = 0;
      _refreshing = false;
    });
    try {
      // Release the previous account before this one proves.
      await previous?.close();
      if (identical(_network, Network.devnet) && _apiKey.isEmpty) {
        throw 'Build with --dart-define=ZOLANA_API_KEY=... '
            '(a Helius key) to use devnet.';
      }
      final signer = await DemoSigner.fromSeedHex(
        _account.seedHex,
        onSign: _snack,
        spendChecks: SpendChecks(TestClusterRpc(_network.rpcUrl)),
      );
      final keys =
          '${(await getApplicationSupportDirectory()).path}/proving-keys';
      final config = WalletConfig(
        rpcUrl: _network.rpcUrl,
        indexerUrl: _network.indexerUrl,
        proverUrl: _network.proverUrl,
        provingKeyDir: keys,
        maxSpendInputs: 8,
        mints: [for (final asset in demoAssets) ?asset.config],
      );
      final wallet = await ZolanaWallet.open(
        signer: signer,
        config: config,
        // A test cluster on the host machine is plaintext.
        allowInsecureHttp: [
          config.rpcUrl,
          config.indexerUrl,
          config.provingKeyUrl,
        ].nonNulls.any((url) => Uri.parse(url).scheme == 'http'),
      );
      if (!_current(generation)) {
        unawaited(wallet.close());
        return;
      }
      _wallet = wallet;
      _signer = signer;
      await _finishSetup(generation);
    } catch (error) {
      _fail(generation, error);
    }
  }

  /// Registration is the one step a new wallet needs before others can pay
  /// it. Do it without asking, once the account can pay for it.
  Future<void> _finishSetup(int generation) async {
    try {
      final wallet = _wallet!;
      // Registration is paid in SOL, whichever asset is shown.
      final public = await _publicBalance(wallet, sol);
      if (!_current(generation)) return;
      setState(() => _publics = {..._publics, null: public});
      final registration = await wallet.registrationStatus();
      if (!_current(generation)) return;
      if (registration == RegistrationStatus.conflict) {
        throw WalletError.registrationConflict(owner: wallet.solanaPublicKey);
      }
      if (registration == RegistrationStatus.notRegistered) {
        if (public < _setupMinimum) {
          setState(() => _phase = _Phase.needsFunds);
          return;
        }
        setState(() => _phase = _Phase.settingUp);
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
      // One read of the indexer covers every asset's balance and history;
      // the public balances are read beside it.
      final publics = Future.wait([
        for (final asset in demoAssets) _publicBalance(wallet, asset),
      ]);
      final overview = await wallet.overview();
      final public = await publics;
      if (!_current(generation)) return;
      setState(() {
        _balances = overview.balances;
        _history = overview.activity;
        _unresolved =
            overview.unresolvedAssets.assetIds.length +
            overview.unresolvedAssets.mints.length;
        _publics = {
          for (final (index, asset) in demoAssets.indexed)
            asset.mint: public[index],
        };
      });
    } catch (error) {
      if (_current(generation)) _snack(friendlyError(error, asset: _asset));
    } finally {
      if (_current(generation)) setState(() => _refreshing = false);
    }
  }

  /// Public SOL and tokens are plain Solana state, read with the app's own
  /// RPC client.
  Future<BigInt> _publicBalance(ZolanaWallet wallet, DemoAsset asset) {
    final rpc = TestClusterRpc(_network.rpcUrl);
    final mint = asset.mint;
    return mint == null
        ? rpc.balance(wallet.solanaPublicKey)
        : rpc.tokenBalance(wallet.solanaPublicKey, mint);
  }

  /// The selected asset's balance; `null` until the first refresh. An asset
  /// without notes has none.
  TokenBalance? get _balance {
    final balances = _balances;
    if (balances == null) return null;
    for (final balance in balances) {
      if (balance.mint == _asset.mint) return balance;
    }
    return null;
  }

  /// The selected asset's private balance; `null` until the first refresh.
  BigInt? get _private =>
      _balances == null ? null : _balance?.amount ?? BigInt.zero;

  BigInt? get _public => _publics[_asset.mint];

  List<ActivityEntry> get _activity => [
    for (final entry in _history)
      if (entry.mint == _asset.mint) entry,
  ];

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
      // The error can name the RPC URL, and with it the API key.
      if (_current(generation)) {
        _snack(
          'Airdrop failed. Devnet airdrops are rate limited: '
          'use faucet.solana.com.',
        );
      }
    }
  }

  Future<void> _act(WalletAction action) async {
    final wallet = _wallet!;
    final signer = _signer!;
    final balance = _balance;
    // Notes an unfinished payment holds are not free for this one.
    final free = balance == null
        ? BigInt.zero
        : balance.amount - balance.reserved;
    final (available, limit) = switch (action) {
      WalletAction.shield => (_maxShield(), null),
      // The recipient and the fee are private outputs.
      WalletAction.send => (free, spendLimit(balance, 2)),
      // The fee is the one private output: the withdrawal takes no slot.
      WalletAction.unshield => (free, spendLimit(balance, 1)),
    };
    final other = demoAccounts.firstWhere((account) => account != _account);
    final mint = _asset.mint;
    // The application's fee is one more private output, to its collector, in
    // the same proof: the payment and the fee land together or not at all.
    Output? fee(BigInt units) => demoFee(units) > BigInt.zero
        ? Output.private(
            recipient: demoFeeCollector.publicKey,
            amount: demoFee(units),
          )
        : null;
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (_) => ActionSheet(
        action: action,
        asset: _asset,
        available: available,
        limit: limit,
        suggestedRecipient: switch (action) {
          WalletAction.send => other,
          WalletAction.shield => null,
          WalletAction.unshield => _account,
        },
        fee: action == WalletAction.shield ? null : demoFee,
        run: (units, recipient) => switch (action) {
          WalletAction.send => signer.spend(
            wallet,
            outputs: [
              Output.private(recipient: recipient!, amount: units),
              ?fee(units),
            ],
            mint: mint,
          ),
          WalletAction.shield => wallet.deposit(units, mint: mint),
          // A token goes to the recipient's token account, which must exist:
          // this account's holds the demo token already.
          WalletAction.unshield => signer.spend(
            wallet,
            outputs: [
              Output.public(recipient: recipient!, amount: units),
              ?fee(units),
            ],
            mint: mint,
          ),
        },
      ),
    );
    // However the sheet closed (Done, swipe, back) the balance may have
    // changed, so the screen never shows notes the action already spent.
    await _refresh();
  }

  /// SOL keeps [_feeReserve] public for fees; a token can be shielded whole.
  BigInt _maxShield() {
    final public = _public ?? BigInt.zero;
    if (_asset.mint != null) return public;
    return public > _feeReserve ? public - _feeReserve : BigInt.zero;
  }

  void _receive() => showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    builder: (_) => ReceiveSheet(address: _wallet!.solanaPublicKey),
  );

  Future<void> _merge() async {
    final prover = Uri.tryParse(_network.proverUrl);
    if (prover == null ||
        prover.scheme != 'https' ||
        prover.host.isEmpty ||
        prover.host == 'localhost' ||
        (InternetAddress.tryParse(prover.host)?.isLoopback ?? false)) {
      _snack('Set a remote HTTPS TEE prover in Network first.');
      return;
    }
    final wallet = _wallet!;
    final asset = _asset;
    final generation = _generation;
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      isDismissible: false,
      enableDrag: false,
      builder: (_) => MergeSheet(
        asset: asset,
        notes: _balance?.notes ?? BigInt.zero,
        readEnabled: wallet.mergingEnabled,
        setEnabled: (enabled) => wallet.setMerging(enabled),
        merge: () => wallet.merge(mint: asset.mint, proving: Proving.remote),
      ),
    );
    if (_current(generation)) await _refresh();
  }

  void _snack(String message) =>
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(message)));

  Future<void> _editNetwork() async {
    final rpc = TextEditingController(text: _network.rpcUrl);
    final indexer = TextEditingController(text: _network.indexerUrl);
    final prover = TextEditingController(text: _network.proverUrl);
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
            TextField(
              controller: prover,
              decoration: const InputDecoration(labelText: 'TEE prover'),
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
              Network(rpc.text.trim(), indexer.text.trim(), prover.text.trim()),
            ),
            child: const Text('Save'),
          ),
        ],
      ),
    );
    rpc.dispose();
    indexer.dispose();
    prover.dispose();
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
            Text(
              _phase == _Phase.settingUp
                  ? 'Setting up your private balance…'
                  : 'Opening wallet…',
            ),
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
              Center(
                child: SegmentedButton<DemoAsset>(
                  segments: [
                    for (final asset in demoAssets)
                      ButtonSegment(value: asset, label: Text(asset.symbol)),
                  ],
                  selected: {_asset},
                  onSelectionChanged: (selected) =>
                      setState(() => _asset = selected.single),
                ),
              ),
              const SizedBox(height: 24),
              _Balances(
                asset: _asset,
                private: _private,
                public: _public,
                balance: _balance,
                refreshing: _refreshing,
              ),
              const SizedBox(height: 12),
              const Text(
                'Local proofs use at most 8 input notes. '
                'Merge with the TEE first if a payment needs more.',
                textAlign: TextAlign.center,
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
              const SizedBox(height: 16),
              OutlinedButton.icon(
                onPressed: _refreshing ? null : _merge,
                icon: const Icon(Icons.merge),
                label: const Text('Merge with TEE'),
              ),
              if (_unresolved > 0)
                Padding(
                  padding: const EdgeInsets.only(top: 16),
                  child: Text(
                    'Notes in $_unresolved unrecognized '
                    '${_unresolved == 1 ? 'asset are' : 'assets are'} '
                    'not counted.',
                    textAlign: TextAlign.center,
                  ),
                ),
              const SizedBox(height: 32),
              Text('Activity', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              if (_activity.isEmpty)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 24),
                  child: Text(
                    _refreshing
                        ? 'Loading…'
                        : 'Nothing yet. Shield some ${_asset.symbol} to start.',
                    textAlign: TextAlign.center,
                  ),
                )
              else
                for (final entry in _activity)
                  _ActivityRow(
                    asset: _asset,
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
    required this.asset,
    required this.private,
    required this.public,
    required this.balance,
    required this.refreshing,
  });

  final DemoAsset asset;
  final BigInt? private;
  final BigInt? public;

  /// The asset's notes, with what unfinished payments hold of them; `null`
  /// without notes.
  final TokenBalance? balance;
  final bool refreshing;

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
        // An unknown balance is not zero: show a dash until it is read.
        Text(
          private == null ? '—' : asset.format(private!),
          style: theme.textTheme.displaySmall,
        ),
        const SizedBox(height: 8),
        Text(
          refreshing
              ? 'Refreshing…'
              : 'Public ${public == null ? '—' : asset.format(public!)}',
          style: muted,
        ),
        if (balance case final balance?) ...[
          const SizedBox(height: 4),
          Text(
            'Reserved ${asset.format(balance.reserved)} · '
            'Free ${asset.format(balance.amount - balance.reserved)}',
            style: muted,
          ),
        ],
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
  const _ActivityRow({
    required this.asset,
    required this.entry,
    required this.onTap,
  });

  final DemoAsset asset;
  final ActivityEntry entry;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final (icon, title, route, sign) = switch (entry.kind) {
      ActivityKind.deposit => (
        Icons.shield_outlined,
        'Shielded',
        'Public → Private',
        '+',
      ),
      ActivityKind.withdrawal => (
        Icons.lock_open,
        'Unshielded',
        'Private → Public',
        '−',
      ),
      // A send, or an unshield that paid a private fee: the chain does not
      // tell them apart.
      ActivityKind.sent => (
        Icons.north_east,
        'Sent',
        'From private balance',
        '−',
      ),
      ActivityKind.received => (
        Icons.south_west,
        'Received',
        'Private transfer',
        '+',
      ),
      ActivityKind.selfTransfer => (Icons.swap_horiz, 'Self transfer', '', ''),
    };
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: CircleAvatar(child: Icon(icon, size: 20)),
      title: Text(title),
      subtitle: Text(route),
      trailing: Text(
        '$sign${asset.format(entry.amount)}',
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
