import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';
import 'package:zolana_mobile_demo/wallet_screen.dart' show Network;

/// A private send from demo account A to B on Zolana devnet, proved on the
/// device or simulator running the test. Opt in, since it spends devnet SOL
/// and needs A funded:
///
/// ```sh
/// flutter test integration_test/devnet_wallet_test.dart -d DEVICE \
///   --dart-define=ZOLANA_E2E=true [--dart-define=ZOLANA_API_KEY=...]
/// ```
const _enabled = bool.fromEnvironment('ZOLANA_E2E');

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(initZolanaMobile);

  Future<ZolanaWallet> open(DemoAccount account) async => ZolanaWallet.open(
    signer: await DemoSigner.fromSeedHex(account.seedHex),
    config: WalletConfig(
      rpcUrl: Network.devnet.rpcUrl,
      indexerUrl: Network.devnet.indexerUrl,
      provingKeyDir:
          '${(await getApplicationSupportDirectory()).path}/proving-keys',
      allowInsecureHttp: false,
    ),
  );

  testWidgets('sends privately from A to B, proving on this device', (
    tester,
  ) async {
    final lamports = BigInt.from(1000000);
    final sender = await open(demoAccounts[0]);
    final recipient = await open(demoAccounts[1]);
    expect(await sender.isRegistered(), isTrue);
    expect(await recipient.isRegistered(), isTrue);

    final senderBefore = (await sender.sync()).privateLamports;
    final recipientBefore = (await recipient.sync()).privateLamports;
    expect(senderBefore, greaterThanOrEqualTo(lamports));

    final started = DateTime.now();
    final signature = await sender.transfer(
      recipient: demoAccounts[1].publicKey,
      lamports: lamports,
    );
    // ignore: avoid_print
    print(
      'sent $signature in ${DateTime.now().difference(started).inMilliseconds} ms '
      '(proving key download, proof, signing, confirmation)',
    );

    expect(await sender.privateLamports(), senderBefore - lamports);
    expect(
      (await recipient.sync()).privateLamports,
      recipientBefore + lamports,
    );
  }, skip: !_enabled);
}
