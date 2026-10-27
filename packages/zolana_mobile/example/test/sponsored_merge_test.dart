import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/sponsored_merge.dart';
import 'package:zolana_mobile_demo/spend_checks.dart';
import 'package:zolana_mobile_demo/encoding.dart';

void main() {
  test(
    'proxy auth is scoped, ciphertext is preserved, redirects are not followed',
    () async {
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      final seen = <({String path, String? auth, List<int> body})>[];
      server.listen((request) async {
        final body = await request.fold<List<int>>(
          [],
          (bytes, chunk) => bytes..addAll(chunk),
        );
        seen.add((
          path: request.uri.path,
          auth: request.headers.value('x-sf-auth'),
          body: body,
        ));
        if (request.uri.path.endsWith('/redirect')) {
          request.response.statusCode = 302;
          request.response.headers.set('location', '/unexpected');
        } else {
          request.response.headers.set('zolana-tee', '1');
          request.response.add([8, 9, 10]);
        }
        await request.response.close();
      });
      final backend = SponsoredBackend(
        Uri.parse('http://127.0.0.1:${server.port}/'),
        'example-auth',
      );
      addTearDown(() async {
        backend.close();
        await server.close(force: true);
      });
      Future<TransportResponse> send(String path) => backend.transport(
        TransportRequest(
          method: 'POST',
          url: 'http://127.0.0.1:${server.port}$path',
          headers: {
            'zolana-tee': '1',
            'zolana-tee-enc': 'opaque',
            'x-sync': 'true',
          },
          body: Uint8List.fromList([1, 2, 3]),
          timeoutMs: 1000,
        ),
      );
      final proof = await send('/v1/zolana/prove/merge_8_1');
      expect(proof.body, [8, 9, 10]);
      expect(proof.headers['zolana-tee'], '1');
      await send('/rpc');
      final redirect = await send('/v1/zolana/redirect');
      expect(redirect.status, 302);
      expect(seen.map((r) => r.auth), ['example-auth', null, 'example-auth']);
      expect(seen.every((r) => r.body.join(',') == '1,2,3'), isTrue);
      expect(seen.any((r) => r.path == '/unexpected'), isFalse);
    },
  );

  test(
    'queued status GET keeps auth, encrypted headers and encoded job ID',
    () async {
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      server.listen((request) async {
        expect(request.method, 'GET');
        expect(
          request.uri.toString(),
          '/v1/zolana/prove/merge_24_1/status?jobId=job%2F1',
        );
        expect(request.headers.value('x-sf-auth'), 'example-auth');
        expect(request.headers.value('zolana-tee-enc'), 'enc');
        expect(request.headers.value('zolana-tee-ciphertext'), '010203');
        expect(
          await request.fold<List<int>>([], (all, chunk) => all..addAll(chunk)),
          isEmpty,
        );
        request.response.headers.set('zolana-tee', 'v1');
        request.response.add([4, 5, 6]);
        await request.response.close();
      });
      final backend = SponsoredBackend(
        Uri.parse('http://127.0.0.1:${server.port}/'),
        'example-auth',
      );
      addTearDown(() async {
        backend.close();
        await server.close(force: true);
      });
      final response = await backend.transport(
        TransportRequest(
          method: 'GET',
          url: '${backend.proverUrl}/prove/merge_24_1/status?jobId=job%2F1',
          headers: {
            'zolana-tee': 'v1',
            'zolana-tee-enc': 'enc',
            'zolana-tee-ciphertext': '010203',
          },
          body: Uint8List(0),
          timeoutMs: 1000,
        ),
      );
      expect(response.status, 200);
      expect(response.headers['zolana-tee'], 'v1');
      expect(response.body, [4, 5, 6]);
    },
  );

  test('merge retries persist state before every relayer handoff', () async {
    final store = _Store();
    final backend = _Backend();
    final wallet = _Wallet();
    final flow = SponsoredFlow(
      wallet,
      _UnusedSigner(),
      backend,
      store,
      _UnusedChecks(),
    );
    await flow.start(rounds: 1, treeId: 0);
    store.failWrites = true;
    for (var retry = 0; retry < 2; retry++) {
      await expectLater(flow.mergeRounds(), throwsStateError);
      expect(backend.relays, 0);
    }
    store.failWrites = false;
    await flow.mergeRounds();
    expect(wallet.prepares, 1);
    expect(backend.relays, 1);
    expect(store.data!['phase'], 'ready');
  });

  test('recovery resends saved bytes and retains state until completion is durable', () async {
    for (final replayFails in [false, true]) {
      final store = _Store()
        ..data = {
          'version': 1,
          'backend': 'https://backend.example/',
          'phase': 'submitted',
          'merges': [],
          'payment': 'payment-snapshot',
          'paymentSignature': base58Encode(List.filled(64, 7)),
        };
      final wallet = _Wallet()
        ..unknown = true
        ..replayFails = replayFails;
      final flow = SponsoredFlow(
        wallet,
        _UnusedSigner(),
        _Backend(),
        store,
        _UnusedChecks(),
      );
      await flow.restore();
      store.failCompletion = true;
      await expectLater(flow.reconcilePayment(), throwsStateError);
      expect(flow.complete, isFalse);
      expect(store.data!['phase'], 'submitted');
      expect(wallet.submissions.single, List.filled(64, 7));
      expect(wallet.prepares, 0);
      store.failCompletion = false;
      await flow.reconcilePayment();
      expect(flow.complete, isTrue);
      expect(store.data!['phase'], 'complete');
    }
  });

  test('run state survives restart and rejects tampered ciphertext', () async {
    final directory = await Directory.systemTemp.createTemp(
      'sponsored-state-test',
    );
    addTearDown(() => directory.delete(recursive: true));
    final file = File('${directory.path}/run');
    final key = SecretKey(List.filled(32, 7));
    final store = SponsoredStore(file, key);
    final state = {
      'phase': 'submitted',
      'paymentSignature': 'saved-before-submission',
      'amount': '123456789',
    };
    await store.write(state);
    expect(await SponsoredStore(file, key).read(), state);
    expect(
      utf8.decode(await file.readAsBytes(), allowMalformed: true),
      isNot(contains('saved-before-submission')),
    );
    final bytes = await file.readAsBytes();
    bytes[20] ^= 1;
    await file.writeAsBytes(bytes);
    await expectLater(
      store.read(),
      throwsA(isA<SecretBoxAuthenticationError>()),
    );
  });
}

