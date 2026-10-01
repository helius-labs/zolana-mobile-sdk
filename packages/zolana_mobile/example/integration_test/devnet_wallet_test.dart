import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';
import 'package:zolana_mobile_demo/http_client_transport.dart';
import 'package:zolana_mobile_demo/wallet_screen.dart' show Network;

/// Private sends from demo account A to B on Zolana devnet: one proved on the
/// device or simulator running the test, with A reopened from its exported
/// keys and the transfer signed by the test after a blockhash refresh, as an
/// application that signs itself does; one proved by a backend that forwards
/// the request to the Helius prover. In both, A sends every request through a
/// `dart:io` transport, as an application with its own networking does; B
/// uses the wallet's HTTP clients. Opt in, since it spends devnet SOL and
/// needs A funded and a Helius key for devnet:
///
/// ```sh
/// flutter test integration_test/devnet_wallet_test.dart -d DEVICE \
///   --dart-define=ZOLANA_E2E=true --dart-define=ZOLANA_API_KEY=...
/// ```
const _enabled = bool.fromEnvironment('ZOLANA_E2E');
const _apiKey = String.fromEnvironment('ZOLANA_API_KEY');

/// An application's backend: it forwards each request to the Helius prover as
/// the Zolana SDK's prover client does, and returns the proof.
Future<Uint8List> heliusProver(Uint8List request) async {
  final base = Uri.parse(Network.devnet.indexerUrl);
  Uri at(String path, [Map<String, String> query = const {}]) => base.replace(
    path: '${base.path}/$path',
    queryParameters: {...base.queryParameters, ...query},
  );
  final http = HttpClient();
  Future<Map<String, Object?>> json(HttpClientRequest call) async {
    final response = await call.close();
    final body = await utf8.decodeStream(response);
    if (response.statusCode >= 300) {
      throw HttpException('prover status ${response.statusCode}');
    }
    return jsonDecode(body) as Map<String, Object?>;
  }

  try {
    final post = await http.postUrl(at('prove'));
    post.headers
      ..contentType = ContentType.json
      ..set('X-Sync', 'true');
    post.add(request);
    var response = await json(post);
    final job = response['jobId'];
    if (job is! String) return utf8.encode(jsonEncode(response));
    for (var poll = 0; poll < 240; poll++) {
      await Future<void>.delayed(const Duration(milliseconds: 250));
      response = await json(
        await http.getUrl(at('prove/status', {'jobId': job})),
      );
      switch (response['status']) {
        case 'completed':
          return utf8.encode(jsonEncode(response['result']));
        case 'failed':
          throw StateError('the prover failed');
      }
    }
    throw TimeoutException('the proof is not ready');
  } finally {
    http.close();
  }
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(initZolanaMobile);

  Future<WalletConfig> config() async => WalletConfig(
    rpcUrl: Network.devnet.rpcUrl,
    indexerUrl: Network.devnet.indexerUrl,
    provingKeyDir:
        '${(await getApplicationSupportDirectory()).path}/proving-keys',
    allowInsecureHttp: false,
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
    final transport = HttpClientTransport();
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
      await signer.signMessage(
        transaction.message,
        purpose: transaction.summary,
      ),
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

  testWidgets('sends privately from A to B, proved by the backend', (
    tester,
  ) async {
    final lamports = BigInt.from(1000000);
    RemoteProver backend = (_) => throw const SocketException('unreachable');
    final transport = HttpClientTransport();
    addTearDown(transport.close);
    final sender = await ZolanaWallet.open(
      signer: await DemoSigner.fromSeedHex(demoAccounts[0].seedHex),
      config: await config(),
      remoteProver: (request) => backend(request),
      transport: transport.send,
    );
    addTearDown(sender.close);
    final recipient = await open(demoAccounts[1]);
    addTearDown(recipient.close);
    final senderBefore = await sender.privateBalance();
    final recipientBefore = await recipient.privateBalance();

    await expectLater(
      sender.prepareTransfer(
        recipient: demoAccounts[1].publicKey,
        amount: lamports,
        proving: Proving.remote,
      ),
      throwsA(
        isA<ZolanaWalletException>().having(
          (e) => e.message,
          'message',
          'remote_prover_failed',
        ),
      ),
    );
    backend = heliusProver;
    final started = DateTime.now();
    final signature = await sender.transfer(
      recipient: demoAccounts[1].publicKey,
      amount: lamports,
      proving: Proving.remote,
    );
    // ignore: avoid_print
    print(
      'sent $signature in ${DateTime.now().difference(started).inMilliseconds} ms '
      '(backend proof, verification, signing, confirmation)',
    );
    expect(await sender.privateBalance(), senderBefore - lamports);
    expect(await recipient.privateBalance(), recipientBefore + lamports);
    // ignore: avoid_print
    print('the sender sent ${transport.sent.length} requests through Dart');
    expect(transport.sent, contains('POST /'));
    expect(transport.sent, contains('POST /v1/zolana/getMerkleProofs'));
    expect(transport.sent.where((r) => r.startsWith('GET ')), isEmpty);
  }, skip: !_enabled);
}
