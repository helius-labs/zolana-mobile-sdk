import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets(
    'clean consumer initializes, proves a request, and drains on lock',
    (tester) async {
      await initZolanaMobile();
      final directory = await Directory.systemTemp.createTemp(
        'zolana-consumer-',
      );
      LocalProver? prover;
      try {
        const name = 'transfer_confidential_2_2.zopro';
        final data = await rootBundle.load('assets/proving/$name');
        final keyPath = '${directory.path}/$name';
        await File(keyPath).writeAsBytes(data.buffer.asUint8List());
        prover = await LocalProver.load(keyPath: keyPath);
        final request = await rootBundle.loadString(
          'assets/proving/prove-request-2x2.json',
        );
        final result = await prover.prove(requestJson: request);
        expect(result.verified, isTrue);
        expect((result.inputs, result.outputs), (2, 2));
        expect(
          jsonDecode(result.proofJson),
          containsPair('ar', isA<List<dynamic>>()),
        );
        await expectLater(
          prover.prove(requestJson: '{"Secret": "private-sentinel"}'),
          throwsA(
            isA<WalletError>().having(
              (error) => error.toString(),
              'message',
              isNot(contains('private-sentinel')),
            ),
          ),
        );
        // Locking during a proof: the proof fails, the circuit is released.
        final warm = prover.prove(requestJson: request);
        final rejected = expectLater(
          warm,
          throwsA(const WalletError.proverClosed()),
        );
        await prover.close();
        await rejected;
        await expectLater(
          prover.prove(requestJson: request),
          throwsA(const WalletError.proverClosed()),
        );
      } finally {
        await prover?.close();
        await directory.delete(recursive: true);
      }
    },
    timeout: const Timeout(Duration(minutes: 5)),
  );
}
