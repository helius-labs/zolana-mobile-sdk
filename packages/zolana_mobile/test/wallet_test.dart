import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:zolana_mobile/src/http_transport.dart';
import 'package:zolana_mobile/src/rust/third_party/zolana_mobile.dart'
    show MobileWallet, PendingTransaction, TransportOutcome;
import 'package:zolana_mobile/zolana_mobile.dart';

const owner = 'Owner1111111111111111111111111111111111111';
const config = WalletConfig(
  rpcUrl: 'https://rpc.example',
  indexerUrl: 'https://indexer.example',
  provingKeyDir: '/keys',
  mints: [],
);

final savedKeys = WalletKeys(
  viewingPrivateKey: Uint8List.fromList([1]),
  viewingPublicKey: Uint8List.fromList([2]),
  nullifierPrivateKey: Uint8List.fromList([3]),
  nullifierPublicKey: Uint8List.fromList([4]),
);

/// Declines every transaction; opening only needs the derivation message.
class DecliningSigner extends RecordingSigner {
  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    PreparedTransaction? transaction,
  }) async {
    if (requests.isNotEmpty) throw StateError('declined');
    return super.signMessage(message, transaction: transaction);
  }
}

class RecordingSigner implements SolanaSigner {
  final requests = <(Uint8List, PreparedTransaction?)>[];

  @override
  String get publicKey => owner;

  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    PreparedTransaction? transaction,
  }) async {
    requests.add((message, transaction));
    return Uint8List.fromList([...message.reversed]);
  }
}

class FakePending implements PendingTransaction {
  FakePending(this.bytes, {this.signerKeys = const [owner], this.height = 100});

  final Uint8List bytes;
  final List<String> signerKeys;
  final int height;

  @override
  Future<BigInt> lastValidBlockHeight() async => BigInt.from(height);

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;

  @override
  Future<PendingTransactionKind> kind() async =>
      PendingTransactionKind.transfer;

  @override
  Future<Uint8List> messageBytes() async => bytes;

  @override
  Future<List<String>> signers() async => signerKeys;

  @override
  Future<BigInt?> amount() async => BigInt.from(bytes.first);

  @override
  Future<String?> mint() async => null;

  @override
  Future<String?> recipient() async => 'Recipient';

  @override
  Future<String> feePayer() async => signerKeys.first;
}

class FakeWallet implements MobileWallet {
  final submitted = <(PendingTransaction, List<Uint8List>)>[];
  final released = <PendingTransaction>[];
  final waited = <String>[];
  final confirmed = <(PendingTransaction, String)>[];
  final events = <String>[];
  Completer<void>? holdBalances;
  Completer<void>? holdTransfer;
  Completer<void>? holdRegistration;
  Object? transferError;
  Object? shieldedAddressError;
  Object? disposeError;
  bool disposed = false;
  List<String> transferSigners = const [owner];
  String? transferFeePayer;
  final proved = <Proving?>[];

  @override
  Future<String> shieldedAddress() async {
    if (shieldedAddressError case final error?) throw error;
    return 'shielded';
  }

  @override
  Future<WalletKeys> exportKeys() async => savedKeys;

  @override
  Future<void> release({required PendingTransaction pending}) async =>
      released.add(pending);

  @override
  Future<void> waitForTransaction({required String signature}) async =>
      waited.add(signature);

  /// A new blockhash: new message bytes and a later height, same signers.
  @override
  Future<PendingTransaction> refresh({
    required PendingTransaction pending,
  }) async {
    final old = pending as FakePending;
    return FakePending(
      Uint8List.fromList([...old.bytes, 0]),
      signerKeys: old.signerKeys,
      height: old.height + 150,
    );
  }

  RegistrationStatus status = RegistrationStatus.registered;

  @override
  Future<RegistrationStatus> registrationStatus() async => status;

  @override
  Future<List<TokenBalance>> balances() async {
    events.add('balances start');
    await holdBalances?.future;
    events.add('balances end');
    return const [];
  }

  @override
  Future<BigInt> privateBalance({String? mint}) async => BigInt.two;

