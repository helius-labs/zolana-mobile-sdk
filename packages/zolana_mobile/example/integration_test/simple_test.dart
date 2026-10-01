import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(initZolanaMobile);

  testWidgets('opens a wallet from a device-held signer', (tester) async {
    final signer = await DemoSigner.generate();
    Future<ZolanaWallet> open() => ZolanaWallet.open(
      signer: signer,
      // Opening touches no service; these are never contacted.
      config: const WalletConfig(
        rpcUrl: 'http://127.0.0.1:1',
        indexerUrl: 'http://127.0.0.1:1',
        provingKeyDir: '/nonexistent',
        allowInsecureHttp: false,
        mints: [],
      ),
    );

    final wallet = await open();
    addTearDown(wallet.close);
    expect(wallet.solanaPublicKey, signer.publicKey);
    expect(wallet.shieldedAddress, isNotEmpty);
    final again = await open();
    addTearDown(again.close);
    expect(again.shieldedAddress, wallet.shieldedAddress);
  });
}
