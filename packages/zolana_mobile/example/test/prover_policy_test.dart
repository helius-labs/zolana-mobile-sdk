import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile_demo/prover_policy.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'the example bundles the complete reviewed policy for remote clients',
    () async {
      final policy = jsonDecode(await demoProverPolicy())['deployment'] as Map;
      expect(policy['platform'], 'aws-nitro');
      expect(policy['hpke_public_key'], matches(RegExp(r'^[0-9a-f]{64}$')));
      expect(policy['measurements'], hasLength(1));
      for (final pcr in ['pcr0', 'pcr1', 'pcr2']) {
        expect(
          policy['measurements'][0][pcr],
          matches(RegExp(r'^[0-9a-f]{96}$')),
        );
      }
    },
  );
}
