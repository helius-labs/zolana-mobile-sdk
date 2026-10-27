import 'dart:async';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

import 'demo_keys.dart';
import 'demo_signer.dart';
import 'sponsored_merge.dart';
import 'spend_checks.dart';
import 'test_cluster_rpc.dart';

const _backendUrl = String.fromEnvironment(
  'ZOLANA_BACKEND_URL',
  defaultValue: 'http://127.0.0.1:8788/',
);
const _auth = String.fromEnvironment('ZOLANA_BACKEND_AUTH');
const _rpc = String.fromEnvironment('ZOLANA_RPC_URL');
const _indexer = String.fromEnvironment(
  'ZOLANA_INDEXER_URL',
  defaultValue: 'https://d21d98xh5lgv3b.cloudfront.net',
);
const _policy = String.fromEnvironment('ZOLANA_TEE_POLICY');
const _syncOnly = bool.fromEnvironment('ZOLANA_PROVER_SYNC_ONLY');
const _mint = String.fromEnvironment('ZOLANA_MINT');
const _rounds = int.fromEnvironment('ZOLANA_MERGE_ROUNDS', defaultValue: 1);
const _tree = int.fromEnvironment('ZOLANA_TREE_ID', defaultValue: 0);
const _maxInputs = int.fromEnvironment('ZOLANA_MERGE_INPUTS', defaultValue: 8);

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initZolanaMobile();
  runApp(const MaterialApp(home: SponsoredDemo()));
}

class SponsoredDemo extends StatefulWidget {
  const SponsoredDemo({super.key});
  @override
  State<SponsoredDemo> createState() => _SponsoredDemoState();
}

class _SponsoredDemoState extends State<SponsoredDemo> {
  SponsoredBackend? _backend;
  ZolanaWallet? _wallet;
  SponsoredFlow? _flow;
  bool _busy = false;
  bool _withdraw = true;
  String _status = 'Opening demo wallet…';
  final _recipient = TextEditingController(text: demoAccounts[0].publicKey);
  final _amount = TextEditingController(text: '1000000');

  @override
  void initState() {
    super.initState();
    unawaited(_open());
  }

  Future<void> _open() => _run(() async {
    if (_rpc.isEmpty || _auth.isEmpty) {
      throw StateError('Set ZOLANA_RPC_URL and ZOLANA_BACKEND_AUTH');
    }
    final base = Uri.parse(
      _backendUrl.endsWith('/') ? _backendUrl : '$_backendUrl/',
    );
    final backend = SponsoredBackend(base, _auth);
    _backend = backend;
    final directory = await getApplicationSupportDirectory();
    final signer = await DemoSigner.fromSeedHex(demoAccounts[0].seedHex);
    final wallet = await ZolanaWallet.open(
      signer: signer,
      transport: backend.transport,
      config: WalletConfig(
        rpcUrl: _rpc,
        indexerUrl: _indexer,
        provingKeyDir: '${directory.path}/proving',
        proverUrl: backend.proverUrl,
        requireProverTee: true,
        proverSyncOnly: _syncOnly,
        proverTeePolicy: _policy.isEmpty ? null : _policy,
        proving: Proving.local,
      ),
    );
    _wallet = wallet;
    final flow = SponsoredFlow(
      wallet,
      signer,
      backend,
      await SponsoredStore.forWallet(directory, wallet),
      SpendChecks(TestClusterRpc(_rpc)),
    );
    await flow.restore();
    _flow = flow;
    _status = 'Ready. Account A must have devnet SOL for setup and the final payment.';
  });

