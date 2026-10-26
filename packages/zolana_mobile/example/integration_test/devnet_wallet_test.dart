import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:zolana_mobile/zolana_mobile.dart';
import 'package:zolana_mobile_demo/app_transport.dart';
import 'package:zolana_mobile_demo/demo_assets.dart';
import 'package:zolana_mobile_demo/demo_keys.dart';
import 'package:zolana_mobile_demo/demo_signer.dart';
import 'package:zolana_mobile_demo/test_cluster_rpc.dart';
import 'package:zolana_mobile_demo/spend_checks.dart';
import 'package:zolana_mobile_demo/wallet_screen.dart' show Network;

/// Private sends from demo account A to B on Zolana devnet: one proved on the
/// device or simulator running the test, with the demo's fee paid to its
/// collector as a second private output of the same proof, with A reopened
/// from its exported keys and the spend signed by the test after a blockhash
/// refresh, as an application that signs itself does; one proved remotely by
/// the Helius prover, asked through the transport. In both, A sends every request
/// through the application's own transport (`package:http`, logged), as an
/// application with its own networking does; B uses the package's default
/// transport. Then B merges its notes twice, proved remotely and on the
/// device, and the demo token is shielded, sent and unshielded. Opt in,
/// since it spends devnet SOL and needs A funded and a Helius key for
/// devnet:
///
/// ```sh
/// flutter test integration_test/devnet_wallet_test.dart -d DEVICE \
///   --dart-define=ZOLANA_E2E=true --dart-define=ZOLANA_API_KEY=...
/// ```
///
/// `--dart-define=ZOLANA_E2E_PROVER_URL=...` proves remotely with another
/// prover than the deployment's.
const _enabled = bool.fromEnvironment('ZOLANA_E2E');
const _apiKey = String.fromEnvironment('ZOLANA_API_KEY');
const _proverUrl = String.fromEnvironment('ZOLANA_E2E_PROVER_URL');

