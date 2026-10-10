import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/app_transport.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';
import 'package:zolana_mobile_demo/wallet_screen.dart' show Network;

/// Private sends from demo account A to B on Zolana devnet: one proved on the
/// device or simulator running the test, with A reopened from its exported
/// keys and the transfer signed by the test after a blockhash refresh, as an
/// application that signs itself does; one proved remotely by the Helius
/// prover, asked through the transport. In both, A sends every request
/// through the application's own transport (`package:http`, logged), as an
/// application with its own networking does; B uses the package's default
/// transport. Then B merges its notes twice, proved remotely and on the
/// device. Opt in, since it spends devnet SOL and needs A funded and a
/// Helius key for devnet:
///
/// ```sh
/// flutter test integration_test/devnet_wallet_test.dart -d DEVICE \
///   --dart-define=ZOLANA_E2E=true --dart-define=ZOLANA_API_KEY=...
/// ```
///
/// `--dart-define=ZOLANA_E2E_PROVER_URL=...` proves remotely with another
/// prover than the Helius gateway.
const _enabled = bool.fromEnvironment('ZOLANA_E2E');
const _apiKey = String.fromEnvironment('ZOLANA_API_KEY');
const _proverUrl = String.fromEnvironment('ZOLANA_E2E_PROVER_URL');

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(initZolanaMobile);

  Future<WalletConfig> config({String? proverUrl}) async => WalletConfig(
    rpcUrl: Network.devnet.rpcUrl,
    indexerUrl: Network.devnet.indexerUrl,
    provingKeyDir:
        '${(await getApplicationSupportDirectory()).path}/proving-keys',
    proverUrl: proverUrl,
    mints: const [],
  );

  Future<ZolanaWallet> open(DemoAccount account) async => ZolanaWallet.open(
    signer: await DemoSigner.fromSeedHex(account.seedHex),
    config: await config(),
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
    final signer = await DemoSigner.fromSeedHex(demoAccounts[0].seedHex);
    final signed = await open(demoAccounts[0]);
    final keys = await signed.exportKeys();
    await signed.close();
    final transport = AppTransport();
    addTearDown(transport.close);
    final sender = await ZolanaWallet.openWithKeys(
      config: await config(),
      solanaPublicKey: demoAccounts[0].publicKey,
      keys: keys,
      transport: transport.send,
    );
    addTearDown(sender.close);
    expect(sender.shieldedAddress, signed.shieldedAddress);
    final recipient = await open(demoAccounts[1]);
    addTearDown(recipient.close);
    expect(await sender.registrationStatus(), RegistrationStatus.registered);
    expect(await recipient.registrationStatus(), RegistrationStatus.registered);

    final senderBefore = await sender.privateBalance();
    final recipientBefore = await recipient.privateBalance();
    expect(senderBefore, greaterThanOrEqualTo(lamports));

    final started = DateTime.now();
    final prepared = await sender.prepareTransfer(
      recipient: demoAccounts[1].publicKey,
      amount: lamports,
    );
    // A slow approval: the same proof under a new blockhash.
    final transaction = await sender.refresh(prepared);
    expect(
      transaction.lastValidBlockHeight,
      greaterThanOrEqualTo(prepared.lastValidBlockHeight),
    );
    final signature = await sender.submit(transaction, [
      await signer.signMessage(transaction.message, transaction: transaction),
    ]);
    // ignore: avoid_print
    print(
      'sent $signature in ${DateTime.now().difference(started).inMilliseconds} ms '
      '(proving key download, proof, signing, confirmation)',
    );

    // The transfer returned once the indexer had it, so both balances
    // already read its notes.
    expect(await sender.privateBalance(), senderBefore - lamports);
    expect(await recipient.privateBalance(), recipientBefore + lamports);
    // ignore: avoid_print
    print(
      'the sender sent ${transport.sent.length} requests through Dart, '
      'key downloads: ${transport.sent.where((r) => r.startsWith('GET '))}',
    );
    expect(transport.sent, contains('POST /'));
    expect(transport.sent, contains('POST /v1/zolana/getMerkleProofs'));
  }, skip: !_enabled);

  testWidgets('sends privately from A to B, proved remotely', (tester) async {
    final lamports = BigInt.from(1000000);
    final transport = AppTransport();
    addTearDown(transport.close);
    Future<ZolanaWallet> sender({String? proverUrl}) async => ZolanaWallet.open(
      signer: await DemoSigner.fromSeedHex(demoAccounts[0].seedHex),
      config: await config(proverUrl: proverUrl),
      transport: transport.send,
    );
    final local = await sender();
    addTearDown(local.close);
    await expectLater(
      local.prepareTransfer(
        recipient: demoAccounts[1].publicKey,
        amount: lamports,
        proving: Proving.remote,
      ),
      throwsA(
        isA<ZolanaWalletException>().having(
          (e) => e.error,
          'error',
          const WalletError.remoteProverMissing(),
        ),
      ),
    );
    await local.close();
    // By default the Helius gateway, whose base URL is the indexer's.
    final remote = await sender(
      proverUrl: _proverUrl.isEmpty ? Network.devnet.indexerUrl : _proverUrl,
    );
    addTearDown(remote.close);
    final recipient = await open(demoAccounts[1]);
    addTearDown(recipient.close);
    final senderBefore = await remote.privateBalance();
    final recipientBefore = await recipient.privateBalance();

    final started = DateTime.now();
    final signature = await remote.transfer(
      recipient: demoAccounts[1].publicKey,
      amount: lamports,
      proving: Proving.remote,
    );
    // ignore: avoid_print
    print(
      'sent $signature in ${DateTime.now().difference(started).inMilliseconds} ms '
      '(remote proof, verification, signing, confirmation)',
    );
    expect(await remote.privateBalance(), senderBefore - lamports);
    expect(await recipient.privateBalance(), recipientBefore + lamports);
    // ignore: avoid_print
    print(
      'the sender sent ${transport.sent.length} requests through Dart, '
      'proofs: ${transport.sent.where(_isProof)}',
    );
    expect(transport.sent, contains('POST /'));
    expect(transport.sent, contains('POST /v1/zolana/getMerkleProofs'));
    expect(
      transport.sent.where((r) => r.startsWith('POST ') && _isProof(r)),
      hasLength(1),
    );
    expect(
      transport.sent.where((r) => r.startsWith('GET ') && !_isProof(r)),
      isEmpty,
    );
  }, skip: !_enabled);

  testWidgets('merges B notes, proved remotely and on this device', (
    tester,
  ) async {
    final transport = AppTransport();
    addTearDown(transport.close);
    final wallet = await ZolanaWallet.open(
      signer: await DemoSigner.fromSeedHex(demoAccounts[1].seedHex),
      config: await config(
        proverUrl: _proverUrl.isEmpty ? Network.devnet.indexerUrl : _proverUrl,
      ),
      transport: transport.send,
    );
    addTearDown(wallet.close);
    await wallet.setMerging(true);
    expect(await wallet.mergingEnabled(), isTrue);
    for (final proving in [Proving.remote, Proving.local]) {
      final before = await wallet.privateBalance();
      final started = DateTime.now();
      final signature = await wallet.merge(maxInputs: 8, proving: proving);
      // ignore: avoid_print
      print(
        'merged ${proving.name} in '
        '${DateTime.now().difference(started).inMilliseconds} ms: $signature',
      );
      expect(await wallet.privateBalance(), before);
      final newest = (await wallet.activity()).first;
      expect(
        (newest.signature, newest.kind),
        (signature, ActivityKind.selfTransfer),
      );
    }
    expect(
      transport.sent.where((r) => r.startsWith('POST ') && _isProof(r)),
      hasLength(1),
    );
  }, skip: !_enabled);
}

/// A proof request or status poll: `/prove/<key>` on a prover, the key-less
/// `/prove` on the Helius gateway.
bool _isProof(String request) =>
    RegExp(r'/prove(/|$)').hasMatch(request.split(' ').last);
