import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/demo_signer.dart' show base58Encode;
import 'package:zolana_mobile_demo/recovery.dart';

void main() {
  test(
    'client errors keep the send until recovery finishes indexing',
    () async {
      final pending = MemoryPendingSends();
      final wallet = _Wallet(pending);
      final signer = _Signer();
      final transaction = _Transaction();
      final send = (
        signature: base58Encode(signer.signature),
        lastValidBlockHeight: transaction.lastValidBlockHeight,
      );
      const error = WalletError.client(
        message: 'transaction indexing timed out',
      );
      wallet.error = error;

      await expectLater(
        saveAndSend(wallet, signer, pending, transaction),
        throwsA(same(error)),
      );
      expect(await pending.read(wallet.solanaPublicKey), send);

      await expectLater(
        settlePendingSend(wallet, pending),
        throwsA(same(error)),
      );
      expect(await pending.read(wallet.solanaPublicKey), send);

      wallet.error = null;
      expect(await settlePendingSend(wallet, pending), send.signature);
      expect(wallet.waited, [send, send]);
      expect(await pending.read(wallet.solanaPublicKey), isNull);
    },
  );
}

class _Wallet implements ZolanaWallet {
  _Wallet(this.pending);

  final PendingSends pending;
  WalletError? error;
  final waited = <PendingSend>[];

  @override
  String get solanaPublicKey => 'account';

  @override
  Future<String> submit(
    PreparedTransaction transaction,
    List<Uint8List> signatures,
  ) async {
    final signature = base58Encode(signatures.single);
    expect(await pending.read(solanaPublicKey), (
      signature: signature,
      lastValidBlockHeight: transaction.lastValidBlockHeight,
    ));
    if (error case final error?) throw error;
    return signature;
  }

  @override
  Future<void> waitForTransaction(
    String signature, {
    BigInt? lastValidBlockHeight,
  }) async {
    waited.add((
      signature: signature,
      lastValidBlockHeight: lastValidBlockHeight!,
    ));
    if (error case final error?) throw error;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _Signer implements SolanaSigner {
  final signature = Uint8List.fromList(List.filled(64, 1));

  @override
  String get publicKey => 'account';

  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    PreparedTransaction? transaction,
  }) async => signature;
}

class _Transaction implements PreparedTransaction {
  @override
  Uint8List get message => Uint8List.fromList([1, 2, 3]);

  @override
  BigInt get lastValidBlockHeight => BigInt.from(100);

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