/// devnet-d's attested prover, directly until the Helius gateway serves
/// devnet-d.
const _deploymentProver = 'https://d1unet9hn0mci0.cloudfront.net';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(initZolanaMobile);

  Future<WalletConfig> config({String? proverUrl}) async => WalletConfig(
    rpcUrl: Network.devnet.rpcUrl,
    indexerUrl: Network.devnet.indexerUrl,
    provingKeyDir:
        '${(await getApplicationSupportDirectory()).path}/proving-keys',
    proverUrl: proverUrl,
    mints: [for (final asset in demoAssets) ?asset.config],
  );

  final signers = <ZolanaWallet, DemoSigner>{};
  Future<ZolanaWallet> open(
    DemoAccount account, {
    String? proverUrl,
    ZolanaTransport? transport,
  }) async {
    final signer = await DemoSigner.fromSeedHex(
      account.seedHex,
      spendChecks: SpendChecks(TestClusterRpc(Network.devnet.rpcUrl)),
    );
    final wallet = await ZolanaWallet.open(
      signer: signer,
      config: await config(proverUrl: proverUrl),
      transport: transport,
    );
    signers[wallet] = signer;
    return wallet;
  }

  Future<String> checkedSpend(
    ZolanaWallet wallet, {
    required List<Output> outputs,
    String? mint,
    Proving? proving,
  }) => signers[wallet]!.spend(
    wallet,
    outputs: outputs,
    mint: mint,
    proving: proving,
  );

  Future<void> checkFreshLookup(ZolanaWallet wallet, String signature) async {
    final transport = AppTransport();
    final cold = await ZolanaWallet.openWithKeys(
      config: await config(),
      solanaPublicKey: wallet.solanaPublicKey,
      keys: await wallet.exportKeys(),
      transport: transport.send,
    );
    try {
      final clock = Stopwatch()..start();
      final result = await cold.transaction(signature);
      // ignore: avoid_print
      print(
        'cold signature lookup: ${clock.elapsedMilliseconds} ms, ${result.entries.length} rows',
      );
      expect(result.status, TransactionLookupStatus.ready);
      expect(
        transport.sent,
        anyElement(endsWith('/getShieldedTransactionsBySignature')),
      );
      expect(
        transport.sent.where(
          (request) => [
            '/getShieldedTransactionsByTags',
            '/getShieldedTransactionsByNullifiers',
            '/getEncryptedUtxosByTags',
          ].any(request.endsWith),
        ),
        isEmpty,
      );
      final expected = (await wallet.activity())
          .where((row) => row.signature == signature)
          .toList();
      expect(result.entries, orderedEquals(expected));
    } finally {
      await cold.close();
      transport.close();
    }
  }

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
    final collector = await open(demoFeeCollector);
    addTearDown(collector.close);
    expect(await sender.registrationStatus(), RegistrationStatus.registered);
    expect(await recipient.registrationStatus(), RegistrationStatus.registered);
    expect(await collector.registrationStatus(), RegistrationStatus.registered);

    final fee = demoFee(lamports);
    final senderBefore = await sender.privateBalance();
    final recipientBefore = await recipient.privateBalance();
    final collectorBefore = await collector.privateBalance();
    expect(senderBefore, greaterThanOrEqualTo(lamports + fee));

    final started = DateTime.now();
    // The demo's fee is a second private output in the same proof.
    final outputs = [
      Output.private(recipient: demoAccounts[1].publicKey, amount: lamports),
      Output.private(recipient: demoFeeCollector.publicKey, amount: fee),
    ];
    final prepared = await sender.prepareSpend(outputs: outputs);
    expect(
      prepared.operation,
      Operation.spend(outputs: outputs, createdTokenAccounts: const []),
    );
    final checks = SpendChecks(TestClusterRpc(Network.devnet.rpcUrl));
    final request = SpendRequest(outputs: outputs);
    await checks.verify(
      message: prepared.message,
      openings: prepared.outputs,
      sender: signer.publicKey,
      request: request,
    );
    // A slow approval: the same proof under a new blockhash.
    final transaction = await sender.refresh(prepared);
    expect(
      transaction.lastValidBlockHeight,
      greaterThanOrEqualTo(prepared.lastValidBlockHeight),
    );
    await checks.verify(
      message: transaction.message,
      openings: transaction.outputs,
      sender: signer.publicKey,
      request: request,
    );
    final signature = await sender.submit(transaction, [
      await signer.signMessage(transaction.message, transaction: transaction),
    ]);
    // ignore: avoid_print
    print(
      'sent $signature in ${DateTime.now().difference(started).inMilliseconds} ms '
      '(proving key download, proof, signing, confirmation)',
    );

    await checkFreshLookup(sender, signature);
    await checkFreshLookup(recipient, signature);
    await checkFreshLookup(collector, signature);

    // The transfer returned once the indexer had it, so both balances
    // already read its notes.
    expect(await sender.privateBalance(), senderBefore - lamports - fee);
    expect(await recipient.privateBalance(), recipientBefore + lamports);
    expect(await collector.privateBalance(), collectorBefore + fee);
    // ignore: avoid_print
    print(
      'the sender sent ${transport.sent.length} requests through Dart, '
      'key downloads: ${transport.sent.where((r) => r.startsWith('GET '))}',
    );
    expect(transport.sent, contains('POST /'));
    expect(transport.sent, anyElement(endsWith('/getMerkleProofs')));
  }, skip: !_enabled);

  testWidgets('sends privately from A to B, proved remotely', (tester) async {
    final lamports = BigInt.from(1000000);
    final transport = AppTransport();
    addTearDown(transport.close);
    Future<ZolanaWallet> sender({String? proverUrl}) =>
        open(demoAccounts[0], proverUrl: proverUrl, transport: transport.send);
    final local = await sender();
    addTearDown(local.close);
    await expectLater(
      local.prepareSpend(
        outputs: [
          Output.private(
            recipient: demoAccounts[1].publicKey,
            amount: lamports,
          ),
        ],
        proving: Proving.remote,
      ),
      throwsA(const WalletError.remoteProverMissing()),
    );
    await local.close();
    // By default the deployment's attested prover.
    final remote = await sender(
      proverUrl: _proverUrl.isEmpty ? _deploymentProver : _proverUrl,
    );
    addTearDown(remote.close);
    final recipient = await open(demoAccounts[1]);
    addTearDown(recipient.close);
    final senderBefore = await remote.privateBalance();
    final recipientBefore = await recipient.privateBalance();

    final started = DateTime.now();
    final signature = await checkedSpend(
      remote,
      outputs: [
        Output.private(recipient: demoAccounts[1].publicKey, amount: lamports),
      ],
      proving: Proving.remote,
    );
    // ignore: avoid_print
    print(
      'sent $signature in ${DateTime.now().difference(started).inMilliseconds} ms '
      '(remote proof, verification, signing, confirmation)',
    );
    await checkFreshLookup(remote, signature);
    expect(await remote.privateBalance(), senderBefore - lamports);
    expect(await recipient.privateBalance(), recipientBefore + lamports);
    // ignore: avoid_print
    print(
      'the sender sent ${transport.sent.length} requests through Dart, '
      'proofs: ${transport.sent.where(_isProof)}',
    );
    expect(transport.sent, contains('POST /'));
    expect(transport.sent, anyElement(endsWith('/getMerkleProofs')));
    expect(
      transport.sent.where((r) => r.startsWith('POST ') && _isProof(r)),
      hasLength(1),
    );
    // The prover attests before it gets the encrypted request; nothing else
    // is fetched, no proving key among it.
    expect(transport.sent, contains('GET /tee/v1/attestation'));
    expect(
      transport.sent.where(
        (r) =>
            r.startsWith('GET ') &&
            !_isProof(r) &&
            r != 'GET /tee/v1/attestation',
      ),
      isEmpty,
    );
  }, skip: !_enabled);

  testWidgets('shields, sends and unshields the demo token', (tester) async {
    final mint = demoToken.mint!;
    final rpc = TestClusterRpc(Network.devnet.rpcUrl);
    final sender = await open(demoAccounts[0]);
    addTearDown(sender.close);
    final recipient = await open(demoAccounts[1]);
    addTearDown(recipient.close);
    final shield = demoToken.parse('1')!;
    final send = demoToken.parse('0.5')!;
    final unshield = demoToken.parse('0.25')!;
    expect(
      await rpc.tokenBalance(demoAccounts[0].publicKey, mint),
      greaterThanOrEqualTo(shield),
      reason: 'account A holds the demo token',
    );
    final senderBefore = await sender.privateBalance(mint: mint);
    final recipientBefore = await recipient.privateBalance(mint: mint);

    await sender.deposit(shield, mint: mint);
    expect(await sender.privateBalance(mint: mint), senderBefore + shield);

    final sent = await checkedSpend(
      sender,
      outputs: [
        Output.private(recipient: demoAccounts[1].publicKey, amount: send),
      ],
      mint: mint,
    );
    await checkFreshLookup(sender, sent);
    await checkFreshLookup(recipient, sent);
    expect(
      await sender.privateBalance(mint: mint),
      senderBefore + shield - send,
    );
    expect(await recipient.privateBalance(mint: mint), recipientBefore + send);
    final received = (await recipient.activity()).first;
    expect(
      (received.signature, received.kind, received.mint, received.amount),
      (sent, ActivityKind.received, mint, send),
    );

    final publicBefore = await rpc.tokenBalance(
      demoAccounts[1].publicKey,
      mint,
    );
    final withdrawn = await checkedSpend(
      recipient,
      outputs: [
        Output.public(recipient: demoAccounts[1].publicKey, amount: unshield),
      ],
      mint: mint,
    );
    await checkFreshLookup(recipient, withdrawn);
    expect(
      await rpc.tokenBalance(demoAccounts[1].publicKey, mint),
      publicBefore + unshield,
    );
    expect(
      await recipient.privateBalance(mint: mint),
      recipientBefore + send - unshield,
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
        proverUrl: _proverUrl.isEmpty ? _deploymentProver : _proverUrl,
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
      await checkFreshLookup(wallet, signature);
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
