import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:http/http.dart' as http;
import 'package:zolana_mobile/zolana_mobile.dart';

import 'encoding.dart';
import 'spend_checks.dart';

/// Authentication is scoped to this backend. RPC, indexer and key downloads
/// retain the SDK transport's streaming, size limits and timeout handling.
class SponsoredBackend {
  SponsoredBackend(this.base, this.token, {http.Client? client})
    : _proxy = HttpTransport(client: client, followRedirects: false) {
    if (!base.hasAuthority ||
        base.userInfo.isNotEmpty ||
        base.hasQuery ||
        base.hasFragment ||
        !base.path.endsWith('/') ||
        !(base.scheme == 'https' ||
            (base.scheme == 'http' &&
                const {
                  'localhost',
                  '127.0.0.1',
                  '::1',
                  '10.0.2.2',
                }.contains(base.host)))) {
      throw ArgumentError(
        'Use an HTTPS backend base URL ending in / (HTTP is allowed on a local emulator)',
      );
    }
  }
  final Uri base;
  final String token;
  final _direct = HttpTransport();
  final HttpTransport _proxy;

  String get proverUrl => base.resolve('v1/zolana').toString();

  Future<TransportResponse> transport(TransportRequest request) {
    final uri = Uri.parse(request.url);
    final prover = Uri.parse(proverUrl);
    final scoped =
        uri.origin == prover.origin &&
        (uri.path == prover.path || uri.path.startsWith('${prover.path}/'));
    if (!scoped) return _direct.send(request);
    return _proxy.send(
      TransportRequest(
        method: request.method,
        url: request.url,
        headers: {...request.headers, 'x-sf-auth': token},
        body: request.body,
        timeoutMs: request.timeoutMs,
        maxResponseBytes: request.maxResponseBytes,
        downloadPath: request.downloadPath,
      ),
    );
  }

  Future<Map<String, dynamic>> call(
    String path, [
    Map<String, dynamic>? body,
  ]) async {
    final uri = base.resolve(path);
    if (uri.origin != base.origin || !uri.path.startsWith(base.path)) {
      throw ArgumentError(
        'Backend request must stay under its configured base URL',
      );
    }
    final response = await _proxy.send(
      TransportRequest(
        method: body == null ? 'GET' : 'POST',
        url: uri.toString(),
        headers: {'x-sf-auth': token, 'content-type': 'application/json'},
        body: Uint8List.fromList(
          body == null ? [] : utf8.encode(jsonEncode(body)),
        ),
        timeoutMs: 45000,
        maxResponseBytes: 1024 * 1024,
      ),
    );
    if (response.status != 200) {
      throw StateError('Backend HTTP ${response.status}');
    }
    return jsonDecode(utf8.decode(response.body)) as Map<String, dynamic>;
  }

  void close() {
    _direct.close();
    _proxy.close();
  }
}

/// Demo persistence encrypted with a wallet-derived key. Production should
/// supply device-keystore-backed storage and exclude it from cloud backups.
/// The demo's published seeds are not secrets.
class SponsoredStore {
  SponsoredStore(this.file, this.key);
  final File file;
  final SecretKey key;
  final _cipher = AesGcm.with256bits();

  static Future<SponsoredStore> forWallet(
    Directory directory,
    ZolanaWallet wallet,
  ) async {
    final keys = await wallet.exportKeys();
    final digest = await Sha256().hash([
      ...utf8.encode('zolana-example/sponsored-state/v1'),
      ...keys.viewingPrivateKey,
    ]);
    return SponsoredStore(
      File('${directory.path}/sponsored-${wallet.solanaPublicKey}.json'),
      SecretKey(digest.bytes),
    );
  }

  Future<Map<String, dynamic>?> read() async {
    if (!await file.exists()) return null;
    final box = SecretBox.fromConcatenation(
      await file.readAsBytes(),
      nonceLength: 12,
      macLength: 16,
    );
    final plain = await _cipher.decrypt(box, secretKey: key);
    return jsonDecode(utf8.decode(plain)) as Map<String, dynamic>;
  }

  Future<void> write(Map<String, dynamic> state) async {
    final box = await _cipher.encrypt(
      utf8.encode(jsonEncode(state)),
      secretKey: key,
    );
    final temporary = File('${file.path}.tmp');
    await temporary.writeAsBytes(box.concatenation(), flush: true);
    await temporary.rename(file.path);
  }
}