  Future<void> _run(Future<void> Function() action) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await action();
    } catch (error) {
      _status = error.toString();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<bool> _approve(String title, String text) async {
    if (!mounted) return false;
    return await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text(title),
            content: Text(text),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: const Text('Cancel'),
              ),
              FilledButton(
                onPressed: () => Navigator.pop(context, true),
                child: const Text('Approve'),
              ),
            ],
          ),
        ) ??
        false;
  }

  Future<void> _setup() => _run(() async {
    if (!await _approve(
      'Enable merges',
      'Register demo account A if needed and enable merging. Account A pays the network fees.',
    )) {
      return;
    }
    await _wallet!.register();
    await _wallet!.setMerging(true);
    _status = 'Merging enabled. Shield enough notes for the requested rounds.';
  });

  Future<void> _deposit() => _run(() async {
    if (!await _approve(
      'Create demo notes',
      'Shield ${_rounds * 2} notes of 1,000,000 base units each. Account A pays for each deposit.',
    )) {
      return;
    }
    for (var i = 0; i < _rounds * 2; i++) {
      await _wallet!.deposit(
        BigInt.from(1000000),
        mint: _mint.isEmpty ? null : _mint,
      );
    }
    _status = 'Notes shielded.';
  });

  Future<void> _merge() => _run(() async {
    final flow = _flow!;
    if (flow.state == null || flow.complete) {
      await flow.start(
        rounds: _rounds,
        treeId: _tree,
        mint: _mint.isEmpty ? null : _mint,
      );
    }
    await flow.mergeRounds(maxInputs: _maxInputs);
    _status = 'Cache outputs verified. Prepare the final payment.';
  });

  Future<void> _prepare() => _run(() async {
    final amount = BigInt.parse(_amount.text);
    final output = _withdraw
        ? Output.public(recipient: _recipient.text.trim(), amount: amount)
        : Output.private(recipient: _recipient.text.trim(), amount: amount);
    await _flow!.preparePayment([output]);
    _status = 'Payment proved locally and saved. Review and sign to submit.';
  });

  Future<void> _send() => _run(() async {
    final signature = await _flow!.signAndSend(
      (transaction) => _approve(
        'Sign payment',
        '${transaction.operation}\nPriority fee: ${transaction.priorityFeeLamports} lamports\nAccount A signs and pays the network fee.',
      ),
    );
    _status = 'Confirmed: $signature';
  });

  Future<void> _reconcile() => _run(() async {
    _status = 'Confirmed: ${await _flow!.reconcilePayment()}';
  });

  @override
  void dispose() {
    final wallet = _wallet;
    final backend = _backend;
    if (wallet != null) {
      unawaited(wallet.close().whenComplete(() => backend?.close()));
    } else {
      backend?.close();
    }
    _recipient.dispose();
    _amount.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('Sponsored cache merge · devnet')),
    body: ListView(
      padding: const EdgeInsets.all(20),
      children: [
        const Text('Uses public demo account A. Devnet funds only.'),
        SelectableText(
          'Backend: $_backendUrl\nAccount: ${demoAccounts[0].publicKey}',
        ),
        const SizedBox(height: 16),
        Text('Run: ${_flow?.phase ?? "opening"}'),
        if (_busy) const LinearProgressIndicator(),
        SelectableText(_status),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: _busy || _wallet == null ? null : _setup,
          child: const Text('Register / enable merging'),
        ),
        OutlinedButton(
          onPressed: _busy || _wallet == null ? null : _deposit,
          child: const Text('Shield demo notes'),
        ),
        FilledButton(
          onPressed:
              _busy ||
                  _flow == null ||
                  (_flow!.payment != null && !_flow!.complete)
              ? null
              : _merge,
          child: const Text('Start / resume sponsored merges'),
        ),
        SwitchListTile(
          title: const Text('Withdraw publicly'),
          value: _withdraw,
          onChanged: _busy
              ? null
              : (value) => setState(() {
                  _withdraw = value;
                  _recipient.text = demoAccounts[value ? 0 : 1].publicKey;
                }),
        ),
        TextField(
          controller: _recipient,
          decoration: const InputDecoration(labelText: 'Recipient'),
        ),
        TextField(
          controller: _amount,
          keyboardType: TextInputType.number,
          decoration: const InputDecoration(labelText: 'Amount in base units'),
        ),
        FilledButton(
          onPressed: _busy || _flow?.phase != 'ready' ? null : _prepare,
          child: const Text('Prove payment locally'),
        ),
        OutlinedButton(
          onPressed: _busy || _flow?.phase != 'approval'
              ? null
              : () => _run(() async {
                  await _flow!.refreshPayment();
                  _status = 'Blockhash refreshed. Review and sign again.';
                }),
          child: const Text('Refresh unsigned payment'),
        ),
        FilledButton(
          onPressed: _busy || _flow?.phase != 'approval' ? null : _send,
          child: const Text('Review and sign'),
        ),
        OutlinedButton(
          onPressed: _busy || _flow?.phase != 'submitted' ? null : _reconcile,
          child: const Text('Recover submitted payment'),
        ),
      ],
    ),
  );
}
