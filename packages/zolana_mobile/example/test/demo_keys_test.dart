import 'package:flutter_test/flutter_test.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';

void main() {
  test('each embedded seed derives its listed account', () async {
    for (final account in demoAccounts) {
      final signer = await DemoSigner.fromSeedHex(account.seedHex);
      expect(signer.publicKey, account.publicKey, reason: account.name);
    }
  });
}