/// Wallet-initiated, one-cache run. The SDK awaits the proof, including encrypted
/// job polling when queued. Chain/cache confirmation is a separate step.
class SponsoredFlow {
  SponsoredFlow(
    this.wallet,
    this.signer,
    this.backend,
    this.store,
    this.checks,
  );
  final ZolanaWallet wallet;
  final SolanaSigner signer;
  final SponsoredBackend backend;
  final SponsoredStore store;
  final SpendChecks checks;
  Map<String, dynamic>? state;
  final _merges = <PreparedTransaction>[];
  PreparedTransaction? payment;

  bool get complete => state?['phase'] == 'complete';
  String get phase => state?['phase'] as String? ?? 'No active run';
  String? get paymentSignature => state?['paymentSignature'] as String?;

  Future<void> restore() async {
    state = await store.read();
    if (state == null || complete) return;
    _checkBackend();
    if (state!['version'] != 1) {
      throw StateError('Unsupported saved run version');
    }
    final restored = <PreparedTransaction>[];
    for (final merge in state!['merges'] as List) {
      restored.add(
        await wallet.restorePreparedTransaction(merge['snapshot'] as String),
      );
    }
    final snapshot = state!['payment'] as String?;
    payment = snapshot == null
        ? null
        : await wallet.restorePreparedTransaction(snapshot);
    _merges
      ..clear()
      ..addAll(restored);
  }

  Future<void> start({
    required int rounds,
    required int treeId,
    String? mint,
  }) async {
    if (state != null && !complete) {
      throw StateError('Resume the existing run first');
    }
    final run = await backend.call('runs', {
      'owner': wallet.solanaPublicKey,
      'treeId': treeId,
      'slots': rounds,
    });
    state = {
      'version': 1,
      'backend': backend.base.toString(),
      'run': run,
      'mint': mint,
      'merges': <Map<String, dynamic>>[],
      'phase': 'merging',
    };
    _merges.clear();
    payment = null;
    await _save();
  }

  CacheTarget _target(int slot) {
    final target = (state!['run'] as Map)['target'] as Map;
    return CacheTarget(
      sponsor: target['sponsor'] as String,
      nonce: BigInt.parse(target['nonce'].toString()),
      treeId: target['treeId'] as int,
      expiresAt: (target['expiresAt'] as num).toInt(),
      slot: slot,
      maxPriorityFeeLamports: BigInt.parse(
        target['maxPriorityFeeLamports'].toString(),
      ),
      computeUnitLimit: target['computeUnitLimit'] as int,
    );
  }

  Future<void> mergeRounds({int maxInputs = 8}) async {
    _checkBackend();
    final run = state!['run'] as Map;
    final rounds = run['slots'] as int;
    if (payment != null || complete) {
      throw StateError('Merging is already finished');
    }
    for (var slot = 0; slot < rounds; slot++) {
      if (_merges.length == slot) {
        final prepared = await wallet.prepareMerge(
          mint: state!['mint'] as String?,
          cacheTarget: _target(slot),
          maxInputs: maxInputs,
          proving: Proving.remote,
          priorityFeeLamports: BigInt.from(5000),
        );
        final snapshot = await wallet.exportPreparedTransaction(prepared);
        _merges.add(prepared);
        (state!['merges'] as List).add({'snapshot': snapshot});
      }
      final merge = _merges[slot];
      // Persist on every attempt, including a retry after a failed write.
      await _save();
      final result = await backend.call('runs/${run['id']}/merges', {
        'slot': slot,
        'message': base64Encode(merge.message),
      });
      (state!['merges'] as List)[slot]['signature'] = result['signature'];
      await _save();
      // Only proceed once this exact predicted output is in the confirmed
      // cache. A retry resends identical bytes through the relayer journal.
      await _waitForCache([merge.cachedOutput!]);
      // Keep merge inputs reserved until the whole payment completes. This
      // prevents the next round from selecting an earlier merged input.
    }
    state!['phase'] = 'ready';
    await _save();
  }