class _Store implements SponsoredStore {
  Map<String, dynamic>? data;
  bool failWrites = false, failCompletion = false;
  @override
  Future<Map<String, dynamic>?> read() async => data == null
      ? null
      : jsonDecode(jsonEncode(data)) as Map<String, dynamic>;
  @override
  Future<void> write(Map<String, dynamic> state) async {
    if (failWrites || (failCompletion && state['phase'] == 'complete')) {
      throw StateError('disk full');
    }
    data = jsonDecode(jsonEncode(state)) as Map<String, dynamic>;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _Backend implements SponsoredBackend {
  int relays = 0;
  @override
  Uri get base => Uri.parse('https://backend.example/');
  @override
  Future<Map<String, dynamic>> call(
    String path, [
    Map<String, dynamic>? body,
  ]) async {
    if (path == 'runs') {
      return {
        'id': 'run',
        'slots': 1,
        'target': {
          'sponsor': 'sponsor',
          'nonce': 1,
          'treeId': 0,
          'expiresAt': 2000000000,
          'maxPriorityFeeLamports': 10000,
          'computeUnitLimit': 400000,
        },
      };
    }
    expect(path, 'runs/run/merges');
    relays++;
    return {'signature': 'merge-signature'};
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _Wallet implements ZolanaWallet {
  int prepares = 0;
  bool unknown = false, replayFails = false;
  final submissions = <List<int>>[];
  @override
  String get solanaPublicKey => 'owner';
  @override
  Future<PreparedTransaction> prepareMerge({
    String? mint,
    int? maxInputs,
    String? feePayer,
    Proving? proving,
    BigInt? priorityFeeLamports,
    CacheTarget? cacheTarget,
  }) async {
    prepares++;
    return _Transaction();
  }

  @override
  Future<String> exportPreparedTransaction(
    PreparedTransaction transaction,
  ) async => 'snapshot';
  @override
  Future<PreparedTransaction> restorePreparedTransaction(
    String snapshot,
  ) async => _Transaction();
  @override
  Future<void> verifyCachedOutputs(List<CachedOutput> outputs) async {}
  @override
  Future<void> confirm(
    PreparedTransaction transaction,
    String signature,
  ) async {
    if (unknown) {
      unknown = false;
      throw WalletError.transactionOutcomeUnknown(
        signature: signature,
        message: 'not found',
      );
    }
  }

  @override
  Future<String> submit(
    PreparedTransaction transaction,
    List<Uint8List> signatures,
  ) async {
    expect(transaction.message, [1, 2, 3]);
    submissions.add(signatures.single);
    if (replayFails) throw const WalletError.notesAlreadySpent();
    return base58Encode(signatures.single);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _Transaction implements PreparedTransaction {
  @override
  Uint8List get message => Uint8List.fromList([1, 2, 3]);
  @override
  CachedOutput get cachedOutput => _CachedOutput();
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _CachedOutput implements CachedOutput {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _UnusedSigner implements SolanaSigner {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _UnusedChecks implements SpendChecks {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
