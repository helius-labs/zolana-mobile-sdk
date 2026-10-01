import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';
import 'package:zolana_mobile_demo/wallet_screen.dart' show Network;

/// A private send from demo account A to B on Zolana devnet, proved on the
/// device or simulator running the test. Opt in, since it spends devnet SOL
/// and needs A funded and a Helius key for devnet:
///
/// ```sh
/// flutter test integration_test/devnet_wallet_test.dart -d DEVICE \
///   --dart-define=ZOLANA_E2E=true --dart-define=ZOLANA_API_KEY=...
/// ```
const _enabled = bool.fromEnvironment('ZOLANA_E2E');
const _apiKey = String.fromEnvironment('ZOLANA_API_KEY');

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
      mints: const [],
    ),
  );

  testWidgets('sends privately from A to B, proving on this device', (
    tester,
  ) async {
    expect(
      _apiKey,
      isNotEmpty,
      reason: 'pass --dart-define=ZOLANA_API_KEY=...',
    );
    final lamports = BigInt.from(1000000);
    final sender = await open(demoAccounts[0]);
    addTearDown(sender.close);
    final recipient = await open(demoAccounts[1]);
    addTearDown(recipient.close);
    expect(await sender.registrationStatus(), RegistrationStatus.registered);
    expect(await recipient.registrationStatus(), RegistrationStatus.registered);

    final senderBefore = await sender.privateBalance();
    final recipientBefore = await recipient.privateBalance();
    expect(senderBefore, greaterThanOrEqualTo(lamports));

    final started = DateTime.now();
    final signature = await sender.transfer(
      recipient: demoAccounts[1].publicKey,
      amount: lamports,
    );
    // ignore: avoid_print
    print(
      'sent $signature in ${DateTime.now().difference(started).inMilliseconds} ms '
      '(proving key download, proof, signing, confirmation)',
    );

    // The transfer returned once the indexer had it, so both balances
    // already read its notes.
    expect(await sender.privateBalance(), senderBefore - lamports);
    expect(await recipient.privateBalance(), recipientBefore + lamports);
  }, skip: !_enabled);
}