  Future<void> preparePayment(List<Output> outputs) async {
    if (phase != 'ready' || payment != null) {
      throw StateError('Finish merges before preparing the payment');
    }
    final cached = _merges.map((merge) => merge.cachedOutput!).toList();
    final prepared = await wallet.prepareSpend(
      outputs: outputs,
      mint: state!['mint'] as String?,
      cachedInputs: cached,
      proving: Proving.local,
      priorityFeeLamports: BigInt.from(5000),
    );
    final snapshot = await wallet.exportPreparedTransaction(prepared);
    payment = prepared;
    state!['requestedOutputs'] = outputs
        .map(
          (output) => switch (output) {
            Output_Private(:final recipient, :final amount) => {
              'kind': 'private',
              'recipient': recipient,
              'amount': amount.toString(),
            },
            Output_Public(:final recipient, :final amount) => {
              'kind': 'public',
              'recipient': recipient,
              'amount': amount.toString(),
            },
          },
        )
        .toList();
    state!['payment'] = snapshot;
    state!['phase'] = 'approval';
    await _save();
  }

  Future<void> refreshPayment() async {
    if (payment == null || paymentSignature != null || phase != 'approval') {
      throw StateError('Only an unsigned payment can be refreshed');
    }
    payment = await wallet.refresh(payment!);
    state!['payment'] = await wallet.exportPreparedTransaction(payment!);
    await _save();
  }

  Future<String> signAndSend(
    Future<bool> Function(PreparedTransaction) approve,
  ) async {
    if (paymentSignature != null) return reconcilePayment();
    final transaction = payment;
    if (transaction == null || phase != 'approval') {
      throw StateError('Prepare the payment first');
    }
    final cached = _merges.map((merge) => merge.cachedOutput!).toList();
    await wallet.verifyCachedOutputs(cached);
    final outputs = (state!['requestedOutputs'] as List).map((item) {
      final amount = BigInt.parse(item['amount'] as String);
      return item['kind'] == 'private'
          ? Output.private(
              recipient: item['recipient'] as String,
              amount: amount,
            )
          : Output.public(
              recipient: item['recipient'] as String,
              amount: amount,
            );
    }).toList();
    if (transaction.priorityFeeLamports > _target(0).maxPriorityFeeLamports) {
      throw StateError('Payment exceeds run fee limit');
    }
    await checks.verify(
      message: transaction.message,
      openings: transaction.outputs,
      sender: wallet.solanaPublicKey,
      request: SpendRequest(
        outputs: outputs,
        mint: state!['mint'] as String?,
        cacheAddress: await _target(0).cacheAddress(),
        cacheSlots: cached.map((c) => c.target.slot).toList(),
      ),
    );
    if (!await approve(transaction)) {
      throw StateError('Payment approval declined');
    }
    final signature = await signer.signMessage(
      transaction.message,
      transaction: transaction,
    );
    state!['paymentSignature'] = base58Encode(signature);
    state!['phase'] = 'submitted';
    await _save(); // BEFORE submitting the user-signed payment.
    final result = await wallet.submit(transaction, [signature]);
    await _completed();
    return result;
  }

  Future<String> reconcilePayment() async {
    final signature = paymentSignature;
    if (signature == null || payment == null) {
      throw StateError('No submitted payment to reconcile');
    }
    // A process may stop after saving the signature but before submitting it.
    // Retry only those identical signed bytes; never refresh an unknown send.
    await _save();
    try {
      await wallet.confirm(payment!, signature);
    } on WalletError_TransactionOutcomeUnknown {
      try {
        await wallet.submit(payment!, [base58Decode(signature)]);
      } catch (_) {
        // A replay can fail preflight after the original landed. Reconcile
        // the original signature before drawing any conclusion from that error.
        await wallet.confirm(payment!, signature);
      }
    }
    await _completed();
    return signature;
  }

  Future<void> _completed() async {
    final completed = {...state!, 'phase': 'complete'};
    await store.write(completed);
    state = completed;
    for (final merge in _merges) {
      await wallet.release(merge);
    }
  }

  Future<void> _save() => store.write(state!);
  void _checkBackend() {
    if (state == null || state!['backend'] != backend.base.toString()) {
      throw StateError('Run belongs to another backend');
    }
  }

  Future<void> _waitForCache(List<CachedOutput> outputs) async {
    Object? last;
    for (var attempt = 0; attempt < 30; attempt++) {
      try {
        await wallet.verifyCachedOutputs(outputs);
        return;
      } catch (error) {
        last = error;
      }
      await Future<void>.delayed(const Duration(seconds: 1));
    }
    throw StateError('Cache not ready; run retained for retry. $last');
  }
}
