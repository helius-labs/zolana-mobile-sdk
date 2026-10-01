import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
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
  FakePending(this.bytes, {this.signerKeys = const [owner]});

  final Uint8List bytes;
  final List<String> signerKeys;

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
  final confirmed = <(PendingTransaction, String)>[];
  final events = <String>[];
  Completer<void>? holdBalances;
  Completer<void>? holdTransfer;
  Object? transferError;
  bool disposed = false;
  List<String> transferSigners = const [owner];
  String? transferFeePayer;

  @override
  Future<String> shieldedAddress() async => 'shielded';

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
  Future<PendingTransaction?> prepareRegistration() async => switch (status) {
    RegistrationStatus.registered => null,
    RegistrationStatus.conflict => throw 'registration_conflict',
    RegistrationStatus.notRegistered => FakePending(Uint8List.fromList([3])),
  };

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
  }) async {
    events.add('transfer ${mint ?? 'SOL'}');
    transferFeePayer = feePayer;
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
  }) async => FakePending(Uint8List.fromList([9]));

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

  @override
  Future<Uint8List> derivationMessage(String solanaPubkey) async =>
      Uint8List.fromList(solanaPubkey.codeUnits);

  @override
  Future<MobileWallet> open(
    WalletConfig config,
    String solanaPubkey,
    Uint8List derivationSignature,
  ) async {
    openedWith = derivationSignature;
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