  @override
  Future<List<ActivityEntry>> activity() async => const [];

  @override
  Future<PendingTransaction?> prepareRegistration() async {
    await holdRegistration?.future;
    return switch (status) {
      RegistrationStatus.registered => null,
      RegistrationStatus.conflict =>
        throw const WalletError.registrationConflict(owner: owner),
      RegistrationStatus.notRegistered => FakePending(Uint8List.fromList([3])),
    };
  }

  @override
  Future<PendingTransaction> prepareDeposit({
    String? mint,
    required BigInt amount,
  }) async => FakePending(Uint8List.fromList([1, 2]));

  @override
  Future<PendingTransaction> prepareTransfer({
    required String recipient,
    String? mint,
    required BigInt amount,
    String? feePayer,
    Proving? proving,
  }) async {
    events.add('transfer ${mint ?? 'SOL'}');
    transferFeePayer = feePayer;
    proved.add(proving);
    await holdTransfer?.future;
    final error = transferError;
    if (error != null) throw error;
    return FakePending(Uint8List.fromList([7, 8]), signerKeys: transferSigners);
  }

  @override
  Future<PendingTransaction> prepareWithdrawal({
    required String recipient,
    String? mint,
    required BigInt amount,
    String? feePayer,
    Proving? proving,
  }) async {
    proved.add(proving);
    return FakePending(Uint8List.fromList([9]));
  }

  @override
  Future<String> submit({
    required PendingTransaction pending,
    required List<Uint8List> signatures,
  }) async {
    submitted.add((pending, signatures));
    return 'signature';
  }

  @override
  Future<void> confirm({
    required PendingTransaction pending,
    required String signature,
  }) async {
    confirmed.add((pending, signature));
  }

  @override
  void dispose() {
    events.add('dispose');
    disposed = true;
    if (disposeError case final error?) throw error;
  }

  @override
  bool get isDisposed => disposed;
}

class FakeBackend implements WalletBackend {
  FakeBackend(this.wallet);

  final FakeWallet wallet;
  WalletConfig? config;
  Uint8List? openedWith;
  WalletKeys? openedWithKeys;
  NativeTransport? transport;

  @override
  Future<Uint8List> derivationMessage(String solanaPubkey) async =>
      Uint8List.fromList(solanaPubkey.codeUnits);

  @override
  Future<MobileWallet> open(
    WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
    NativeTransport transport,
  ) async {
    this.config = config;
    openedWith = derivationSignature;
    this.transport = transport;
    return wallet;
  }

  @override
  Future<MobileWallet> openWithKeys(
    WalletConfig config,
    String solanaPubkey,
    WalletKeys keys,
    NativeTransport transport,
  ) async {
    this.config = config;
    openedWithKeys = keys;
    this.transport = transport;
    return wallet;
  }
}

Future<(ZolanaWallet, FakeWallet, RecordingSigner, FakeBackend)>
openWallet() async {
  final native = FakeWallet();
  final backend = FakeBackend(native);
  final signer = RecordingSigner();
  final wallet = await ZolanaWallet.open(
    config: config,
    signer: signer,
    backend: backend,
  );
  return (wallet, native, signer, backend);
}

Matcher throwsWalletError(WalletError error) => throwsA(
  isA<ZolanaWalletException>().having((e) => e.error, 'error', error),
);

