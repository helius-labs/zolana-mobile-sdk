import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

void main() {
  test('transfer construction remains part of the SDK API', () {
    final request = TransferDraftRequest(
      senderSeed: Uint8List.fromList(List.filled(32, 7)),
      recipient: 'recipient',
      inputLamports: BigInt.from(10),
      transferLamports: BigInt.from(4),
    );

    expect(request.senderSeed, hasLength(32));
    expect(request.inputLamports, BigInt.from(10));
    expect(request.transferLamports, BigInt.from(4));
  });
}
