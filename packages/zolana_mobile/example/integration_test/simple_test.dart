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
      ),
    );

    final wallet = await open();
    expect(wallet.solanaPublicKey, signer.publicKey);
    expect(wallet.shieldedAddress, isNotEmpty);
    expect((await open()).shieldedAddress, wallet.shieldedAddress);
  });
}
