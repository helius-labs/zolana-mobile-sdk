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
      await RustLib.init();
      final directory = await Directory.systemTemp.createTemp(
        'zolana-consumer-',
      );
      LocalProver? prover;
      try {
        for (final extension in ['pk', 'vk', 'r1cs']) {
          final name = 'transfer_confidential_2_3.$extension';
          final data = await rootBundle.load('assets/proving/$name');
          await File('${directory.path}/$name')
              .writeAsBytes(data.buffer.asUint8List());
        }
        final basename = '${directory.path}/transfer_confidential_2_3';
        prover = await LocalProver.load(
          r1csPath: '$basename.r1cs',
          provingKeyPath: '$basename.pk',
          verifyingKeyPath: '$basename.vk',
        );
        final request = await rootBundle.loadString(
          'assets/proving/prove-request-2x3.json',
        );
        final result = await prover.proveRequest(request).result;
        expect(result.verified, isTrue);
        expect((result.inputs, result.outputs), (2, 3));
        expect(
          jsonDecode(result.proofJson),
          containsPair('ar', isA<List<dynamic>>()),
        );
        final malformed = prover.proveWitness('{"Secret": "private-sentinel"}');
        await expectLater(
          malformed.result,
          throwsA(
            isA<ProverException>().having(
              (error) => error.toString(),
              'message',
              isNot(contains('private-sentinel')),
            ),
          ),
        );
        await malformed.done;
        final warm = prover.proveRequest(request);
        final rejected = expectLater(
          warm.result,
          throwsA(
            isA<ProverException>().having(
              (error) => error.code,
              'code',
              ProverErrorCode.discarded,
            ),
          ),
        );
        final draining = prover.close();
        expect(prover.isClosed, isTrue);
        expect(
          () => prover!.proveRequest(request),
          throwsA(isA<ProverException>()),
        );
        await rejected;
        await draining;
      } finally {
        await prover?.close();
        await directory.delete(recursive: true);
      }
    },
    timeout: const Timeout(Duration(minutes: 5)),
  );
}
