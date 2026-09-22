import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

const owner = 'Owner1111111111111111111111111111111111111';
const config = WalletConfig(
  rpcUrl: 'https://rpc.example',
  indexerUrl: 'https://indexer.example',
  provingKeyDir: '/keys',
  allowInsecureHttp: false,
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

class FakePending implements NativePending {
  FakePending(this.bytes, {this.signerKeys = const [owner]});

  final Uint8List bytes;
  final List<String> signerKeys;

  @override
  Future<Uint8List> messageBytes() async => bytes;

  @override
  Future<List<String>> signers() async => signerKeys;

  @override
  Future<String> summary() async => 'summary of ${bytes.first}';
}

class FakeWallet implements NativeWallet {
  final submitted = <(NativePending, List<Uint8List>)>[];
  final events = <String>[];
  Completer<void>? holdSync;
  Object? transferError;
  List<String> transferSigners = const [owner];

  @override
  Future<String> shieldedAddress() async => 'shielded';

  @override
  Future<bool> isRegistered() async => true;

  @override
  Future<SyncSummary> sync() async {
    events.add('sync start');
    await holdSync?.future;
    events.add('sync end');
    return SyncSummary(storedUtxos: BigInt.one, privateLamports: BigInt.two);
  }

  @override
  Future<BigInt> privateLamports() async => BigInt.two;

  @override
  Future<BigInt> publicLamports() async => BigInt.one;

  @override
  Future<List<ActivityEntry>> activity() async => const [];

  @override
  Future<NativePending?> prepareRegistration() async => null;

  @override
  Future<NativePending> prepareDeposit(BigInt lamports) async =>
      FakePending(Uint8List.fromList([1, 2]));

  @override
  Future<NativePending> prepareTransfer(
    String recipient,
    BigInt lamports,
  ) async {
    events.add('transfer');
    final error = transferError;
    if (error != null) throw error;
    return FakePending(Uint8List.fromList([7, 8]), signerKeys: transferSigners);
  }

  @override
  Future<NativePending> prepareWithdrawal(
    String recipient,
    BigInt lamports,
  ) async => FakePending(Uint8List.fromList([9]));

  @override
  Future<String> submit(
    NativePending pending,
    List<Uint8List> signatures,
  ) async {
    submitted.add((pending, signatures));
    return 'signature';
  }
}

class FakeBackend implements WalletBackend {
  FakeBackend(this.wallet);

  final FakeWallet wallet;
  Uint8List? openedWith;

  @override
  Future<Uint8List> derivationMessage(String solanaPubkey) async =>
      Uint8List.fromList(solanaPubkey.codeUnits);

  @override
  Future<NativeWallet> open(
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
      lamports: BigInt.from(5),
    );

    expect(signature, 'signature');
    expect(signer.requests.last.$1, [7, 8]);
    expect(signer.requests.last.$2, 'summary of 7');
    expect(native.submitted.single.$2.single, [8, 7]);
  });

  test('refuses a transaction another key must sign', () async {
    final (wallet, native, signer, _) = await openWallet();
    native.transferSigners = const [owner, 'SomeoneElse'];

    await expectLater(
      wallet.transfer(recipient: 'Recipient', lamports: BigInt.one),
      throwsA(
        isA<ZolanaWalletException>().having(
          (e) => e.message,
          'message',
          'unexpected_signers',
        ),
      ),
    );
    expect(signer.requests, hasLength(1), reason: 'only the open was signed');
    expect(native.submitted, isEmpty);
  });

  test('surfaces native failures as wallet exceptions', () async {
    final (wallet, native, _, _) = await openWallet();
    native.transferError = 'recipient_not_registered';

    await expectLater(
      wallet.transfer(recipient: 'Recipient', lamports: BigInt.one),
      throwsA(
        isA<ZolanaWalletException>().having(
          (e) => e.message,
          'message',
          'recipient_not_registered',
        ),
      ),
    );
  });

  test('runs operations one at a time and survives a failure', () async {
    final (wallet, native, _, _) = await openWallet();
    native.holdSync = Completer();
    native.transferError = 'proof_failed';

    final sync = wallet.sync();
    final transfer = wallet.transfer(recipient: 'R', lamports: BigInt.one);
    final deposit = wallet.deposit(BigInt.one);
    await Future<void>.delayed(Duration.zero);
    expect(native.events, ['sync start']);

    native.holdSync!.complete();
    await sync;
    await expectLater(transfer, throwsA(isA<ZolanaWalletException>()));
    expect(await deposit, 'signature');
    expect(native.events, ['sync start', 'sync end', 'transfer']);
  });
}