void main() {
  test('opens from a signature over the derivation message', () async {
    final (wallet, _, signer, backend) = await openWallet();

    expect(wallet.shieldedAddress, 'shielded');
    expect(signer.requests.single.$1, Uint8List.fromList(owner.codeUnits));
    expect(signer.requests.single.$2, isNull);
    expect(
      backend.openedWith,
      Uint8List.fromList(owner.codeUnits.reversed.toList()),
    );
  });

  test('hands the application transport to the native wallet', () async {
    final requests = <TransportRequest>[];
    Future<TransportResponse> transport(TransportRequest request) async {
      requests.add(request);
      return TransportResponse(
        status: 200,
        headers: const {},
        body: Uint8List.fromList([7]),
      );
    }

    final request = TransportRequest(
      method: 'POST',
      url: 'https://rpc.example',
      headers: const {'x-token': 'token'},
      body: Uint8List.fromList([1]),
    );
    final opened = FakeBackend(FakeWallet());
    await ZolanaWallet.open(
      config: config,
      signer: RecordingSigner(),
      transport: transport,
      backend: opened,
    );
    final outcome = await opened.transport!(request);
    expect(outcome.response!.status, 200);
    expect(outcome.response!.body, [7]);
    expect(outcome.failure, isNull);

    final reopened = FakeBackend(FakeWallet());
    await ZolanaWallet.openWithKeys(
      config: config,
      solanaPublicKey: owner,
      keys: savedKeys,
      transport: transport,
      backend: reopened,
    );
    await reopened.transport!(request);
    expect(requests, [request, request]);
  });

  test('hands the wallet a transport failure as its message only', () async {
    final request = TransportRequest(
      method: 'POST',
      url: 'https://rpc.example/?api-key=secret',
      headers: const {},
      body: Uint8List(0),
    );
    Future<TransportOutcome> failing(Object error) async {
      final backend = FakeBackend(FakeWallet());
      await ZolanaWallet.open(
        config: config,
        signer: RecordingSigner(),
        transport: (_) async => throw error,
        backend: backend,
      );
      return backend.transport!(request);
    }

    final outcome = await failing(StateError('no network'));
    expect(outcome.response, isNull);
    // No stack trace: the message is all the wallet gets.
    expect(outcome.failure!.message, 'Bad state: no network');
    expect(outcome.failure!.status, isNull);
    expect((await failing(_Unprintable())).failure!.message, 'transport error');

    // The status arrived: the wallet does not send the request again.
    final lost = await failing(
      const TransportResponseLost(200, 'Connection reset by peer'),
    );
    expect(lost.response, isNull);
    expect(lost.failure!.message, 'Connection reset by peer');
    expect(lost.failure!.status, 200);
  });

  test(
    'hands the default transport to the native wallet without one',
    () async {
      final (wallet, _, _, backend) = await openWallet();
      final transport = backend.transport!;

      // Closing the wallet closes its connection pool: the request fails
      // before any connection, as a message.
      await wallet.close();
      final outcome = await transport(
        TransportRequest(
          method: 'POST',
          url: 'https://rpc.example',
          headers: const {},
          body: Uint8List(0),
        ),
      );
      expect(outcome.response, isNull);
      expect(outcome.failure!.message, contains('closed'));
    },
  );

  test('refuses plaintext URLs at open with the default transport', () async {
    Future<FakeBackend> open(
      WalletConfig config, {
      bool allowInsecureHttp = false,
      ZolanaTransport? transport,
      RecordingSigner? signer,
    }) async {
      final backend = FakeBackend(FakeWallet());
      await ZolanaWallet.open(
        config: config,
        signer: signer ?? RecordingSigner(),
        transport: transport,
        allowInsecureHttp: allowInsecureHttp,
        backend: backend,
      );
      return backend;
    }

    WalletConfig withUrls({
      String? rpc,
      String? indexer,
      String? keys,
      String? prover,
    }) => WalletConfig(
      rpcUrl: rpc ?? config.rpcUrl,
      indexerUrl: indexer ?? config.indexerUrl,
      provingKeyDir: config.provingKeyDir,
      provingKeyUrl: keys,
      proverUrl: prover,
      mints: const [],
    );
    const plaintext = 'http://10.0.2.2:8899/?api-key=secret';
    // The error names the URL without its key.
    const reported = 'http://10.0.2.2:8899/?api-key=redacted';
    for (final (insecure, error) in [
      (withUrls(rpc: plaintext), WalletError.rpcUrlInsecure(url: reported)),
      (
        withUrls(indexer: plaintext),
        WalletError.indexerUrlInsecure(url: reported),
      ),
      (
        withUrls(keys: plaintext),
        WalletError.provingKeyUrlInsecure(url: reported),
      ),
      (
        withUrls(prover: plaintext),
        WalletError.proverUrlInsecure(url: plaintext),
      ),
    ]) {
      final signer = RecordingSigner();
      await expectLater(
        open(insecure, signer: signer),
        throwsWalletError(error),
      );
      // Before the signer is asked or the native wallet opens.
      expect(signer.requests, isEmpty);
      await open(insecure, allowInsecureHttp: true);
      // An application transport applies its own policy.
      await open(
        insecure,
        transport: (_) async =>
            TransportResponse(status: 200, headers: const {}, body: Uint8List(0)),
      );
    }
    await expectLater(
      ZolanaWallet.openWithKeys(
        config: withUrls(indexer: plaintext),
        solanaPublicKey: owner,
        keys: savedKeys,
        backend: FakeBackend(FakeWallet()),
      ),
      throwsWalletError(WalletError.indexerUrlInsecure(url: reported)),
    );
    await open(
      withUrls(
        rpc: 'http://localhost:8899',
        indexer: 'http://127.0.0.1:8784/v1/zolana',
        keys: 'http://[::1]:9000',
        prover: 'http://127.0.0.1:3001',
      ),
    );
  });

  test(
    'hands the prover URL and proving default to the native wallet',
    () async {
      const remote = WalletConfig(
        rpcUrl: 'https://rpc.example',
        indexerUrl: 'https://indexer.example',
        provingKeyDir: '/keys',
        proving: Proving.remote,
        proverUrl: 'https://backend.example/zolana?api-key=secret',
        mints: [],
      );
      final opened = FakeBackend(FakeWallet());
      await ZolanaWallet.open(
        config: remote,
        signer: RecordingSigner(),
        backend: opened,
      );
      final reopened = FakeBackend(FakeWallet());
      await ZolanaWallet.openWithKeys(
        config: remote,
        solanaPublicKey: owner,
        keys: savedKeys,
        backend: reopened,
      );
      for (final backend in [opened, reopened]) {
        expect(backend.config!.proverUrl, remote.proverUrl);
        expect(backend.config!.proving, Proving.remote);
      }
      expect(config.proverUrl, isNull);
    },
  );

  test('a failed open releases the native wallet and its transport', () async {
    final native = FakeWallet()..shieldedAddressError = StateError('bridge');
    final backend = FakeBackend(native);
    await expectLater(
      ZolanaWallet.open(
        config: config,
        signer: RecordingSigner(),
        backend: backend,
      ),
      throwsStateError,
    );
    expect(native.disposed, isTrue);
    final outcome = await backend.transport!(
      TransportRequest(
        method: 'GET',
        url: 'https://keys.example/k.key',
        headers: const {},
        body: Uint8List(0),
      ),
    );
    expect(outcome.failure!.message, contains('closed'));
  });

  test('close releases the default transport when dispose fails', () async {
    final (wallet, native, _, backend) = await openWallet();
    native.disposeError = StateError('dispose');
    await expectLater(wallet.close(), throwsStateError);
    final outcome = await backend.transport!(
      TransportRequest(
        method: 'GET',
        url: 'https://keys.example/k.key',
        headers: const {},
        body: Uint8List(0),
      ),
    );
    expect(outcome.failure!.message, contains('closed'));
  });

  group('HttpTransport', () {
    final request = TransportRequest(
      method: 'POST',
      url: 'https://rpc.example/?api-key=secret',
      headers: const {'content-type': 'application/json'},
      body: Uint8List.fromList([1, 2]),
    );

    test('sends the request as it is and returns any status', () async {
      http.Request? sent;
      final transport = HttpTransport(
        client: MockClient((request) async {
          sent = request;
          return http.Response.bytes([9], 401);
        }),
      );
      final response = await transport.send(request);
      expect(response.status, 401);
      expect(response.body, [9]);
      expect(sent!.method, 'POST');
      expect(sent!.url.toString(), request.url);
      expect(sent!.headers, request.headers);
      expect(sent!.bodyBytes, [1, 2]);
      expect(sent!.followRedirects, isTrue);
    });

    test('stops reading a body past maxResponseBytes', () async {
      TransportRequest key(int? max) => TransportRequest(
        method: 'GET',
        url: 'https://keys.example/k.key',
        headers: const {},
        body: Uint8List(0),
        maxResponseBytes: max,
      );
      final declared = HttpTransport(
        client: MockClient(
          (_) async => http.Response.bytes([1, 2, 3, 4, 5], 200),
        ),
      );
      // Without a content length, it counts what arrives.
      final chunks = <List<int>>[];
      final streamed = HttpTransport(
        client: MockClient.streaming(
          (_, _) async => http.StreamedResponse(
            Stream.fromIterable([
              [1, 2, 3],
              [4, 5],
            ]).map((chunk) {
              chunks.add(chunk);
              return chunk;
            }),
            200,
          ),
        ),
      );
      for (final transport in [declared, streamed]) {
        expect((await transport.send(key(5))).body, [1, 2, 3, 4, 5]);
        expect((await transport.send(key(null))).body, hasLength(5));
        await expectLater(
          transport.send(key(4)),
          throwsA(
            isA<Exception>().having(
              (e) => '$e',
              'message',
              'response body exceeds 4 bytes',
            ),
          ),
        );
      }
      expect(chunks, hasLength(6));
    });

    test(
      'reports a body lost after the status, not a body too large',
      () async {
        TransportRequest get(int? max) => TransportRequest(
          method: 'GET',
          url: 'https://prover.example/prove/k/status',
          headers: const {},
          body: Uint8List(0),
          maxResponseBytes: max,
        );
        Stream<List<int>> resetAfterTwoBytes() async* {
          yield [1, 2];
          throw http.ClientException('Connection reset');
        }

        final reset = HttpTransport(
          client: MockClient.streaming(
            (_, _) async => http.StreamedResponse(resetAfterTwoBytes(), 200),
          ),
        );
        await expectLater(
          reset.send(get(null)),
          throwsA(
            isA<TransportResponseLost>()
                .having((e) => e.status, 'status', 200)
                .having(
                  (e) => '${e.cause}',
                  'cause',
                  contains('Connection reset'),
                ),
          ),
        );
        await expectLater(
          reset.send(get(1)),
          throwsA(isNot(isA<TransportResponseLost>())),
        );
      },
    );

    test('bounds a request by its timeoutMs', () async {
      final slow = HttpTransport(
        client: MockClient((_) async {
          await Future<void>.delayed(const Duration(milliseconds: 200));
          return http.Response('', 200);
        }),
      );
      TransportRequest after(int timeoutMs) => TransportRequest(
        method: 'POST',
        url: 'https://prover.example/prove',
        headers: const {},
        body: Uint8List(0),
        timeoutMs: timeoutMs,
      );
      await expectLater(slow.send(after(20)), throwsA(isA<TimeoutException>()));
      expect((await slow.send(after(2000))).status, 200);
    });

    // A proving-key download takes the same bound, 30 s without timeoutMs.
    test('fails a body that stalls, not one that keeps moving', () async {
      TransportRequest key(int timeoutMs) => TransportRequest(
        method: 'GET',
        url: 'https://keys.example/k.key',
        headers: const {},
        body: Uint8List(0),
        timeoutMs: timeoutMs,
      );
      final stalled = HttpTransport(
        client: MockClient.streaming(
          (_, _) async => http.StreamedResponse(
            // One byte, then nothing: the body never ends.
            (StreamController<List<int>>()..add([1])).stream,
            200,
            contentLength: 3,
          ),
        ),
      );
      final started = DateTime.now();
      await expectLater(
        stalled.send(key(100)),
        throwsA(
          isA<TransportResponseLost>()
              .having((e) => e.status, 'status', 200)
              .having((e) => e.cause, 'cause', isA<TimeoutException>()),
        ),
      );
      expect(DateTime.now().difference(started).inSeconds, lessThan(5));

      Stream<List<int>> slowly() async* {
        for (final byte in [1, 2, 3]) {
          await Future<void>.delayed(const Duration(milliseconds: 60));
          yield [byte];
        }
      }

      final slow = HttpTransport(
        client: MockClient.streaming(
          (_, _) async => http.StreamedResponse(slowly(), 200),
        ),
      );
      final moving = DateTime.now();
      expect((await slow.send(key(100))).body, [1, 2, 3]);
      expect(
        DateTime.now().difference(moving).inMilliseconds,
        greaterThan(100),
      );
    });

    test('isSecureUrl allows https and http to this device only', () {
      for (final url in [
        'https://keys.example/k.key',
        'http://localhost:8899/',
        'http://127.0.0.1:8784/v1/zolana',
        'http://[::1]:1/',
      ]) {
        expect(isSecureUrl(url), isTrue, reason: url);
      }
      for (final url in [
        'http://10.0.2.2:8784/v1/zolana',
        'http://localhost.evil.com/',
        'ftp://keys.example/',
        '::not a url',
      ]) {
        expect(isSecureUrl(url), isFalse, reason: url);
      }
    });
  });

  test('opens from saved keys without asking for a signature', () async {
    final native = FakeWallet();
    final backend = FakeBackend(native);
    final wallet = await ZolanaWallet.openWithKeys(
      config: config,
      solanaPublicKey: owner,
      keys: savedKeys,
      backend: backend,
    );

    expect(backend.openedWithKeys, savedKeys);
    expect(wallet.solanaPublicKey, owner);
    expect(wallet.shieldedAddress, 'shielded');
    expect(await wallet.exportKeys(), savedKeys);
    // Without a signer the application signs the prepared transactions.
    final prepared = await wallet.prepareTransfer(
      recipient: 'Recipient',
      amount: BigInt.one,
    );
    expect(prepared.signers, [owner]);
    await expectLater(
      wallet.transfer(recipient: 'Recipient', amount: BigInt.one),
      throwsWalletError(const WalletError.signerMissing()),
    );
    expect(native.submitted, isEmpty);
  });

  test('signs with the signer passed with the saved keys', () async {
    final native = FakeWallet();
    final signer = RecordingSigner();
    final wallet = await ZolanaWallet.openWithKeys(
      config: config,
      solanaPublicKey: owner,
      keys: savedKeys,
      signer: signer,
      backend: FakeBackend(native),
    );

    expect(await wallet.deposit(BigInt.one), 'signature');
    expect(signer.requests.single.$2?.amount, BigInt.one);
    await expectLater(
      ZolanaWallet.openWithKeys(
        config: config,
        solanaPublicKey: 'Other',
        keys: savedKeys,
        signer: signer,
        backend: FakeBackend(FakeWallet()),
      ),
      throwsWalletError(
        const WalletError.signerMismatch(wallet: 'Other', signer: owner),
      ),
    );
  });

  test('releases the notes of a spend that is never sent', () async {
    final native = FakeWallet();
    final wallet = await ZolanaWallet.open(
      config: config,
      signer: DecliningSigner(),
      backend: FakeBackend(native),
    );

    await expectLater(
      wallet.transfer(recipient: 'Recipient', amount: BigInt.one),
      throwsA(isA<StateError>()),
    );
    expect((native.released.single as FakePending).bytes, [7, 8]);
    expect(native.submitted, isEmpty);

    final prepared = await wallet.prepareTransfer(
      recipient: 'Recipient',
      amount: BigInt.one,
    );
    await wallet.release(prepared);
    expect(native.released, hasLength(2));
    await wallet.waitForTransaction('signature');
    expect(native.waited, ['signature']);
  });

  test('refreshes the blockhash of a prepared transaction', () async {
    final (wallet, native, _, _) = await openWallet();
    final prepared = await wallet.prepareTransfer(
      recipient: 'Recipient',
      amount: BigInt.one,
    );
    expect(prepared.lastValidBlockHeight, BigInt.from(100));

    final refreshed = await wallet.refresh(prepared);
    expect(refreshed.lastValidBlockHeight, BigInt.from(250));
    expect(refreshed.message, isNot(prepared.message));
    expect(refreshed.signers, prepared.signers);
    expect(
      (refreshed.amount, refreshed.recipient, refreshed.feePayer),
      (prepared.amount, prepared.recipient, prepared.feePayer),
    );

    await wallet.submit(refreshed, [Uint8List(64)]);
    expect(
      (native.submitted.single.$1 as FakePending).bytes,
      refreshed.message,
    );
    await wallet.close();
    await expectLater(
      wallet.refresh(prepared),
      throwsWalletError(const WalletError.walletClosed()),
    );
  });

  test('signs the proved message with its facts and submits it', () async {
    final (wallet, native, signer, _) = await openWallet();

    final signature = await wallet.transfer(
      recipient: 'Recipient',
      amount: BigInt.from(5),
    );

    expect(signature, 'signature');
    expect(signer.requests.last.$1, [7, 8]);
    expect(signer.requests.last.$2?.amount, BigInt.from(7));
    expect(native.submitted.single.$2.single, [8, 7]);
  });

  test('prepares for an application that signs and sends itself', () async {
    final (wallet, native, signer, _) = await openWallet();

    final transaction = await wallet.prepareTransfer(
      recipient: 'Recipient',
      amount: BigInt.from(5),
      mint: 'Mint',
    );
    expect(native.events, ['transfer Mint']);
    expect(transaction.kind, PendingTransactionKind.transfer);
    expect(transaction.message, [7, 8]);
    expect(transaction.signers, [owner]);
    expect(
      (
        transaction.amount,
        transaction.mint,
        transaction.recipient,
        transaction.feePayer,
      ),
      (BigInt.from(7), null, 'Recipient', owner),
    );

    await wallet.confirm(transaction, 'sent-by-the-app');
    expect(native.confirmed.single.$2, 'sent-by-the-app');
    expect(native.submitted, isEmpty);

    final signature = Uint8List.fromList([1]);
    expect(await wallet.submit(transaction, [signature]), 'signature');
    expect(native.submitted.single.$2.single, signature);
    expect(signer.requests, hasLength(1), reason: 'only the open was signed');
  });

  test('passes the fee payer to prepare, not to the signing path', () async {
    final (wallet, native, signer, _) = await openWallet();
    native.transferSigners = const ['Backend', owner];

    final transaction = await wallet.prepareTransfer(
      recipient: 'Recipient',
      amount: BigInt.one,
      feePayer: 'Backend',
    );
    expect(native.transferFeePayer, 'Backend');
    expect(transaction.signers, ['Backend', owner]);

    final signatures = [
      Uint8List.fromList([1]),
      Uint8List.fromList([2]),
    ];
    expect(await wallet.submit(transaction, signatures), 'signature');
    expect(native.submitted.single.$2, signatures);
    expect(signer.requests, hasLength(1), reason: 'only the open was signed');
  });

  test(
    'passes the proving choice of each spend, null for the config',
    () async {
      final (wallet, native, _, _) = await openWallet();

      await wallet.prepareTransfer(recipient: 'R', amount: BigInt.one);
      await wallet.prepareTransfer(
        recipient: 'R',
        amount: BigInt.one,
        proving: Proving.remote,
      );
      await wallet.prepareWithdrawal(
        recipient: 'R',
        amount: BigInt.one,
        proving: Proving.local,
      );
      await wallet.transfer(
        recipient: 'R',
        amount: BigInt.one,
        proving: Proving.remote,
      );
      await wallet.withdraw(recipient: 'R', amount: BigInt.one);
      expect(native.proved, [
        null,
        Proving.remote,
        Proving.local,
        Proving.remote,
        null,
      ]);
    },
  );

  test('refuses a transaction another key must sign', () async {
    final (wallet, native, signer, _) = await openWallet();
    native.transferSigners = const [owner, 'SomeoneElse'];

    await expectLater(
      wallet.transfer(recipient: 'Recipient', amount: BigInt.one),
      throwsWalletError(
        const WalletError.unexpectedSigners(signers: [owner, 'SomeoneElse']),
      ),
    );
    expect(signer.requests, hasLength(1), reason: 'only the open was signed');
    expect(native.submitted, isEmpty);
  });

  test('surfaces native failures as wallet exceptions', () async {
    final (wallet, native, _, _) = await openWallet();
    const unregistered = WalletError.recipientNotRegistered(
      recipient: 'Recipient',
    );
    native.transferError = unregistered;

    await expectLater(
      wallet.transfer(recipient: 'Recipient', amount: BigInt.one),
      throwsWalletError(unregistered),
    );
  });

  test('registers once and never replaces other keys', () async {
    final (wallet, native, signer, _) = await openWallet();

    native.status = RegistrationStatus.notRegistered;
    expect(await wallet.register(), 'signature');
    expect(signer.requests.last.$2?.amount, BigInt.from(3));

    native.status = RegistrationStatus.registered;
    expect(await wallet.register(), isNull);

    native.status = RegistrationStatus.conflict;
    expect(await wallet.registrationStatus(), RegistrationStatus.conflict);
    await expectLater(
      wallet.register(),
      throwsWalletError(const WalletError.registrationConflict(owner: owner)),
    );
    expect(native.submitted, hasLength(1));
  });

  test('a prepare that is proving when close is called fails', () async {
    final prepares = <Future<Object?> Function(ZolanaWallet, FakeWallet)>[
      (wallet, native) =>
          wallet.prepareTransfer(recipient: 'R', amount: BigInt.one),
      (wallet, native) {
        native.status = RegistrationStatus.notRegistered;
        return wallet.prepareRegistration();
      },
      // Registered: the native call returns no transaction.
      (wallet, native) => wallet.prepareRegistration(),
    ];
    for (final prepare in prepares) {
      final (wallet, native, _, _) = await openWallet();
      native
        ..holdTransfer = Completer()
        ..holdRegistration = Completer();
      final prepared = prepare(wallet, native);
      await Future<void>.delayed(Duration.zero);
      final closed = wallet.close();
      native.holdTransfer!.complete();
      native.holdRegistration!.complete();

      await expectLater(
        prepared,
        throwsWalletError(const WalletError.walletClosed()),
      );
      await closed;
      expect(native.disposed, isTrue);
    }
  });

  test('close drains the running step and never signs after it', () async {
    final (wallet, native, signer, _) = await openWallet();
    native.holdTransfer = Completer();

    final transfer = wallet.transfer(recipient: 'R', amount: BigInt.one);
    final queued = wallet.balances();
    await Future<void>.delayed(Duration.zero);
    final closed = wallet.close();
    expect(wallet.isClosed, isTrue);
    await Future<void>.delayed(Duration.zero);
    expect(native.disposed, isFalse, reason: 'the proof is still running');

    native.holdTransfer!.complete();
    await expectLater(
      transfer,
      throwsWalletError(const WalletError.walletClosed()),
    );
    await expectLater(
      queued,
      throwsWalletError(const WalletError.walletClosed()),
    );
    await closed;
    expect(native.events, ['transfer SOL', 'dispose']);
    expect(signer.requests, hasLength(1), reason: 'only the open was signed');
    expect(native.submitted, isEmpty);

    await expectLater(
      wallet.deposit(BigInt.one),
      throwsWalletError(const WalletError.walletClosed()),
    );
    expect(identical(wallet.close(), closed), isTrue);
  });

  test('runs operations one at a time and survives a failure', () async {
    final (wallet, native, _, _) = await openWallet();
    native.holdBalances = Completer();
    native.transferError = const WalletError.proofFailed();

    final balances = wallet.balances();
    final transfer = wallet.transfer(recipient: 'R', amount: BigInt.one);
    final deposit = wallet.deposit(BigInt.one);
    await Future<void>.delayed(Duration.zero);
    expect(native.events, ['balances start']);

    native.holdBalances!.complete();
    await balances;
    await expectLater(transfer, throwsA(isA<ZolanaWalletException>()));
    expect(await deposit, 'signature');
    expect(native.events, ['balances start', 'balances end', 'transfer SOL']);
  });
}

class _Unprintable {
  @override
  String toString() => throw StateError('unprintable');
}
