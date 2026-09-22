import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

import 'demo_signer.dart';
import 'test_cluster_rpc.dart';

const _lamportsPerSol = 1000000000;

/// The full private payment flow against a Zolana test cluster: fund a demo
/// key, register, deposit, sync, and send privately. Every transfer proof is
/// generated on this device.
class WalletScreen extends StatefulWidget {
  const WalletScreen({super.key});

  @override
  State<WalletScreen> createState() => _WalletScreenState();
}

class _WalletScreenState extends State<WalletScreen> {
  // An Android emulator reaches its host at 10.0.2.2; an iOS simulator
  // shares the host's loopback. Both match `just localnet` in zolana.
  final _rpcUrl = TextEditingController(
    text: Platform.isAndroid ? 'http://10.0.2.2:8899' : 'http://127.0.0.1:8899',
  );
  final _indexerUrl = TextEditingController(
    text: Platform.isAndroid ? 'http://10.0.2.2:8784' : 'http://127.0.0.1:8784',
  );
  final _recipient = TextEditingController();
  final _amount = TextEditingController(text: '0.1');
  final _log = <String>[];

  DemoSigner? _signer;
  ZolanaWallet? _wallet;
  BigInt? _publicLamports;
  BigInt? _privateLamports;
  bool _registered = false;
  String? _busy;

  @override
  void dispose() {
    for (final controller in [_rpcUrl, _indexerUrl, _recipient, _amount]) {
      controller.dispose();
    }
    super.dispose();
  }

  Future<void> _run(String label, Future<String?> Function() step) async {
    setState(() => _busy = label);
    final started = DateTime.now();
    try {
      final detail = await step();
      final elapsed = DateTime.now().difference(started).inMilliseconds;
      _record('$label: ${detail ?? 'done'} ($elapsed ms)');
    } catch (error) {
      _record('$label failed: $error');
    } finally {
      if (mounted) setState(() => _busy = null);
    }
  }

  void _record(String line) {
    if (mounted) setState(() => _log.insert(0, line));
  }

  TestClusterRpc get _rpc => TestClusterRpc(_rpcUrl.text.trim());

  Future<String?> _open() async {
    final signer = _signer ?? await DemoSigner.generate();
    final keys = Directory(
      '${(await getApplicationSupportDirectory()).path}/proving-keys',
    );
    final url = Uri.parse(_indexerUrl.text.trim());
    final wallet = await ZolanaWallet.open(
      signer: signer,
      config: WalletConfig(
        rpcUrl: _rpcUrl.text.trim(),
        indexerUrl: url.toString(),
        provingKeyDir: keys.path,
        // Test clusters behind an emulator's host alias are plain HTTP.
        allowInsecureHttp: url.scheme == 'http',
      ),
    );
    _signer = signer;
    _wallet = wallet;
    _registered = await wallet.isRegistered();
    await _refreshPublic();
    return signer.publicKey;
  }

  Future<void> _refreshPublic() async {
    final signer = _signer;
    if (signer == null) return;
    final lamports = await _rpc.balance(signer.publicKey);
    if (mounted) setState(() => _publicLamports = lamports);
  }

  Future<String?> _airdrop() async {
    await _rpc.airdrop(_signer!.publicKey, BigInt.from(2 * _lamportsPerSol));
    await _refreshPublic();
    return '2 SOL';
  }

  Future<String?> _register() async {
    final signature = await _wallet!.register();
    setState(() => _registered = true);
    await _refreshPublic();
    return signature ?? 'already registered';
  }

  Future<String?> _deposit() async {
    final signature = await _wallet!.deposit(_lamports());
    await _refreshPublic();
    return signature;
  }

  Future<String?> _sync() async {
    final summary = await _wallet!.sync();
    setState(() => _privateLamports = summary.privateLamports);
    return '${summary.storedUtxos} notes';
  }

  Future<String?> _send() async {
    final recipient = _recipient.text.trim();
    final signature = await _wallet!.transfer(
      recipient: recipient,
      lamports: _lamports(),
    );
    await _sync();
    return signature;
  }

  BigInt _lamports() {
    final sol = double.parse(_amount.text.trim());
    return BigInt.from((sol * _lamportsPerSol).round());
  }

  @override
  Widget build(BuildContext context) {
    final wallet = _wallet;
    final idle = _busy == null;
    return Scaffold(
      appBar: AppBar(title: const Text('Private wallet')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          TextField(
            controller: _rpcUrl,
            enabled: wallet == null,
            decoration: const InputDecoration(labelText: 'Solana RPC'),
          ),
          TextField(
            controller: _indexerUrl,
            enabled: wallet == null,
            decoration: const InputDecoration(labelText: 'Zolana indexer'),
          ),
          const SizedBox(height: 12),
          Text(
            'Demo signer: an in-memory key for test clusters. A real app '
            'signs through its keystore or wallet adapter instead.',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: 12),
          if (wallet == null)
            FilledButton(
              onPressed: idle ? () => _run('Open wallet', _open) : null,
              child: const Text('Create demo key and open wallet'),
            )
          else ...[
            SelectableText('Account ${wallet.solanaPublicKey}'),
            Text(
              'Public ${_sol(_publicLamports)} · Private ${_sol(_privateLamports)}'
              '${_registered ? '' : ' · not registered'}',
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                OutlinedButton(
                  onPressed: idle ? () => _run('Airdrop', _airdrop) : null,
                  child: const Text('Airdrop 2 SOL'),
                ),
                OutlinedButton(
                  onPressed: idle && !_registered
                      ? () => _run('Register', _register)
                      : null,
                  child: const Text('Register'),
                ),
                OutlinedButton(
                  onPressed: idle ? () => _run('Deposit', _deposit) : null,
                  child: const Text('Deposit amount'),
                ),
                OutlinedButton(
                  onPressed: idle ? () => _run('Sync', _sync) : null,
                  child: const Text('Sync'),
                ),
              ],
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _recipient,
              decoration: const InputDecoration(
                labelText: 'Recipient Solana account (registered)',
              ),
            ),
            TextField(
              controller: _amount,
              keyboardType: const TextInputType.numberWithOptions(
                decimal: true,
              ),
              decoration: const InputDecoration(labelText: 'Amount (SOL)'),
            ),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: idle ? () => _run('Private transfer', _send) : null,
              icon: const Icon(Icons.lock),
              label: const Text('Prove on device and send'),
            ),
          ],
          if (_busy != null) ...[
            const SizedBox(height: 16),
            Row(
              children: [
                const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
                const SizedBox(width: 12),
                Text('$_busy…'),
              ],
            ),
          ],
          const Divider(height: 32),
          for (final line in _log)
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: SelectableText(line),
            ),
        ],
      ),
    );
  }
}

String _sol(BigInt? lamports) {
  if (lamports == null) return '–';
  return '${(lamports.toDouble() / _lamportsPerSol).toStringAsFixed(3)} SOL';
}
