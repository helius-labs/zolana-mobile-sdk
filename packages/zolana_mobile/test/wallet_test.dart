import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:zolana_mobile/src/http_transport.dart';
import 'package:zolana_mobile/src/rust/third_party/zolana_mobile.dart'
    show MobileWallet, PendingTransaction;
import 'package:zolana_mobile/zolana_mobile.dart';

const owner = 'Owner1111111111111111111111111111111111111';
const config = WalletConfig(
  rpcUrl: 'https://rpc.example',
  indexerUrl: 'https://indexer.example',
  provingKeyDir: '/keys',
  allowInsecureHttp: false,
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
    required String purpose,
  }) async {
    if (requests.isNotEmpty) throw StateError('declined');
    return super.signMessage(message, purpose: purpose);
  }
}

class RecordingSigner implements SolanaSigner {
  final requests = <(Uint8List, String)>[];

  @override
  String get publicKey => owner;

  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    required String purpose,
  }) async {
    requests.add((message, purpose));
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
  Future<String> summary() async => 'summary of ${bytes.first}';
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
  bool disposed = false;
  List<String> transferSigners = const [owner];
  String? transferFeePayer;
  final proved = <Proving?>[];
  FutureOr<Uint8List?> Function(Uint8List)? remoteProver;

  @override
  Future<void> setRemoteProver({
    required FutureOr<Uint8List?> Function(Uint8List) prove,
  }) async => remoteProver = prove;

  @override
  Future<String> shieldedAddress() async => 'shielded';

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
  Future<BigInt> publicBalance({String? mint}) async => BigInt.one;

  @override
  Future<List<ActivityEntry>> activity() async => const [];

  @override
  Future<PendingTransaction?> prepareRegistration() async {
    await holdRegistration?.future;
    return switch (status) {
      RegistrationStatus.registered => null,
      RegistrationStatus.conflict => throw 'registration_conflict',
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
  Future<PendingTransaction?> prepareTokenAccount({
    required String owner,
    required String mint,
  }) async => null;

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
  }

  @override
  bool get isDisposed => disposed;
}

class FakeBackend implements WalletBackend {
  FakeBackend(this.wallet);

  final FakeWallet wallet;
  Uint8List? openedWith;
  WalletKeys? openedWithKeys;
  ZolanaTransport? transport;

  @override
  Future<Uint8List> derivationMessage(String solanaPubkey) async =>
      Uint8List.fromList(solanaPubkey.codeUnits);

  @override
  Future<MobileWallet> open(
    WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
    ZolanaTransport transport,
  ) async {
    openedWith = derivationSignature;
    this.transport = transport;
    return wallet;
  }

  @override
  Future<MobileWallet> openWithKeys(
    WalletConfig config,
    String solanaPubkey,
    WalletKeys keys,
    ZolanaTransport transport,
  ) async {
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

Matcher throwsWalletError(String code) => throwsA(
  isA<ZolanaWalletException>().having((e) => e.message, 'message', code),
);

void main() {
  test('opens from a signature over the derivation message', () async {
    final (wallet, _, signer, backend) = await openWallet();

    expect(wallet.shieldedAddress, 'shielded');
    expect(signer.requests.single.$1, Uint8List.fromList(owner.codeUnits));
    expect(signer.requests.single.$2, 'Open your private Zolana wallet');
    expect(
      backend.openedWith,
      Uint8List.fromList(owner.codeUnits.reversed.toList()),
    );
  });

  test('hands the application transport to the native wallet', () async {
    final requests = <TransportRequest>[];
    Future<TransportResponse> transport(TransportRequest request) async {
      requests.add(request);
      return TransportResponse(status: 200, body: Uint8List.fromList([7]));
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
    final response = await opened.transport!(request);
    expect(response.status, 200);
    expect(response.body, [7]);

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

  test('hands the default transport to the native wallet without one', () async {
    final (wallet, _, _, backend) = await openWallet();
    final transport = backend.transport!;
    final plaintext = TransportRequest(
      method: 'POST',
      url: 'http://rpc.example/?api-key=secret',
      headers: const {},
      body: Uint8List(0),
    );
    // The default transport's policy, applied before any connection.
    await expectLater(
      transport(plaintext),
      throwsA(
        isA<Exception>().having(
          (e) => '$e',
          'message',
          allOf(contains('allowInsecureHttp'), isNot(contains('secret'))),
        ),
      ),
    );

    // Closing the wallet closes its connection pool.
    await wallet.close();
    final secure = TransportRequest(
      method: 'POST',
      url: 'https://rpc.example',
      headers: const {},
      body: Uint8List(0),
    );
    await expectLater(
      transport(secure),
      throwsA(isA<http.ClientException>()),
    );
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
        allowInsecureHttp: false,
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
    });

    test('refuses plaintext off loopback unless allowed', () async {
      final urls = <String>[];
      HttpTransport transport(bool allowInsecureHttp) => HttpTransport(
        allowInsecureHttp: allowInsecureHttp,
        client: MockClient((request) async {
          urls.add(request.url.toString());
          return http.Response('', 200);
        }),
      );
      TransportRequest to(String url) => TransportRequest(
        method: 'GET',
        url: url,
        headers: const {},
        body: Uint8List(0),
      );
      const allowed = [
        'https://keys.example/k.key',
        'http://localhost:8899/',
        'http://127.0.0.1:8784/v1/zolana',
        'http://[::1]:1/',
      ];
      for (final url in allowed) {
        await transport(false).send(to(url));
      }
      expect(urls, allowed);

      const plaintext = 'http://10.0.2.2:8784/v1/zolana?api-key=secret';
      await expectLater(
        transport(false).send(to(plaintext)),
        throwsA(
          isA<Exception>().having(
            (e) => '$e',
            'message',
            allOf(
              contains('http://10.0.2.2 is plaintext off loopback'),
              isNot(contains('secret')),
            ),
          ),
        ),
      );
      expect(urls, allowed);
      await transport(true).send(to(plaintext));
      expect(urls.last, plaintext);
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
      throwsWalletError('signer_missing'),
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
    expect(signer.requests.single.$2, 'summary of 1');
    await expectLater(
      ZolanaWallet.openWithKeys(
        config: config,
        solanaPublicKey: 'Other',
        keys: savedKeys,
        signer: signer,
        backend: FakeBackend(FakeWallet()),
      ),
      throwsWalletError('signer_mismatch'),
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
    expect(refreshed.summary, prepared.summary);

    await wallet.submit(refreshed, [Uint8List(64)]);
    expect(
      (native.submitted.single.$1 as FakePending).bytes,
      refreshed.message,
    );
    await wallet.close();
    await expectLater(
      wallet.refresh(prepared),
      throwsWalletError('wallet_closed'),
    );
  });

  test('signs the proved message with its summary and submits it', () async {
    final (wallet, native, signer, _) = await openWallet();

    final signature = await wallet.transfer(
      recipient: 'Recipient',
      amount: BigInt.from(5),
    );

    expect(signature, 'signature');
    expect(signer.requests.last.$1, [7, 8]);
    expect(signer.requests.last.$2, 'summary of 7');
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
    expect(transaction.summary, 'summary of 7');

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

  test('hands the remote prover to the native wallet', () async {
    final requests = <Uint8List>[];
    Future<Uint8List> backend(Uint8List request) async {
      requests.add(request);
      if (request.isEmpty) throw StateError('backend unreachable');
      return Uint8List.fromList([...request.reversed]);
    }

    final opened = FakeWallet();
    await ZolanaWallet.open(
      config: config,
      signer: RecordingSigner(),
      remoteProver: backend,
      backend: FakeBackend(opened),
    );
    final restored = FakeWallet();
    await ZolanaWallet.openWithKeys(
      config: config,
      solanaPublicKey: owner,
      keys: savedKeys,
      remoteProver: backend,
      backend: FakeBackend(restored),
    );
    for (final native in [opened, restored]) {
      final prove = native.remoteProver!;
      expect(await prove(Uint8List.fromList([1, 2])), [2, 1]);
      // A failed backend is a missing proof: the native wallet names it.
      expect(await prove(Uint8List(0)), isNull);
    }
    expect(requests, hasLength(4));

    final (_, local, _, _) = await openWallet();
    expect(local.remoteProver, isNull);
  });

  test('refuses a transaction another key must sign', () async {
    final (wallet, native, signer, _) = await openWallet();
    native.transferSigners = const [owner, 'SomeoneElse'];

    await expectLater(
      wallet.transfer(recipient: 'Recipient', amount: BigInt.one),
      throwsWalletError('unexpected_signers'),
    );
    expect(signer.requests, hasLength(1), reason: 'only the open was signed');
    expect(native.submitted, isEmpty);
  });

  test('surfaces native failures as wallet exceptions', () async {
    final (wallet, native, _, _) = await openWallet();
    native.transferError = 'recipient_not_registered';

    await expectLater(
      wallet.transfer(recipient: 'Recipient', amount: BigInt.one),
      throwsWalletError('recipient_not_registered'),
    );
  });

  test('registers once and never replaces other keys', () async {
    final (wallet, native, signer, _) = await openWallet();

    native.status = RegistrationStatus.notRegistered;
    expect(await wallet.register(), 'signature');
    expect(signer.requests.last.$2, 'summary of 3');

    native.status = RegistrationStatus.registered;
    expect(await wallet.register(), isNull);

    native.status = RegistrationStatus.conflict;
    expect(await wallet.registrationStatus(), RegistrationStatus.conflict);
    await expectLater(
      wallet.register(),
      throwsWalletError('registration_conflict'),
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

      await expectLater(prepared, throwsWalletError('wallet_closed'));
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
    await expectLater(transfer, throwsWalletError('wallet_closed'));
    await expectLater(queued, throwsWalletError('wallet_closed'));
    await closed;
    expect(native.events, ['transfer SOL', 'dispose']);
    expect(signer.requests, hasLength(1), reason: 'only the open was signed');
    expect(native.submitted, isEmpty);

    await expectLater(
      wallet.deposit(BigInt.one),
      throwsWalletError('wallet_closed'),
    );
    expect(identical(wallet.close(), closed), isTrue);
  });

  test('runs operations one at a time and survives a failure', () async {
    final (wallet, native, _, _) = await openWallet();
    native.holdBalances = Completer();
    native.transferError = 'proof_failed';

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
