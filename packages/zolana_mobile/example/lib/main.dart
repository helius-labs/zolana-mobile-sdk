import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

import 'wallet_screen.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initZolanaMobile();
  runApp(const ZolanaMobileDemo());
}

class ZolanaMobileDemo extends StatelessWidget {
  const ZolanaMobileDemo({super.key});

  @override
  Widget build(BuildContext context) {
    ThemeData theme(Brightness brightness) => ThemeData(
      colorScheme: ColorScheme.fromSeed(
        seedColor: Colors.black,
        brightness: brightness,
        dynamicSchemeVariant: DynamicSchemeVariant.monochrome,
      ),
      useMaterial3: true,
    );
    return MaterialApp(
      title: 'Zolana',
      debugShowCheckedModeBanner: false,
      theme: theme(Brightness.light),
      darkTheme: theme(Brightness.dark),
      home: const WalletScreen(),
    );
  }
}

class DemoScreen extends StatefulWidget {
  const DemoScreen({super.key});

  @override
  State<DemoScreen> createState() => _DemoScreenState();
}

class _DemoScreenState extends State<DemoScreen> with WidgetsBindingObserver {
  String _sdk = 'loading';
  LocalProofResult? _proof;
  String? _error;
  bool _proving = false;
  bool _locked = false;
  bool _unlocking = false;
  int _epoch = 0;
  LocalProver? _prover;
  BigInt? _loadMs;
  Directory? _assetDirectory;
  Future<void>? _initializing;
  Future<void> _draining = Future.value();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _initializing = _initialize();
  }

  Future<void> _initialize() async {
    final epoch = _epoch;
    try {
      final version = await sdkVersion();
      final directory = await Directory.systemTemp.createTemp('zolana-demo-');
      _assetDirectory = directory;
      final r1cs = await _copyAsset(
        'transfer_confidential_2_3.r1cs',
        directory,
      );
      final vk = await _copyAsset('transfer_confidential_2_3.vk', directory);
      final pk = await _copyAsset('transfer_confidential_2_3.pk', directory);
      final prover = await LocalProver.load(
        r1csPath: r1cs,
        provingKeyPath: pk,
        verifyingKeyPath: vk,
      );
      if (!mounted || epoch != _epoch || _locked) {
        await prover.close();
        return;
      }
      setState(() {
        _sdk = version;
        _prover = prover;
        _loadMs = prover.loadMs;
      });
    } catch (error) {
      if (epoch == _epoch) _showError(error);
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed && !_locked) _lock();
  }

  void _lock() {
    _epoch++;
    _locked = true;
    final prover = _prover;
    _prover = null;
    _proof = null;
    _proving = false;
    final closing = prover?.close() ?? Future<void>.value();
    _draining = Future.wait([closing, _initializing ?? Future<void>.value()])
        .then((_) async {
          final directory = _assetDirectory;
          if (directory != null) await directory.delete(recursive: true);
          _assetDirectory = null;
        });
    unawaited(
      _draining.catchError((Object error) {
        _showError(error);
      }),
    );
    if (mounted) setState(() {});
  }

  Future<void> _unlock() async {
    if (_unlocking) return;
    setState(() => _unlocking = true);
    try {
      await _draining;
      if (!mounted) return;
      setState(() {
        _locked = false;
        _error = null;
      });
      _initializing = _initialize();
      await _initializing;
    } catch (error) {
      _showError(error);
    } finally {
      if (mounted) setState(() => _unlocking = false);
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    if (!_locked) {
      _locked = true;
      _epoch++;
      final closing = _prover?.close() ?? Future<void>.value();
      unawaited(
        Future.wait([closing, _initializing ?? Future<void>.value()])
            .then((_) async {
              await _assetDirectory?.delete(recursive: true);
            })
            .catchError((Object _) {}),
      );
    }
    super.dispose();
  }

  Future<void> _prove() async {
    final prover = _prover;
    final epoch = _epoch;
    if (prover == null || _locked) return;
    setState(() {
      _proving = true;
      _error = null;
    });
    try {
      final request = await rootBundle.loadString(
        'assets/proving/prove-request-2x3.json',
      );
      if (!mounted || epoch != _epoch || _locked) return;
      final proof = await prover.proveRequest(request).result;
      if (!mounted || epoch != _epoch || _locked) return;
      setState(() => _proof = proof);
    } catch (error) {
      if (epoch == _epoch) _showError(error);
    } finally {
      if (mounted && epoch == _epoch) setState(() => _proving = false);
    }
  }

  Future<String> _copyAsset(String name, Directory directory) async {
    final data = await rootBundle.load('assets/proving/$name');
    final file = File('${directory.path}/$name');
    await file.writeAsBytes(data.buffer.asUint8List(), flush: true);
    return file.path;
  }

  void _showError(Object error) {
    if (!mounted) return;
    setState(
      () => _error = error is ProverException
          ? error.toString()
          : 'The prover operation failed.',
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Proof benchmark'),
        actions: [
          TextButton(
            onPressed: _unlocking ? null : (_locked ? _unlock : _lock),
            child: Text(_locked ? 'Unlock demo' : 'Lock / discard'),
          ),
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Center(child: Text('SDK $_sdk')),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _DemoCard(
            title: 'Local Groth16 proof',
            subtitle: 'Prove a public 2→3 request fixture with Mopro/gnark. Locking discards results; native work drains before keys are released.',
            buttonLabel: 'Generate proof locally',
            busy: _proving,
            onPressed: _locked || _prover == null ? null : _prove,
            child: _proof == null ? null : _ProofResult(proof: _proof!),
          ),
          if (_loadMs != null)
            Text('Key loading: $_loadMs ms (once per session)'),
          if (_error != null) ...[
            const SizedBox(height: 16),
            Text(
              _error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ],
        ],
      ),
    );
  }
}

class _DemoCard extends StatelessWidget {
  const _DemoCard({
    required this.title,
    required this.subtitle,
    required this.buttonLabel,
    required this.busy,
    required this.onPressed,
    this.child,
  });

  final String title;
  final String subtitle;
  final String buttonLabel;
  final bool busy;
  final VoidCallback? onPressed;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 6),
            Text(subtitle),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: busy ? null : onPressed,
              icon: busy
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.play_arrow),
              label: Text(busy ? 'Working…' : buttonLabel),
            ),
            if (child != null) ...[const SizedBox(height: 18), child!],
          ],
        ),
      ),
    );
  }
}

class _ProofResult extends StatelessWidget {
  const _ProofResult({required this.proof});

  final LocalProofResult proof;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _Detail(label: 'Verified', value: proof.verified ? 'yes' : 'no'),
        _Detail(label: 'Circuit', value: '${proof.inputs}→${proof.outputs}'),
        _Detail(label: 'Mopro proving', value: '${proof.proofMs} ms'),
        _Detail(label: 'Witness preparation', value: '${proof.witnessMs} ms'),
        _Detail(label: 'Verification', value: '${proof.verifyMs} ms'),
        _Detail(label: 'Proof JSON', value: _short(proof.proofJson)),
      ],
    );
  }
}

class _Detail extends StatelessWidget {
  const _Detail({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Text('$label: $value'),
    );
  }
}

String _short(String value) {
  if (value.length <= 24) return value;
  return '${value.substring(0, 12)}…${value.substring(value.length - 8)}';
}
